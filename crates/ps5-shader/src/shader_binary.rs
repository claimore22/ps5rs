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
        let stage = if data.len() > 4 && data[0..4] == [0x47, 0x43, 0x4E, 0x00] {
            ShaderStage::Vertex
        } else if data.windows(6).any(|w| w == b"vertex") {
            ShaderStage::Vertex
        } else if data.windows(5).any(|w| w == b"pixel") {
            ShaderStage::Pixel
        } else if data.windows(7).any(|w| w == b"compute") {
            ShaderStage::Compute
        } else {
            ShaderStage::Unknown("unknown".to_string())
        };
        let hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };
        let entry_point = if data.len() >= 8 {
            u64::from_le_bytes(data[0..8].try_into().unwrap_or([0; 8]))
        } else {
            0
        };
        Ok(Self {
            stage,
            size: data.len(),
            hash,
            entry_point,
        })
    }

    pub fn from_roms(roms_path: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return out;
        }
        let walker = walkdir_simple(path);
        for file in walker {
            if let Ok(data) = std::fs::read(&file) {
                if let Ok(shader) = Self::parse(&data) {
                    out.push(shader);
                }
            }
        }
        out
    }
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
                } else if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if matches!(ext.to_ascii_lowercase().as_str(), "bin" | "sb" | "ags" | "agsd" | "gnf" | "elf" | "prx") {
                            files.push(p);
                        }
                    }
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
        let data = b"some vertex shader code";
        let s = ShaderBinary::parse(data).unwrap();
        assert_eq!(s.stage, ShaderStage::Vertex);
        assert_eq!(s.size, data.len());
    }

    #[test]
    fn parse_compute() {
        let data = b"compute shader binary blob";
        let s = ShaderBinary::parse(data).unwrap();
        assert_eq!(s.stage, ShaderStage::Compute);
    }

    #[test]
    fn from_roms_missing_is_empty() {
        let v = ShaderBinary::from_roms("/nonexistent/path/xyz");
        assert!(v.is_empty());
    }
}
