use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrxMetadata {
    pub soname: Option<String>,
    pub needed_files: Vec<String>,
    pub import_libs: Vec<String>,
}
