use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkFunction {
    pub nid: String,
    pub name: String,
    pub library: String,
}
