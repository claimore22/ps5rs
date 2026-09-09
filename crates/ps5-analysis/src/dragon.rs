use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DragonCounts {
    pub par_archives: usize,
    pub pxd_archives: usize,
    pub gmd_models: usize,
    pub dds_textures: usize,
}

impl DragonCounts {
    pub fn total(&self) -> usize {
        self.par_archives + self.pxd_archives + self.gmd_models + self.dds_textures
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragonGameReport {
    pub game: String,
    pub counts: DragonCounts,
    pub by_type: HashMap<String, usize>,
    #[serde(default)]
    pub provenance: String,
}

pub fn is_dragon_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "par" | "pxd" | "gmd" | "dds")
}

pub fn inventory_dragon_game(game_dir: &Path) -> Option<DragonGameReport> {
    let game = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut counts = DragonCounts::default();
    let mut stack = vec![game_dir.to_path_buf()];
    let mut found = false;
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_dragon_file(&path) {
                found = true;
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                match ext.as_str() {
                    "par" => counts.par_archives += 1,
                    "pxd" => counts.pxd_archives += 1,
                    "gmd" => counts.gmd_models += 1,
                    "dds" => counts.dds_textures += 1,
                    _ => {}
                }
            }
        }
    }
    if !found {
        return None;
    }
    let mut by_type = HashMap::new();
    by_type.insert("par".to_string(), counts.par_archives);
    by_type.insert("pxd".to_string(), counts.pxd_archives);
    by_type.insert("gmd".to_string(), counts.gmd_models);
    by_type.insert("dds".to_string(), counts.dds_textures);
    Some(DragonGameReport {
        game,
        counts,
        by_type,
        provenance: "heuristic".to_string(),
    })
}

pub fn list_par_with_tool(par_path: &Path, tool_path: &Path) -> Option<Vec<String>> {
    let output = std::process::Command::new(tool_path)
        .arg("list")
        .arg(par_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.lines().map(|l| l.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dragon_file_detection() {
        assert!(is_dragon_file(Path::new("data.par")));
        assert!(is_dragon_file(Path::new("archive.pxd")));
        assert!(is_dragon_file(Path::new("model.gmd")));
        assert!(is_dragon_file(Path::new("tex.dds")));
        assert!(!is_dragon_file(Path::new("image.png")));
        assert!(!is_dragon_file(Path::new("eboot.bin")));
    }

    #[test]
    fn empty_counts_is_empty() {
        assert!(DragonCounts::default().is_empty());
    }

    #[test]
    fn inventory_missing_is_none() {
        let tmp = std::env::temp_dir().join(format!("ps5rs_dragon_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(inventory_dragon_game(&tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn inventory_counts_par_gmd() {
        let tmp = std::env::temp_dir().join(format!("ps5rs_dragon_test2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("a.par"), b"PAR").unwrap();
        std::fs::write(tmp.join("b.gmd"), b"GMD").unwrap();
        std::fs::write(tmp.join("c.dds"), b"DDS").unwrap();
        let rep = inventory_dragon_game(&tmp).unwrap();
        assert_eq!(rep.counts.par_archives, 1);
        assert_eq!(rep.counts.gmd_models, 1);
        assert_eq!(rep.counts.dds_textures, 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
