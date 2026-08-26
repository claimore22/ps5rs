use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructLayout {
    pub name: String,
    pub size: usize,
    pub align: usize,
}
