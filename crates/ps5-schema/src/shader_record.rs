use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderRecord {
    pub name: String,
    pub stage: String,
}
