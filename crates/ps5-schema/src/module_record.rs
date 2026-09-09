use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub name: String,
    pub path: String,
    pub is_prx: bool,
}
