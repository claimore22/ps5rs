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

    let code = emulator
        .run()
        .expect("sample should run to completion")
        .exit_code;
    assert_eq!(code, 0, "sample should exit with code 0");
    assert!(matches!(emulator.state(), ps5_emu::EmuState::Halted));
}
