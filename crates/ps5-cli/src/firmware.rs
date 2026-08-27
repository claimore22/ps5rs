use std::path::Path;

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_firmware(path: &Path, format: OutputFormat, output: &Option<std::path::PathBuf>) {
    let mut catalog =
        ps5_firmware::FirmwareCatalog::new(ps5_firmware::FirmwareVersion::new(0, 0, 0));
    let mut loaded = 0usize;

    if path.is_dir() {
        // Try as system_modules dir
        let direct = catalog.load_exports_from_dir(path);
        if direct > 0 {
            loaded = direct;
        } else {
            // Try system_modules subdirectory
            let sys = path.join("system_modules");
            if sys.is_dir() {
                loaded = catalog.load_exports_from_dir(&sys);
            }
        }
        // If path is game dir, also try to check compatibility via lib versions
        if loaded == 0 && path.join("eboot.bin").exists() {
            // No firmware table, report as unknown
        }
    }

    match format {
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                let json = serde_json::to_string_pretty(&catalog).unwrap();
                writeln!(w, "{json}")
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
                // Example compatibility check if game dir provided
                if path.join("eboot.bin").exists() {
                    writeln!(
                        w,
                        "\nNote: firmware provides {} libraries; check with is_library_available for compatibility (compatible/insufficient/unknown)",
                        catalog.libraries.len()
                    )?;
                }
                Ok(())
            });
        }
    }
}
