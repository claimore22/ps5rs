use serde::{Deserialize, Serialize};

use crate::shader_binary::ShaderStage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShaderMetadata {
    pub stage: ShaderStage,
    pub entry_point: String,
    pub num_threads: Option<(u32, u32, u32)>,
    pub hash: String,
    pub size: usize,
}

impl ShaderMetadata {
    pub fn from_binary(binary: &crate::shader_binary::ShaderBinary, entry: &str) -> Self {
        Self {
            stage: binary.stage.clone(),
            entry_point: entry.to_string(),
            num_threads: None,
            hash: binary.hash.clone(),
            size: binary.size,
        }
    }

    pub fn with_threads(mut self, x: u32, y: u32, z: u32) -> Self {
        self.num_threads = Some((x, y, z));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_binary() {
        let bin = crate::shader_binary::ShaderBinary::parse(b"vertex data").unwrap();
        let meta = ShaderMetadata::from_binary(&bin, "main");
        assert_eq!(meta.stage, crate::shader_binary::ShaderStage::Vertex);
        assert_eq!(meta.entry_point, "main");
    }
}
