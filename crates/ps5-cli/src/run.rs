use std::path::{Path, PathBuf};

use ps5_loader::OfflineExportTable;

use crate::load::get_elf_bytes;
use crate::util::load_file;

fn prx_dir_for(file: &Path) -> Option<PathBuf> {
    let parent = file.parent().unwrap_or(Path::new("."));
    let direct = parent.join("sce_module");
    if direct.is_dir() {
        return Some(direct);
    }
    let prx = parent.join("prx");
    if prx.is_dir() {
        return Some(prx);
    }
    None
}

fn offline_table() -> Option<OfflineExportTable> {
    let dir = Path::new("system_modules");
    if dir.is_dir() {
        let table = OfflineExportTable::load_from_dir(dir);
        if !table.is_empty() {
            return Some(table);
        }
    }
    None
}

pub(crate) fn cmd_run(file: &PathBuf, prx_dir: Option<PathBuf>, json: bool) {
    let data = load_file(file);
    let elf_bytes = get_elf_bytes(&data);

    let dir = prx_dir.or_else(|| prx_dir_for(file));
    let provider = |name: &str| -> Option<Vec<u8>> {
        let dir = dir.as_ref()?;
        let direct = dir.join(name);
        if direct.is_file() {
            return std::fs::read(direct).ok();
        }
        let suffixed = dir.join(format!("{name}.prx"));
        if suffixed.is_file() {
            return std::fs::read(suffixed).ok();
        }
        None
    };

    let name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "eboot.elf".to_string());

    let mut emulator =
        ps5_emu::Emulator::from_elf(&name, elf_bytes, provider, offline_table().as_ref())
            .unwrap_or_else(|e| {
                eprintln!("error: load failed: {e}");
                std::process::exit(1);
            });

    emulator
        .resolve_imports_with(&ps5_emu::nid::catalog())
        .unwrap_or_else(|e| {
            eprintln!("error: import resolution failed: {e}");
            std::process::exit(1);
        });

    let report = emulator.run().unwrap_or_else(|e| {
        eprintln!("error: execution failed: {e}");
        std::process::exit(1);
    });

    if json {
        let json_str = serde_json::to_string_pretty(&report).unwrap_or_else(|e| {
            eprintln!("error: JSON serialization failed: {e}");
            std::process::exit(1);
        });
        println!("{json_str}");
        return;
    }

    for line in &report.output_lines {
        print!("{line}");
    }

    println!(
        "{} @ {:#x} exited with code {}",
        report.module_name, report.entry_point, report.exit_code
    );
    for call in &report.import_calls {
        println!(
            "  import {}::{} args={:?} -> {}",
            call.library, call.name, call.args, call.return_value
        );
    }
    if report.import_calls.is_empty() {
        println!("  (no import calls)");
    }
}
