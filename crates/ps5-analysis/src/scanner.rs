use crate::dataset::{Manifest, DATASET_SCHEMA_VERSION};
use crate::param_json::{self, GameParam};
use ps5_image::{BinaryImageBuilder, BinaryImageDocument};
use ps5_nid::Catalog;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ScanOptions {
    pub include_prx: bool,
}

pub struct ScanResult {
    pub manifest: Manifest,
    pub image_paths: Vec<PathBuf>,
}

pub fn scan(
    root: &Path,
    output: &Path,
    catalog: &Catalog,
    options: &ScanOptions,
) -> Result<ScanResult, std::io::Error> {
    std::fs::create_dir_all(output)?;
    let images_dir = output.join("images");
    if images_dir.exists() {
        for entry in std::fs::read_dir(&images_dir).into_iter().flatten().flatten() {
            if entry.path().extension().map_or(false, |e| e == "json") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    std::fs::create_dir_all(&images_dir)?;

    let mut image_paths = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut game_params: Vec<GameParam> = Vec::new();

    let game_dirs = find_game_dirs(root);
    for game_dir in &game_dirs {
        let binaries = find_binaries(game_dir, options);
        for bin_path in &binaries {
            if let Some(doc) = analyze_binary(bin_path, catalog, game_dir) {
                let game_name = game_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let safe_name = sanitize_filename(game_name);
                if seen_names.contains(&safe_name) {
                    continue;
                }
                seen_names.insert(safe_name.clone());
                let json_path = output.join("images").join(format!("{safe_name}.json"));
                let json = serde_json::to_string_pretty(&doc)?;
                std::fs::write(&json_path, format!("{json}\n"))?;
                image_paths.push(json_path);

                let param = param_json::read_param(game_dir)
                    .unwrap_or_default();
                let mut param = param;
                if param.name.is_none() {
                    param.name = Some(game_name.to_string());
                }
                game_params.push(param);
            }
        }
    }

    let manifest = Manifest {
        schema_version: DATASET_SCHEMA_VERSION,
        tool: "ps5rs".to_string(),
        created_at: utc_now_iso8601(),
        image_count: image_paths.len(),
        games: game_params,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(output.join("manifest.json"), format!("{manifest_json}\n"))?;

    Ok(ScanResult {
        manifest,
        image_paths,
    })
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resolve_game_dir(&path, &mut dirs);
            }
        }
    }
    dirs.sort();
    dirs
}

fn resolve_game_dir(dir: &Path, result: &mut Vec<PathBuf>) {
    if has_eboot(dir) {
        result.push(dir.to_path_buf());
        return;
    }
    let children: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => return,
    };
    if children.len() == 1 {
        resolve_game_dir(&children[0], result);
    }
}

fn has_eboot(dir: &Path) -> bool {
    dir.join("eboot.bin").exists()
}

fn find_binaries(game_dir: &Path, options: &ScanOptions) -> Vec<PathBuf> {
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
                    if lower == "eboot.bin"
                        || lower.ends_with(".prx")
                        || lower.ends_with(".so")
                    {
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
        result
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.eq_ignore_ascii_case("eboot.bin"))
                    .unwrap_or(false)
            })
            .collect()
    }
}

fn analyze_binary(
    path: &Path,
    catalog: &Catalog,
    game_dir: &Path,
) -> Option<BinaryImageDocument> {
    let data = std::fs::read(path).ok()?;
    let sha256 = ps5_format::sha256_hex(&data);
    let image = BinaryImageBuilder::build_from_file(data, &sha256, catalog);

    let _game_name = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    Some(BinaryImageDocument {
        schema_version: DATASET_SCHEMA_VERSION,
        tool: "ps5rs".to_string(),
        image,
    })
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn utc_now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Civil date from days since 1970-01-01 (algorithm from Howard Hinnant)
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_basic() {
        assert_eq!(sanitize_filename("My Game"), "My_Game");
        assert_eq!(sanitize_filename("Game-A"), "Game-A");
        assert_eq!(sanitize_filename("a_b"), "a_b");
    }

    #[test]
    fn sanitize_filename_path_separators() {
        assert_eq!(sanitize_filename("a/b"), "a_b");
        assert_eq!(sanitize_filename("../escape"), ".._escape");
    }

    #[test]
    fn utc_now_iso8601_format() {
        let s = utc_now_iso8601();
        assert!(s.ends_with('Z'));
        assert_eq!(s.len(), 20);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert_eq!(&s[10..11], "T");
        assert_eq!(&s[13..14], ":");
        assert_eq!(&s[16..17], ":");
    }

    #[test]
    fn find_game_dirs_empty() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_scan_test_empty_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&tmp);
        let dirs = find_game_dirs(&tmp);
        assert!(dirs.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_game_dirs_finds_eboot() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_scan_test_dirs_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("GameA")).unwrap();
        std::fs::create_dir_all(tmp.join("GameB")).unwrap();
        std::fs::write(tmp.join("GameA").join("eboot.bin"), "x").unwrap();
        std::fs::write(tmp.join("GameB").join("eboot.bin"), "x").unwrap();
        std::fs::write(tmp.join("not_a_dir.txt"), "x").unwrap();

        let mut dirs = find_game_dirs(&tmp);
        dirs.sort();
        assert_eq!(dirs.len(), 2);
        assert!(dirs[0].ends_with("GameA"));
        assert!(dirs[1].ends_with("GameB"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_game_dirs_skips_dirs_without_eboot() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_scan_test_no_eboot_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("Empty")).unwrap();

        let dirs = find_game_dirs(&tmp);
        assert!(dirs.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_game_dirs_drills_through_single_child() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_scan_test_drill_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("Wrapper").join("Game")).unwrap();
        std::fs::write(tmp.join("Wrapper").join("Game").join("eboot.bin"), "x").unwrap();

        let dirs = find_game_dirs(&tmp);
        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with("Game"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
