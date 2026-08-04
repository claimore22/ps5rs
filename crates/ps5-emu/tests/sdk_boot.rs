//! Boot an externally-supplied libSceDbg `basic` sample eboot to completion,
//! exercising the full materialize → stub → dispatcher → Registry → escape
//! pipeline against a production binary. The test self-skips when
//! `PS5_SAMPLE_EBOOT` is unset so it stays green on machines without the
//! binary, and is serialized against the other guest-run tests through
//! `GUEST_LOCK`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ps5_emu::nid::catalog;
use ps5_emu::{Emulator, Process};

static GUEST_LOCK: Mutex<()> = Mutex::new(());

/// This binary runs as its own process, so it must not share the default
/// load base with the other test binaries (their identity-mapped
/// reservations would collide).
const LOAD_BASE: u64 = 0x850000000;

fn sample_eboot() -> Option<PathBuf> {
    std::env::var("PS5_SAMPLE_EBOOT").ok().map(PathBuf::from)
}

fn prx_dir_for(eboot: &Path) -> PathBuf {
    let dir = eboot.parent().unwrap_or(Path::new("."));
    let direct = dir.join("sce_module");
    if direct.is_dir() {
        return direct;
    }
    dir.join("prx")
}

#[test]
fn sdk_libc_dbg_basic_boots_to_finalized() {
    let _guard = GUEST_LOCK.lock().unwrap();

    let Some(eboot_path) = sample_eboot() else {
        eprintln!("skipping: PS5_SAMPLE_EBOOT not set");
        return;
    };

    let prx_dir = prx_dir_for(&eboot_path);
    let eboot_bytes = std::fs::read(&eboot_path)
        .unwrap_or_else(|e| panic!("failed to read sample eboot {}: {e}", eboot_path.display()));

    let provider = |name: &str| -> Option<Vec<u8>> {
        let direct = prx_dir.join(name);
        if direct.is_file() {
            return std::fs::read(direct).ok();
        }
        let suffixed = prx_dir.join(format!("{name}.prx"));
        if suffixed.is_file() {
            return std::fs::read(suffixed).ok();
        }
        None
    };

    let process = Process::load_at("eboot.elf", eboot_bytes, provider, None, LOAD_BASE)
        .expect("failed to load sample eboot through loader pipeline");
    assert!(
        process.modules().len() >= 2,
        "expected libc.prx + eboot.elf"
    );

    let mut emulator = Emulator::new(process);
    let table = emulator
        .resolve_imports_with(&catalog())
        .expect("import table build failed");
    assert_eq!(
        table.unknown, 0,
        "all sample imports should resolve to names"
    );

    let report = emulator.run().expect("sample should run to completion");
    assert_eq!(report.exit_code, 0, "sample should exit with code 0");
    assert!(matches!(emulator.state(), ps5_emu::EmuState::Halted));

    let lines = &report.output_lines;
    assert_eq!(lines.len(), 5, "banner + three log chunks + finalized");
    assert!(lines[0].starts_with("## Sample Application: start initializing ##"));
    assert!(lines[1].contains("basic.cpp:36]"));
    assert!(lines[1].contains("Two random numbers: 40788086, 3851444534"));
    assert!(lines[2].contains("basic.cpp:39]"));
    assert!(lines[2].contains("Three random numbers: 915262580, 2714061548, 1316748153"));
    assert!(lines[2].contains("My mind is going"));
    assert!(lines[3].contains("basic.cpp:42]"));
    assert!(
        lines[3].contains("Four random numbers: 3605590735, 452227306, 2966872715, 1229098382")
    );
    assert!(lines[3].contains("Daisy, daisy, give me your answer do"));
    assert!(lines[4].starts_with("## Sample Application: finalized ##"));

    let names: Vec<&str> = report
        .import_calls
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "_init_env",
            "atexit",
            "atexit",
            "puts",
            "sceDbgSetMinimumLogLevel",
            "rand",
            "sceDbgLoggingHandler",
            "rand",
            "rand",
            "sceDbgLoggingHandler",
            "rand",
            "rand",
            "rand",
            "sceDbgLoggingHandler",
            "rand",
            "rand",
            "rand",
            "rand",
            "sceDbgLoggingHandler",
            "puts",
        ]
    );

    let mut rand_values = Vec::new();
    let mut handler_lines = Vec::new();
    let mut handler_levels = Vec::new();
    for call in &report.import_calls {
        match call.name.as_str() {
            "sceDbgSetMinimumLogLevel" => assert_eq!(call.args[0], 1, "min level must be 1"),
            "rand" => rand_values.push(call.return_value),
            "sceDbgLoggingHandler" => {
                handler_lines.push(call.args[1]);
                handler_levels.push(call.args[2]);
            }
            _ => {}
        }
    }
    assert_eq!(
        rand_values,
        [
            200_494_509,
            40_788_086,
            3_851_444_534,
            915_262_580,
            2_714_061_548,
            1_316_748_153,
            3_605_590_735,
            452_227_306,
            2_966_872_715,
            1_229_098_382,
        ]
    );
    assert_eq!(handler_lines, [33, 36, 39, 42]);
    assert_eq!(handler_levels, [0, 1, 3, 4]);
}
