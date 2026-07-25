use crate::model::*;
use ps5_nid::Catalog;
use ps5_image::{BinaryImageBuilder, Platform as ImagePlatform};
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
    let sha256 = ps5_format::sha256_hex(&data);
    let file_size = data.len() as u64;

    let image = BinaryImageBuilder::build_from_file(data, &sha256, catalog);

    let platform = match image.platform {
        ImagePlatform::Ps4 => Platform::Ps4,
        ImagePlatform::Ps5 => Platform::Ps5,
        ImagePlatform::RawElf => Platform::RawElf,
        ImagePlatform::Unknown => Platform::Unknown,
    };

    let imports: Vec<ImportInfo> = image.imports.iter()
        .map(|imp| ImportInfo {
            nid_hash: imp.nid_hash.clone(),
            resolved_name: imp.resolved_name.clone().unwrap_or_else(|| "?".into()),
            library_id: imp.library_id,
            library_name: imp.library_name.clone(),
        })
        .collect();

    let import_libs: Vec<LibInfo> = image.import_libs.iter()
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
        entry_point: image.entry_point,
        is_self: image.is_self,
        imports,
        import_libs,
        needed_files: image.needed_files,
        num_relocations: image.relocations.len(),
        num_symbols: image.imports.len() + image.exports.len(),
        has_tls: image.tls.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
