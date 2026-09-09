use std::path::Path;

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_firmware(path: &Path, format: OutputFormat, output: &Option<std::path::PathBuf>) {
    let mut catalog =
        ps5_firmware::FirmwareCatalog::new(ps5_firmware::FirmwareVersion::new(0, 0, 0));
    let mut loaded = 0usize;

    if path.is_dir() {
        let direct = catalog.load_exports_from_dir(path);
        if direct > 0 {
            loaded = direct;
        } else {
            let sys = path.join("system_modules");
            if sys.is_dir() {
                loaded = catalog.load_exports_from_dir(&sys);
            } else {
                let default_sys = std::path::Path::new("system_modules");
                if default_sys.is_dir() {
                    loaded = catalog.load_exports_from_dir(default_sys);
                }
            }
        }
    }

    let game_dirs: Vec<std::path::PathBuf> = if path.is_dir() {
        let mut dirs = Vec::new();
        if path.join("eboot.bin").exists() {
            dirs.push(path.to_path_buf());
        } else if let Ok(entries) = std::fs::read_dir(path) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    let mut stack = vec![p];
                    while let Some(dir) = stack.pop() {
                        if dir.join("eboot.bin").exists() {
                            dirs.push(dir);
                            break;
                        }
                        if let Ok(sub) = std::fs::read_dir(&dir) {
                            let subs: Vec<std::path::PathBuf> = sub
                                .flatten()
                                .map(|e| e.path())
                                .filter(|p| p.is_dir())
                                .collect();
                            if subs.len() == 1 {
                                stack.push(subs[0].clone());
                            }
                        }
                    }
                }
            }
        }
        dirs
    } else {
        Vec::new()
    };
    let mut compat_results: Vec<(String, Vec<(String, ps5_firmware::LibraryAvailability)>)> =
        Vec::new();
    for game_dir in &game_dirs {
        let eboot = game_dir.join("eboot.bin");
        let data = match std::fs::read(&eboot) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let lib_versions: Vec<(String, String)> =
            if let Ok(img) = ps5_elf::ElfImage::parse(&data, None) {
                img.lib_versions
                    .iter()
                    .map(|lv| (lv.name.clone(), lv.guessed_version_string()))
                    .collect()
            } else if let Ok(simg) = ps5_self::SelfImage::parse(&data) {
                simg.elf
                    .lib_versions
                    .iter()
                    .map(|lv| (lv.name.clone(), lv.guessed_version_string()))
                    .collect()
            } else {
                Vec::new()
            };
        if !lib_versions.is_empty() {
            let checks = catalog.check_requirements(&lib_versions);
            let game_name = game_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("game")
                .to_string();
            compat_results.push((game_name, checks));
        }
    }

    match format {
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                if !compat_results.is_empty() {
                    let json = serde_json::json!({
                        "catalog": catalog,
                        "compatibility": compat_results.iter().map(|(game, checks)| {
                            serde_json::json!({
                                "game": game,
                                "checks": checks.iter().map(|(lib, avail)| {
                                    serde_json::json!({"library": lib, "availability": format!("{:?}", avail)})
                                }).collect::<Vec<_>>()
                            })
                        }).collect::<Vec<_>>()
                    });
                    serde_json::to_writer_pretty(w, &json).map_err(std::io::Error::other)
                } else {
                    serde_json::to_writer_pretty(w, &catalog).map_err(std::io::Error::other)
                }
            });
        }
        _ => {
            write_to_output_or_stdout(output, &|w| {
                if loaded == 0 {
                    writeln!(
                        w,
                        "Firmware catalog: SKIPPED — no system_modules/*.exports.json found at {}",
                        path.display()
                    )?;
                    writeln!(w, "Modules: 0, Exports: 0")?;
                    writeln!(
                        w,
                        "Hint: run with path to system_modules or game dir containing system_modules"
                    )?;
                    return Ok(());
                }
                writeln!(
                    w,
                    "Firmware catalog from {}: {} modules, {} exports",
                    path.display(),
                    catalog.modules.len(),
                    loaded
                )?;
                writeln!(w, "Modules:")?;
                for m in catalog.modules.iter().take(20) {
                    writeln!(w, "  {} ({} exports) @ {}", m.name, m.exports_count, m.path)?;
                }
                if catalog.modules.len() > 20 {
                    writeln!(w, "  ... and {} more", catalog.modules.len() - 20)?;
                }
                writeln!(w, "\nLibraries:")?;
                for lib in catalog.libraries.iter().take(20) {
                    writeln!(
                        w,
                        "  {} v{} ({} modules)",
                        lib.name,
                        lib.version,
                        lib.modules.len()
                    )?;
                }
                if catalog.libraries.len() > 20 {
                    writeln!(w, "  ... and {} more", catalog.libraries.len() - 20)?;
                }
                if !compat_results.is_empty() {
                    writeln!(
                        w,
                        "\nPer-game compatibility (game requires vs firmware provides):"
                    )?;
                    for (game, checks) in &compat_results {
                        writeln!(w, "  {}:", game)?;
                        for (lib, avail) in checks {
                            let status = match avail {
                                ps5_firmware::LibraryAvailability::Compatible => "compatible",
                                ps5_firmware::LibraryAvailability::Insufficient { .. } => {
                                    "insufficient"
                                }
                                ps5_firmware::LibraryAvailability::NotFound => "not found",
                                ps5_firmware::LibraryAvailability::Unknown { .. } => "unknown",
                            };
                            writeln!(w, "    {} -> {}", lib, status)?;
                        }
                    }
                } else if path.join("eboot.bin").exists() {
                    writeln!(
                        w,
                        "\nNote: firmware provides {} libraries; no lib_version requirements found in eboot (check with is_library_available for compatibility)",
                        catalog.libraries.len()
                    )?;
                }
                Ok(())
            });
        }
    }
}
