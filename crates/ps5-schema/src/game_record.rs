use serde::{Deserialize, Serialize};

use crate::module_record::ModuleRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub title_id: Option<String>,
    pub name: String,
    pub modules: Vec<ModuleRecord>,
}
