use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnityAssetCounts {
    pub shader: usize,
    pub compute_shader: usize,
    pub material: usize,
    pub texture2d: usize,
    pub mesh: usize,
    pub animation_clip: usize,
    pub mono_behaviour: usize,
    pub text_asset: usize,
}

impl UnityAssetCounts {
    pub fn total(&self) -> usize {
        self.shader
            + self.compute_shader
            + self.material
            + self.texture2d
            + self.mesh
            + self.animation_clip
            + self.mono_behaviour
            + self.text_asset
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnityGameReport {
    pub game: String,
    pub unity_files: usize,
    pub counts: UnityAssetCounts,
    pub by_type: HashMap<String, usize>,
    #[serde(default)]
    pub provenance: String,
    #[serde(default)]
    pub confidence: String,
}

pub fn is_unity_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "assets" | "bundle" | "unity3d" | "ress" | "resource"
        ) {
            return true;
        }
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name == "globalgamemanagers"
        || name == "globalgamemanagers.assets"
        || name.starts_with("level")
        || name.starts_with("sharedassets")
        || name.starts_with("resources")
        || name == "maindata"
    {
        return true;
    }
    let path_str = path.to_string_lossy().to_ascii_lowercase();
    path_str.contains("globalgamemanagers") || path_str.contains("sharedassets")
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        let before_ok = abs == 0 || !haystack.as_bytes()[abs - 1].is_ascii_alphanumeric();
        let after = abs + needle.len();
        let after_ok =
            after >= haystack.len() || !haystack.as_bytes()[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            count += 1;
        }
        start = abs + needle.len();
        if start >= haystack.len() {
            break;
        }
    }
    count
}

pub fn scan_unity_file(path: &Path) -> UnityAssetCounts {
    if let Some(counts) = try_unitypy_scan(path) {
        return counts;
    }
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return UnityAssetCounts::default(),
    };
    if data.len() < 16 {
        return UnityAssetCounts::default();
    }
    let slice = if data.len() > 512 * 1024 {
        &data[..512 * 1024]
    } else {
        &data[..]
    };
    let text = String::from_utf8_lossy(slice);
    UnityAssetCounts {
        shader: count_occurrences(&text, "Shader"),
        compute_shader: count_occurrences(&text, "ComputeShader"),
        material: count_occurrences(&text, "Material"),
        texture2d: count_occurrences(&text, "Texture2D"),
        mesh: count_occurrences(&text, "Mesh"),
        animation_clip: count_occurrences(&text, "AnimationClip"),
        mono_behaviour: count_occurrences(&text, "MonoBehaviour"),
        text_asset: count_occurrences(&text, "TextAsset"),
    }
}

fn try_unitypy_scan(path: &Path) -> Option<UnityAssetCounts> {
    if std::env::var("PS5RS_UNITYPY").is_err() {
        return None;
    }
    let python = std::env::var("UNITYPY_PYTHON").unwrap_or_else(|_| "py".to_string());
    let script = format!(
        "import UnityPy, json; p=r'''{}'''; env=UnityPy.load(p); from collections import Counter; c=Counter(obj.type.name for obj in env.objects); print(json.dumps(dict(c)))",
        path.to_string_lossy().replace('\'', "\\'")
    );
    let output = std::process::Command::new(python)
        .args(["-c", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: HashMap<String, usize> = serde_json::from_str(stdout.trim()).ok()?;
    Some(UnityAssetCounts {
        shader: *json.get("Shader").unwrap_or(&0),
        compute_shader: *json.get("ComputeShader").unwrap_or(&0),
        material: *json.get("Material").unwrap_or(&0),
        texture2d: *json.get("Texture2D").unwrap_or(&0),
        mesh: *json.get("Mesh").unwrap_or(&0),
        animation_clip: *json.get("AnimationClip").unwrap_or(&0),
        mono_behaviour: *json.get("MonoBehaviour").unwrap_or(&0),
        text_asset: *json.get("TextAsset").unwrap_or(&0),
    })
}

pub fn inventory_unity_game(game_dir: &Path) -> Option<UnityGameReport> {
    let game = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut total = UnityAssetCounts::default();
    let mut unity_files = 0usize;
    let mut stack = vec![game_dir.to_path_buf()];
    let use_unitypy = std::env::var("PS5RS_UNITYPY").is_ok();
    let provenance = if use_unitypy { "unitypy" } else { "heuristic" };
    let confidence = if use_unitypy { "verified" } else { "heuristic" };
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_unity_file(&path) {
                unity_files += 1;
                let counts = scan_unity_file(&path);
                total.shader += counts.shader;
                total.compute_shader += counts.compute_shader;
                total.material += counts.material;
                total.texture2d += counts.texture2d;
                total.mesh += counts.mesh;
                total.animation_clip += counts.animation_clip;
                total.mono_behaviour += counts.mono_behaviour;
                total.text_asset += counts.text_asset;
            }
        }
    }
    if unity_files == 0 {
        return None;
    }
    let mut by_type = HashMap::new();
    by_type.insert("Shader".to_string(), total.shader);
    by_type.insert("ComputeShader".to_string(), total.compute_shader);
    by_type.insert("Material".to_string(), total.material);
    by_type.insert("Texture2D".to_string(), total.texture2d);
    by_type.insert("Mesh".to_string(), total.mesh);
    by_type.insert("AnimationClip".to_string(), total.animation_clip);
    by_type.insert("MonoBehaviour".to_string(), total.mono_behaviour);
    by_type.insert("TextAsset".to_string(), total.text_asset);
    Some(UnityGameReport {
        game,
        unity_files,
        counts: total,
        by_type,
        provenance: provenance.to_string(),
        confidence: confidence.to_string(),
    })
}

#[allow(dead_code)]
fn which_unitypy_available() -> bool {
    std::process::Command::new("py")
        .args(["-c", "import UnityPy"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn inventory_unity_corpus(root: &Path) -> Vec<UnityGameReport> {
    let game_dirs = crate::scanner::find_game_dirs_for_artifacts(root);
    let mut reports = Vec::new();
    for dir in game_dirs {
        if let Some(rep) = inventory_unity_game(&dir)
            && (rep.counts.total() > 0 || rep.unity_files > 0)
        {
            reports.push(rep);
        }
    }
    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_counts_is_empty() {
        let c = UnityAssetCounts::default();
        assert!(c.is_empty());
        assert_eq!(c.total(), 0);
    }

    #[test]
    fn is_unity_file_detection() {
        assert!(is_unity_file(Path::new("globalgamemanagers")));
        assert!(is_unity_file(Path::new("sharedassets0.assets")));
        assert!(is_unity_file(Path::new("level0")));
        assert!(is_unity_file(Path::new("data.bundle")));
        assert!(!is_unity_file(Path::new("image.png")));
        assert!(!is_unity_file(Path::new("eboot.bin")));
    }

    #[test]
    fn count_occurrences_word_boundary() {
        assert_eq!(count_occurrences("Shader ShaderLab Shader", "Shader"), 2);
        assert_eq!(count_occurrences("Material Material", "Material"), 2);
        assert_eq!(count_occurrences("ShaderLab", "Shader"), 0);
    }

    #[test]
    fn inventory_missing_is_empty() {
        let tmp = std::env::temp_dir().join(format!("ps5rs_unity_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let rep = inventory_unity_game(&tmp);
        assert!(rep.is_none() || rep.unwrap().unity_files == 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
