#![cfg(feature = "integration")]

use std::path::Path;

fn find_game_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var("PS5_GAME_DIR").ok()?;
    let dir = std::path::PathBuf::from(&base);
    if dir.exists() { Some(dir) } else { None }
}

fn find_eboot(dir: &Path) -> Option<std::path::PathBuf> {
    let direct = dir.join("eboot.bin");
    if direct.exists() {
        return Some(direct);
    }
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(p) = find_eboot(&entry.path()) {
                return Some(p);
            }
        }
        if entry.file_name().to_string_lossy().to_lowercase() == "eboot.bin" {
            return Some(entry.path());
        }
    }
    None
}

fn parse_and_analyze(data: &[u8]) -> (u64, usize, bool, usize, usize, Vec<String>) {
    let img = ps5_self::SelfImage::parse(data).expect("parse failed");
    let elf = &img.elf;
    let imports: Vec<_> = elf.symbols.iter().filter(|s| s.is_import).collect();

    let catalog = ps5_nid::Catalog::new();
    let mut lib_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for sym in &imports {
        let parts: Vec<&str> = sym.resolved_name.split('#').collect();
        let nid = parts[0];
        let resolved = catalog
            .resolve(nid)
            .and_then(|e| e.primary_name())
            .unwrap_or("?");
        let lib_name = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "?".to_string()
        };
        *lib_counts
            .entry(format!("{lib_name}: {resolved}"))
            .or_insert(0) += 1;
    }

    let mut libs: Vec<String> = elf.import_libs.values().cloned().collect();
    libs.sort();

    (
        elf.header.e_entry,
        img.segments.len(),
        elf.tls.is_some(),
        imports.len(),
        elf.relocations.len(),
        libs,
    )
}

#[test]
fn parse_first_game_in_directory() {
    let base = match find_game_dir() {
        Some(d) => d,
        None => {
            eprintln!("SKIP: set PS5_GAME_DIR to a directory containing game dumps");
            return;
        }
    };

    let mut found = false;
    for entry in std::fs::read_dir(&base).ok().into_iter().flatten() {
        let entry = entry.ok().unwrap();
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Some(eboot) = find_eboot(&entry.path()) else {
            continue;
        };
        let data = std::fs::read(&eboot).expect("read eboot");
        let (entry_point, num_segments, has_tls, num_imports, num_relocs, libs) =
            parse_and_analyze(&data);

        assert!(entry_point > 0, "entry point should be non-zero");
        assert!(num_segments > 0, "should have segments");
        assert!(num_imports > 0, "should have imports");

        println!(
            "{}: entry={:#x} segments={} tls={} imports={} relocs={} libs={:?}",
            eboot.display(),
            entry_point,
            num_segments,
            has_tls,
            num_imports,
            num_relocs,
            libs
        );

        found = true;
        break;
    }

    if !found {
        eprintln!(
            "SKIP: no game directories with eboot.bin found under {}",
            base.display()
        );
    }
}
