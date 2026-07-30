use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const BACKUP_EXTS: &[&str] = &[".esbak", ".bak", ".orig"];

fn is_backup(filename: &str) -> bool {
    BACKUP_EXTS.iter().any(|ext| filename.ends_with(ext))
}

#[derive(Serialize, Deserialize)]
struct ExportsFile {
    module: String,
    exports: Vec<ExportRow>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ExportRow {
    nid: String,
    name: String,
    address: String,
    size: u64,
}

fn get_elf_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() >= 4 && &data[0..4] == b"\x7fELF" {
        return Ok(data.to_vec());
    }
    let result =
        ps5_self::extract::extract_elf(data).map_err(|e| format!("SELF extraction failed: {e}"))?;
    Ok(result.elf)
}

fn strip_backup_suffix(filename: &str) -> &str {
    for ext in BACKUP_EXTS {
        if let Some(base) = filename.strip_suffix(ext) {
            return base;
        }
    }
    filename
}

fn derive_module_name(filename: &str) -> String {
    let clean = strip_backup_suffix(filename);
    if clean.ends_with(".prx") {
        return clean.to_string();
    }
    if let Some(base) = clean.strip_suffix(".prx.elf") {
        return format!("{base}.prx");
    }
    if let Some(base) = clean.strip_suffix(".elf") {
        return format!("{base}.prx");
    }
    if let Some(base) = clean.strip_suffix(".self") {
        return format!("{base}.prx");
    }
    if clean.ends_with(".bin") {
        return clean.to_string();
    }
    format!("{clean}.prx")
}

fn build_export_rows(elf: &ps5_elf::ElfImage) -> Vec<ExportRow> {
    elf.symbols
        .iter()
        .filter(|s| !s.is_import && s.st_value != 0)
        .map(|sym| {
            let (nid, name) = if sym.resolved_name.contains('#') {
                let nid_part = sym.resolved_name.split('#').next().unwrap_or("");
                (nid_part.to_string(), nid_part.to_string())
            } else {
                let nid_str = ps5_nid::hash(&sym.resolved_name);
                (nid_str, sym.resolved_name.clone())
            };
            ExportRow {
                nid,
                name,
                address: format!("{:#018x}", sym.st_value),
                size: sym.st_size,
            }
        })
        .collect()
}

fn load_existing_nids(path: &Path) -> HashSet<String> {
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return HashSet::new(),
    };
    match serde_json::from_str::<ExportsFile>(&data) {
        Ok(file) => file.exports.into_iter().map(|e| e.nid).collect(),
        Err(_) => HashSet::new(),
    }
}

fn output_path_for(module_name: &str, output_dir: &Path) -> PathBuf {
    let stem = module_name.strip_suffix(".prx").unwrap_or(module_name);
    output_dir.join(format!("{stem}.exports.json"))
}

pub(crate) fn cmd_export_scan(scan_dir: &PathBuf, output_dir: &PathBuf) {
    if !scan_dir.is_dir() {
        eprintln!("error: {} is not a directory", scan_dir.display());
        std::process::exit(1);
    }

    fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        eprintln!(
            "error: cannot create output directory {}: {e}",
            output_dir.display()
        );
        std::process::exit(1);
    });

    let entries = match fs::read_dir(scan_dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot read directory {}: {e}", scan_dir.display());
            std::process::exit(1);
        }
    };

    let mut scanned = 0usize;
    let mut written = 0usize;
    let mut merged = 0usize;
    let mut skipped = 0usize;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let filepath = entry.path();
        if !filepath.is_file() {
            continue;
        }
        let filename = filepath
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if filename.is_empty() || is_backup(&filename) {
            continue;
        }

        let data = match fs::read(&filepath) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}", filepath.display());
                skipped += 1;
                continue;
            }
        };

        let elf_bytes = match get_elf_bytes(&data) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: {}: {e}", filepath.display());
                skipped += 1;
                continue;
            }
        };

        let elf = match ps5_elf::ElfImage::parse(&elf_bytes, None) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("warning: cannot parse ELF from {}: {e}", filepath.display());
                skipped += 1;
                continue;
            }
        };

        let module_name = if let Some(ref soname) = elf.soname {
            soname.clone()
        } else {
            derive_module_name(&filename)
        };

        let export_rows = build_export_rows(&elf);
        if export_rows.is_empty() {
            skipped += 1;
            continue;
        }

        scanned += 1;

        let output_path = output_path_for(&module_name, output_dir);

        if output_path.exists() {
            let existing_nids = load_existing_nids(&output_path);
            let new_rows: Vec<ExportRow> = export_rows
                .into_iter()
                .filter(|r| !existing_nids.contains(&r.nid))
                .collect();

            if new_rows.is_empty() {
                eprintln!("  up to date: {filename}");
                continue;
            }

            let existing_content = fs::read_to_string(&output_path).unwrap();
            let mut existing_file: ExportsFile = serde_json::from_str(&existing_content).unwrap();
            let old_count = existing_file.exports.len();
            existing_file.exports.extend(new_rows);
            existing_file.exports.sort_by(|a, b| a.nid.cmp(&b.nid));

            let json = serde_json::to_string_pretty(&existing_file).unwrap();
            fs::write(&output_path, json).unwrap();

            let added = existing_file.exports.len() - old_count;
            eprintln!(
                "  merged {} new exports from {} → {}",
                added,
                filename,
                output_path.display()
            );
            merged += 1;
        } else {
            let file = ExportsFile {
                module: module_name,
                exports: export_rows,
            };
            let json = serde_json::to_string_pretty(&file).unwrap();
            fs::write(&output_path, json).unwrap();
            eprintln!(
                "  wrote {} exports from {} → {}",
                file.exports.len(),
                filename,
                output_path.display()
            );
            written += 1;
        }
    }

    println!();
    println!("Export scan summary");
    println!("-------------------");
    println!("  Files scanned: {scanned}");
    println!("  New files written: {written}");
    println!("  Files merged: {merged}");
    println!("  Skipped: {skipped}");
}
