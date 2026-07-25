#![cfg(feature = "integration")]

use std::path::Path;

struct GameSnapshot {
    name: &'static str,
    path_pattern: &'static str,
    expected_libs: &'static [&'static str],
}

fn find_game_file(pattern: &str) -> Option<Vec<u8>> {
    let base = std::env::var("PS5_GAME_DIR")
        .unwrap_or_else(|_| r"C:\Users\claimoar\Documents\ROMS\PS5".to_string());
    let dir = std::path::PathBuf::from(&base);
    if !dir.exists() {
        return None;
    }
    for entry in std::fs::read_dir(&dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.contains(pattern) {
            continue;
        }
        let eboot = find_eboot(&entry.path());
        if let Some(path) = eboot {
            return std::fs::read(path).ok();
        }
    }
    None
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

fn parse_and_snapshot(data: &[u8]) -> (u64, usize, bool, usize, usize, Vec<String>) {
    let img = ps5_self::SelfImage::parse(data).expect("parse failed");
    let elf = &img.elf;
    let imports: Vec<_> = elf.symbols.iter().filter(|s| s.is_import).collect();

    let catalog = ps5_nid::Catalog::new();
    let mut lib_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for sym in &imports {
        let parts: Vec<&str> = sym.resolved_name.split('#').collect();
        let nid = parts[0];
        let resolved = catalog.resolve(nid).unwrap_or("?");
        let lib_name = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            "?".to_string()
        };
        *lib_counts.entry(format!("{lib_name}: {resolved}")).or_insert(0) += 1;
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

const GAMES: &[GameSnapshot] = &[
    GameSnapshot {
        name: "Stray",
        path_pattern: "Stray-PPSA02100",
        expected_libs: &["libc", "libkernel"],
    },
    GameSnapshot {
        name: "Bugsnax",
        path_pattern: "Bugsnax-PPSA01502",
        expected_libs: &["libc"],
    },
    GameSnapshot {
        name: "GRIS",
        path_pattern: "GRIS-PPSA09804",
        expected_libs: &["libc"],
    },
];

fn snapshot_test(game: &GameSnapshot) {
    let data = match find_game_file(game.path_pattern) {
        Some(d) => d,
        None => {
            eprintln!("SKIP: {} - game directory not found (set PS5_GAME_DIR)", game.name);
            return;
        }
    };
    let (entry, num_segments, has_tls, num_imports, num_relocs, libs) = parse_and_snapshot(&data);

    assert!(entry > 0, "{}: entry point should be non-zero", game.name);
    assert!(num_segments > 0, "{}: should have segments", game.name);
    assert!(num_imports > 0, "{}: should have imports", game.name);

    for expected_lib in game.expected_libs {
        assert!(libs.iter().any(|l| l.contains(expected_lib)),
            "{}: expected library containing '{}' in {:?}",
            game.name, expected_lib, libs);
    }

    println!("{}: entry={:#x} segments={} tls={} imports={} relocs={} libs={:?}",
        game.name, entry, num_segments, has_tls, num_imports, num_relocs, libs);
}

#[test]
fn stray_snapshot() {
    snapshot_test(&GAMES[0]);
}

#[test]
fn bugsnax_snapshot() {
    snapshot_test(&GAMES[1]);
}

#[test]
fn gris_snapshot() {
    snapshot_test(&GAMES[2]);
}
