use std::path::Path;

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_sdk(path: &Path, format: OutputFormat, output: &Option<std::path::PathBuf>) {
    let mut db = ps5_sdk_meta::SdkDatabase::new();
    let mut source = "builtin".to_string();
    let mut count = 0usize;

    if path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("csv"))
            .unwrap_or(false)
    {
        let before = db.len();
        db.populate_from_nids_csv(path);
        count = db.len().saturating_sub(before);
        source = format!("nids.csv:{}", path.display());
    } else if path.is_dir() {
        let before = db.len();
        db.populate_from_stubs_dir(path);
        count = db.len().saturating_sub(before);
        if count > 0 {
            source = format!("stubs:{}", path.display());
        } else {
            let csv_path = path.join("nids.csv");
            if csv_path.exists() {
                let before2 = db.len();
                db.populate_from_nids_csv(&csv_path);
                count = db.len().saturating_sub(before2);
                if count > 0 {
                    source = format!("nids.csv:{}", csv_path.display());
                }
            }
        }
    } else if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("csv"))
        .unwrap_or(false)
    {
        let before = db.len();
        db.populate_from_nids_csv(path);
        count = db.len().saturating_sub(before);
        source = format!("nids.csv:{}", path.display());
    }

    if db.is_empty() {
        match format {
            OutputFormat::Json => {
                write_to_output_or_stdout(output, &|w| {
                    let json = db.to_json_string().unwrap();
                    writeln!(w, "{json}")
                });
            }
            _ => {
                write_to_output_or_stdout(output, &|w| {
                    writeln!(
                        w,
                        "SDK database: SKIPPED — no stubs/.a or nids.csv found at {}",
                        path.display()
                    )?;
                    writeln!(w, "Source: {}", source)?;
                    writeln!(w, "Functions: 0")?;
                    writeln!(w, "Hint: provide path to SDK stubs dir or nids.csv")?;
                    Ok(())
                });
            }
        }
        return;
    }

    match format {
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                let json = db.to_json_string().unwrap();
                writeln!(w, "{json}")
            });
        }
        _ => {
            write_to_output_or_stdout(output, &|w| {
                writeln!(w, "SDK database from {}: {} functions", source, db.len())?;
                let mut libs: std::collections::HashSet<String> = std::collections::HashSet::new();
                for f in db.functions.values() {
                    libs.insert(f.library.clone());
                }
                let mut libs_vec: Vec<String> = libs.into_iter().collect();
                libs_vec.sort();
                writeln!(w, "Libraries ({}):", libs_vec.len())?;
                for lib in libs_vec.iter().take(20) {
                    let cnt = db.query_by_library(lib).len();
                    writeln!(w, "  {} ({} funcs)", lib, cnt)?;
                }
                if libs_vec.len() > 20 {
                    writeln!(w, "  ... and {} more", libs_vec.len() - 20)?;
                }
                writeln!(w, "\nFunctions (first 20):")?;
                for f in db.functions.values().take(20) {
                    writeln!(w, "  {}::{} [{}]", f.library, f.name, f.category)?;
                }
                if db.len() > 20 {
                    writeln!(w, "  ... and {} more", db.len() - 20)?;
                }
                let _ = count;
                Ok(())
            });
        }
    }
}
