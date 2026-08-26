use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBinding {
    pub name: String,
    pub slot: u32,
}
