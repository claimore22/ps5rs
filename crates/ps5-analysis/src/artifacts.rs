use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactCategory {
    Executable,
    Shader,
    Texture,
    Audio,
    Model,
    Metadata,
    Font,
    Archive,
    Unknown,
}

impl ArtifactCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Shader => "shader",
            Self::Texture => "texture",
            Self::Audio => "audio",
            Self::Model => "model",
            Self::Metadata => "metadata",
            Self::Font => "font",
            Self::Archive => "archive",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        let lower = ext.to_ascii_lowercase();
        match lower.as_str() {
            "prx" | "sprx" | "elf" | "so" => Self::Executable,
            "pssl" | "sb" | "ags" | "agsd" => Self::Shader,
            "gnf" | "dds" | "tga" | "bmp" | "png" | "jpg" => Self::Texture,
            "at9" | "bank" => Self::Audio,
            "dae" | "objcache" | "mtl" => Self::Model,
            "ttf" => Self::Font,
            "pak" | "pkg" | "paks" => Self::Archive,
            "json" | "xml" | "txt" | "ucp" | "auth_info" | "irr" | "esbak" | "swatch"
            | "xcache" | "daecache" | "db" | "dat" | "bat" | "bin" => Self::Metadata,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub relative_path: String,
    pub file_name: String,
    pub extension: String,
    pub size: u64,
    pub category: ArtifactCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameArtifacts {
    pub game: String,
    pub game_dir: String,
    pub total_files: usize,
    pub total_bytes: u64,
    pub by_extension: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReport {
    pub games: Vec<GameArtifacts>,
    pub total_games: usize,
    pub total_files: usize,
    pub by_extension: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
}

fn classify_ext(ext: &str) -> ArtifactCategory {
    ArtifactCategory::from_extension(ext)
}

pub fn inventory_game(game_dir: &Path) -> GameArtifacts {
    let game = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut artifacts = Vec::new();
    let mut by_extension: HashMap<String, usize> = HashMap::new();
    let mut by_category: HashMap<String, usize> = HashMap::new();
    let mut total_bytes = 0u64;

    let mut stack = vec![game_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if file_name == "eboot.bin" {
                    // eboot is executable even though extension is bin
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    total_bytes += size;
                    let rel = path
                        .strip_prefix(game_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    *by_extension.entry("eboot.bin".to_string()).or_insert(0) += 1;
                    *by_category
                        .entry(ArtifactCategory::Executable.as_str().to_string())
                        .or_insert(0) += 1;
                    artifacts.push(Artifact {
                        relative_path: rel,
                        file_name,
                        extension: "eboot.bin".to_string(),
                        size,
                        category: ArtifactCategory::Executable,
                    });
                    continue;
                }
                let ext_key = if extension.is_empty() {
                    // handle .auth_info style double extension? Keep as empty for now
                    // try to get full extension via file_name after dot
                    if let Some(dot) = file_name.rfind('.') {
                        file_name[dot + 1..].to_ascii_lowercase()
                    } else {
                        String::new()
                    }
                } else {
                    extension.clone()
                };
                let category = if ext_key == "eboot.bin" {
                    ArtifactCategory::Executable
                } else {
                    classify_ext(&ext_key)
                };
                // special-case auth_info: file has no extension in Path::extension, but name contains it
                let effective_ext = if file_name.ends_with(".auth_info") {
                    "auth_info".to_string()
                } else if ext_key.is_empty() && file_name.contains('.') {
                    file_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                } else {
                    ext_key.clone()
                };
                let effective_category = if file_name.ends_with(".auth_info") {
                    ArtifactCategory::Metadata
                } else {
                    category
                };
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total_bytes += size;
                let rel = path
                    .strip_prefix(game_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let ext_for_map = if effective_ext.is_empty() {
                    "(no_ext)".to_string()
                } else {
                    effective_ext.clone()
                };
                *by_extension.entry(ext_for_map).or_insert(0) += 1;
                *by_category
                    .entry(effective_category.as_str().to_string())
                    .or_insert(0) += 1;
                artifacts.push(Artifact {
                    relative_path: rel,
                    file_name,
                    extension: effective_ext,
                    size,
                    category: effective_category,
                });
            }
        }
    }

    artifacts.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let total_files = artifacts.len();

    GameArtifacts {
        game,
        game_dir: game_dir.to_string_lossy().to_string(),
        total_files,
        total_bytes,
        by_extension,
        by_category,
        artifacts,
    }
}

pub fn inventory_corpus(root: &Path) -> ArtifactReport {
    if root.join("eboot.bin").exists() {
        let inv = inventory_game(root);
        return ArtifactReport {
            games: vec![inv.clone()],
            total_games: 1,
            total_files: inv.total_files,
            by_extension: inv.by_extension.clone(),
            by_category: inv.by_category.clone(),
        };
    }
    let mut games = Vec::new();
    let mut total_files = 0usize;
    let mut by_extension: HashMap<String, usize> = HashMap::new();
    let mut by_category: HashMap<String, usize> = HashMap::new();

    let game_dirs = crate::scanner::find_game_dirs_for_artifacts(root);
    for game_dir in game_dirs {
        let inv = inventory_game(&game_dir);
        total_files += inv.total_files;
        for (k, v) in &inv.by_extension {
            *by_extension.entry(k.clone()).or_insert(0) += v;
        }
        for (k, v) in &inv.by_category {
            *by_category.entry(k.clone()).or_insert(0) += v;
        }
        games.push(inv);
    }
    let total_games = games.len();
    ArtifactReport {
        games,
        total_games,
        total_files,
        by_extension,
        by_category,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_from_extension() {
        assert_eq!(
            ArtifactCategory::from_extension("prx"),
            ArtifactCategory::Executable
        );
        assert_eq!(
            ArtifactCategory::from_extension("pssl"),
            ArtifactCategory::Shader
        );
        assert_eq!(
            ArtifactCategory::from_extension("sb"),
            ArtifactCategory::Shader
        );
        assert_eq!(
            ArtifactCategory::from_extension("gnf"),
            ArtifactCategory::Texture
        );
        assert_eq!(
            ArtifactCategory::from_extension("at9"),
            ArtifactCategory::Audio
        );
        assert_eq!(
            ArtifactCategory::from_extension("json"),
            ArtifactCategory::Metadata
        );
        assert_eq!(
            ArtifactCategory::from_extension("ttf"),
            ArtifactCategory::Font
        );
        assert_eq!(
            ArtifactCategory::from_extension("unknown_ext_xyz"),
            ArtifactCategory::Unknown
        );
    }

    #[test]
    fn inventory_game_empty_dir() {
        let tmp =
            std::env::temp_dir().join(format!("ps5rs_artifact_test_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let inv = inventory_game(&tmp);
        assert_eq!(inv.total_files, 0);
        assert!(inv.artifacts.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn inventory_game_mixed_extensions() {
        let tmp =
            std::env::temp_dir().join(format!("ps5rs_artifact_test_mixed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("Content")).unwrap();
        std::fs::write(tmp.join("eboot.bin"), b"elf").unwrap();
        std::fs::write(tmp.join("Content").join("a.pssl"), b"pssl").unwrap();
        std::fs::write(tmp.join("Content").join("b.sb"), b"sb").unwrap();
        std::fs::write(tmp.join("Content").join("c.gnf"), b"gnf").unwrap();
        std::fs::create_dir_all(tmp.join("sce_module")).unwrap();
        std::fs::write(tmp.join("sce_module").join("lib.prx"), b"prx").unwrap();
        std::fs::write(tmp.join("data.json"), b"{}").unwrap();

        let inv = inventory_game(&tmp);
        assert!(inv.total_files >= 5);
        assert!(inv.by_extension.contains_key("pssl"));
        assert!(inv.by_extension.contains_key("sb"));
        assert!(inv.by_category.contains_key("shader"));
        assert!(inv.by_category.contains_key("executable"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn preserves_relative_paths() {
        let tmp =
            std::env::temp_dir().join(format!("ps5rs_artifact_test_rel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("Content/Shaders/pssl")).unwrap();
        std::fs::write(tmp.join("Content/Shaders/pssl/test.sb"), b"shader").unwrap();
        let inv = inventory_game(&tmp);
        assert_eq!(inv.artifacts.len(), 1);
        assert!(
            inv.artifacts[0]
                .relative_path
                .contains("Content/Shaders/pssl/test.sb")
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn does_not_invent_formats() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_artifact_test_unknown_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("mystery.xyz"), b"unknown content").unwrap();
        let inv = inventory_game(&tmp);
        assert_eq!(inv.artifacts[0].category, ArtifactCategory::Unknown);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
