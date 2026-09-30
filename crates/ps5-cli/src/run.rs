use std::path::{Path, PathBuf};

use ps5_loader::OfflineExportTable;

use crate::load::get_elf_bytes;
use crate::util::load_file;

fn prx_dirs_for(file: &Path, prx_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = prx_dir.into_iter().collect();
    dirs.extend(crate::util::prx_candidate_dirs(file));
    dirs
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
    let container = if data.len() >= 4 && &data[0..4] == b"\x7fELF" {
        "Raw ELF"
    } else if data.len() >= 4 {
        match u32::from_be_bytes([data[0], data[1], data[2], data[3]]) {
            0x5414F5EE => "PS5 SELF",
            0x4F153D1D => "PS4 SELF",
            magic => {
                tracing::warn!(
                    magic = format_args!("{magic:#x}"),
                    "unknown container magic"
                );
                "Unknown"
            }
        }
    } else {
        "Too small"
    };
    tracing::info!(file = %file.display(), container, size = data.len(), "run: input detected");
    let elf_bytes = get_elf_bytes(&data);
    tracing::info!(size = elf_bytes.len(), "run: ELF extraction ok");

    let dirs = prx_dirs_for(file, prx_dir);
    tracing::info!(dirs = ?dirs, "run: PRX provider dirs");
    let provider = |name: &str| -> Option<Vec<u8>> {
        for dir in &dirs {
            for candidate in [dir.join(name), dir.join(format!("{name}.prx"))] {
                if candidate.is_file() {
                    match std::fs::read(&candidate) {
                        Ok(bytes) => {
                            let kind = if bytes.len() >= 4 && &bytes[0..4] == b"\x7fELF" {
                                "Raw ELF"
                            } else if bytes.len() >= 4 {
                                match u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) {
                                    0x5414F5EE => "PS5 SELF",
                                    0x4F153D1D => "PS4 SELF",
                                    _ => "Unknown",
                                }
                            } else {
                                "Too small"
                            };
                            tracing::info!(module = %name, path = %candidate.display(), kind, size = bytes.len(), "run: PRX provider hit");
                            return Some(bytes);
                        }
                        Err(e) => {
                            tracing::warn!(module = %name, path = %candidate.display(), error = %e, "run: PRX read failed");
                        }
                    }
                }
            }
        }
        tracing::warn!(module = %name, "run: PRX not found in provider dirs");
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
            "  import {}::{} nid={:#x} args={:?} -> {}",
            call.library, call.name, call.nid, call.args, call.return_value
        );
    }
    if report.import_calls.is_empty() {
        println!("  (no import calls)");
    }
}
