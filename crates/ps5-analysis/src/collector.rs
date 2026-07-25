use crate::model::*;
use ps5_nid::{Catalog, lib_id_from_nid};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct CollectorOptions {
    pub include_prx: bool,
}

pub fn collect(root: &Path, catalog: &Catalog, options: &CollectorOptions) -> AnalysisDatabase {
    let mut games = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let binaries = find_binaries(&path, options);
            for bin_path in &binaries {
                if let Some(analysis) = analyze_binary(bin_path, catalog, &path) {
                    games.push(analysis);
                }
            }
        }
    }

    AnalysisDatabase {
        schema_version: 1,
        tool: "ps5rs".to_string(),
        games,
    }
}

fn find_binaries(game_dir: &Path, options: &CollectorOptions) -> Vec<PathBuf> {
    let mut result = Vec::new();

    fn walk(dir: &Path, result: &mut Vec<PathBuf>, depth: usize) {
        if depth > 4 {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, result, depth + 1);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_ascii_lowercase();
                    if lower == "eboot.bin" || lower.ends_with(".prx") || lower.ends_with(".so") {
                        result.push(path);
                    }
                }
            }
        }
    }

    walk(game_dir, &mut result, 0);

    if options.include_prx {
        result
    } else {
        result.into_iter().filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case("eboot.bin"))
                .unwrap_or(false)
        }).collect()
    }
}

fn analyze_binary(path: &Path, catalog: &Catalog, game_dir: &Path) -> Option<GameAnalysis> {
    let data = std::fs::read(path).ok()?;
    let sha256 = compute_sha256(&data);
    let file_size = data.len() as u64;

    let img = ps5_self::SelfImage::parse(&data).ok()?;

    let platform = match img.platform {
        ps5_self::SelfPlatform::Ps4 => Platform::Ps4,
        ps5_self::SelfPlatform::Ps5 => Platform::Ps5,
        ps5_self::SelfPlatform::RawElf => Platform::RawElf,
        ps5_self::SelfPlatform::Unknown(_) => Platform::Unknown,
    };

    let imports: Vec<ImportInfo> = img.elf.symbols.iter()
        .filter(|s| s.is_import)
        .map(|sym| {
            let parts: Vec<&str> = sym.resolved_name.split('#').collect();
            let nid = parts[0];
            let lib_id = lib_id_from_nid(&sym.resolved_name).unwrap_or(0);
            let lib_name = img.elf.import_libs.get(&lib_id).cloned()
                .unwrap_or_else(|| format!("lib_{}", parts.get(1).unwrap_or(&"?")));
            let resolved = catalog.resolve(nid).unwrap_or("?").to_string();

            ImportInfo {
                nid_hash: nid.to_string(),
                resolved_name: resolved,
                library_id: lib_id,
                library_name: lib_name,
            }
        })
        .collect();

    let import_libs: Vec<LibInfo> = img.elf.import_libs.iter()
        .map(|(id, name)| LibInfo { id: *id, name: name.clone() })
        .collect();

    let game_name = game_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Some(GameAnalysis {
        name: game_name,
        path: path.display().to_string(),
        sha256,
        file_size,
        platform,
        entry_point: img.elf.header.e_entry,
        is_self: img.is_self(),
        imports,
        import_libs,
        needed_files: img.elf.needed_files,
        num_relocations: img.elf.relocations.len(),
        num_symbols: img.elf.symbols.len(),
        has_tls: img.elf.tls.is_some(),
    })
}

fn compute_sha256(data: &[u8]) -> String {
    ps5_format::sha256_hex(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_sha256_matches_known_vectors() {
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn find_binaries_eboot_only() {
        let tmp = tempdir_for_test().join("eboot_only");
        let game_dir = tmp.join("MyGame");
        let sce_dir = game_dir.join("sce_module");
        std::fs::create_dir_all(&sce_dir).unwrap();
        std::fs::write(game_dir.join("eboot.bin"), b"fake").unwrap();
        std::fs::write(sce_dir.join("libc.prx"), b"fake").unwrap();
        std::fs::write(sce_dir.join("libScePfs.prx"), b"fake").unwrap();

        let opts = CollectorOptions { include_prx: false };
        let bins = find_binaries(&game_dir, &opts);
        assert_eq!(bins.len(), 1);
        assert!(bins[0].file_name().unwrap().to_str().unwrap().eq_ignore_ascii_case("eboot.bin"));
    }

    #[test]
    fn find_binaries_include_prx() {
        let tmp = tempdir_for_test().join("include_prx");
        let game_dir = tmp.join("MyGame");
        let sce_dir = game_dir.join("sce_module");
        std::fs::create_dir_all(&sce_dir).unwrap();
        std::fs::write(game_dir.join("eboot.bin"), b"fake").unwrap();
        std::fs::write(sce_dir.join("libc.prx"), b"fake").unwrap();

        let opts = CollectorOptions { include_prx: true };
        let bins = find_binaries(&game_dir, &opts);
        assert_eq!(bins.len(), 2);
    }

    #[test]
    fn find_binaries_empty_dir() {
        let tmp = tempdir_for_test().join("empty_dir");
        let game_dir = tmp.join("EmptyGame");
        std::fs::create_dir_all(&game_dir).unwrap();

        let opts = CollectorOptions { include_prx: false };
        let bins = find_binaries(&game_dir, &opts);
        assert!(bins.is_empty());
    }

    #[test]
    fn find_binaries_skips_non_game_files() {
        let tmp = tempdir_for_test().join("skips_non_game");
        let game_dir = tmp.join("Game");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::write(game_dir.join("README.txt"), b"text").unwrap();
        std::fs::write(game_dir.join("icon.png"), b"png").unwrap();
        std::fs::write(game_dir.join("eboot.bin"), b"fake").unwrap();

        let opts = CollectorOptions { include_prx: false };
        let bins = find_binaries(&game_dir, &opts);
        assert_eq!(bins.len(), 1);
    }

    fn tempdir_for_test() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ps5rs_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}
