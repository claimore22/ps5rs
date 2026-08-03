//! Boot a PS5 eboot under the host-side emulator and report the readiness gate.

use std::path::{Path, PathBuf};

use ps5_emu::Emulator;
use ps5_emu::nid::catalog;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let eboot_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/test/generated_elfs/hello_puts.elf"
            ))
        });

    let prx_dir = prx_dir_for(&eboot_path);
    let eboot_bytes = std::fs::read(&eboot_path)?;

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

    let mut emulator = Emulator::from_elf("eboot.elf", eboot_bytes, provider, None)?;

    let cat = catalog();
    emulator.resolve_imports_with(&cat)?;
    let table = emulator.process().imports().expect("imports resolved");

    println!("modules: {}", emulator.process().modules().len());
    for module in emulator.process().modules() {
        println!(
            "  {} @ {:#x} ({} imports, {} exports)",
            module.canonical_name(),
            module.load_bias,
            module.imports_stubbed + module.imports_resolved + module.imports_known,
            module.exports_count,
        );
    }

    println!("imports: {} known / {} unknown", table.known, table.unknown);
    for binding in &table.bindings {
        let name = binding.name.as_deref().unwrap_or("?");
        println!(
            "  {:<28} {:<14} {:<10} got={:#x} = {:#x}",
            name, binding.nid_str, binding.library, binding.got_slot, binding.current
        );
    }

    let report = emulator.run()?;
    println!(
        "{} @ {:#x} exited with code {} — {} import calls",
        report.module_name,
        report.entry_point,
        report.exit_code,
        report.import_calls.len()
    );
    Ok(())
}

fn prx_dir_for(eboot: &Path) -> PathBuf {
    let dir = eboot.parent().unwrap_or(Path::new("."));
    let direct = dir.join("sce_module");
    if direct.is_dir() {
        return direct;
    }
    dir.join("prx")
}
