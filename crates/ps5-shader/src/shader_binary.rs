#![allow(clippy::collapsible_if)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Pixel,
    Compute,
    Hull,
    Domain,
    Geometry,
    Unknown(String),
}

impl ShaderStage {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Vertex => "vertex",
            Self::Pixel => "pixel",
            Self::Compute => "compute",
            Self::Hull => "hull",
            Self::Domain => "domain",
            Self::Geometry => "geometry",
            Self::Unknown(s) => s.as_str(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "vertex" | "vs" => Self::Vertex,
            "pixel" | "ps" | "fragment" => Self::Pixel,
            "compute" | "cs" => Self::Compute,
            "hull" | "hs" => Self::Hull,
            "domain" | "ds" => Self::Domain,
            "geometry" | "gs" => Self::Geometry,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderBinary {
    pub stage: ShaderStage,
    pub size: usize,
    pub hash: String,
    pub entry_point: u64,
}

impl ShaderBinary {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("empty shader data".to_string());
        }
        let stage = if let Some(pos) = find_shdr(data) {
            let stage_off = pos + 12;
            if stage_off < data.len() {
                match data[stage_off] {
                    1 => ShaderStage::Vertex,
                    2 => ShaderStage::Pixel,
                    3 => ShaderStage::Compute,
                    other => ShaderStage::Unknown(format!("type_{other}")),
                }
            } else {
                ShaderStage::Unknown("unknown".to_string())
            }
        } else if data.len() > 4 && data[0..4] == [0x47, 0x43, 0x4E, 0x00] {
            ShaderStage::Vertex
        } else {
            ShaderStage::Unknown("unknown".to_string())
        };
        let hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(data);
            let result = hasher.finalize();
            result
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let entry_point = 0;
        Ok(Self {
            stage,
            size: data.len(),
            hash,
            entry_point,
        })
    }

    pub fn parse_with_path(data: &[u8], path: &std::path::Path) -> Result<Self, String> {
        let mut shader = Self::parse(data)?;
        if matches!(shader.stage, ShaderStage::Unknown(_)) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_ascii_lowercase();
                if lower.contains("_vs") || lower.contains("_vv") || lower.contains("_vertex") {
                    shader.stage = ShaderStage::Vertex;
                } else if lower.contains("_ps") || lower.contains("_p.") || lower.contains("_pixel")
                {
                    shader.stage = ShaderStage::Pixel;
                } else if lower.contains("_cs") || lower.contains("_compute") {
                    shader.stage = ShaderStage::Compute;
                }
            }
        }
        Ok(shader)
    }

    pub fn from_roms(roms_path: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return out;
        }
        let walker = walkdir_simple(path);
        for file in walker {
            if let Ok(data) = std::fs::read(&file)
                && let Ok(shader) = Self::parse_with_path(&data, &file)
            {
                if !matches!(shader.stage, ShaderStage::Unknown(_)) || data.len() >= 32 {
                    out.push(shader);
                }
            }
        }
        out
    }
}

fn find_shdr(data: &[u8]) -> Option<usize> {
    let limit = usize::min(data.len().saturating_sub(16), 512);
    for i in 0..=limit {
        if data[i..].starts_with(b"Shdr") {
            return Some(i);
        }
    }
    None
}

fn walkdir_simple(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file()
                    && let Some(ext) = p.extension().and_then(|s| s.to_str())
                    && matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "sb" | "ags" | "agsd" | "pssl"
                    )
                {
                    files.push(p);
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_fails() {
        assert!(ShaderBinary::parse(&[]).is_err());
    }

    #[test]
    fn parse_vertex_keyword() {
        let mut data = vec![0u8; 50];
        data[32..36].copy_from_slice(b"Shdr");
        data[44] = 1;
        let s = ShaderBinary::parse(&data).unwrap();
        assert_eq!(s.stage, ShaderStage::Vertex);
        assert_eq!(s.size, data.len());
    }

    #[test]
    fn parse_compute() {
        let mut data = vec![0u8; 50];
        data[32..36].copy_from_slice(b"Shdr");
        data[44] = 3;
        let s = ShaderBinary::parse(&data).unwrap();
        assert_eq!(s.stage, ShaderStage::Compute);
    }

    #[test]
    fn from_roms_missing_is_empty() {
        let v = ShaderBinary::from_roms("/nonexistent/path/xyz");
        assert!(v.is_empty());
    }
}
