use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderMetadata {
    pub stage: String,
    pub entry_point: String,
}
