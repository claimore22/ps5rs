use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderBinary {
    pub stage: String,
}

impl ShaderBinary {
    pub fn from_roms(roms_path: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return out;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten().take(1) {
                out.push(Self {
                    stage: "vertex".to_string(),
                });
                let _ = entry;
            }
        }
        out
    }
}
