use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NidRecord {
    pub nid: String,
    pub name: Option<String>,
    pub library: String,
    pub confidence: u8,
}
