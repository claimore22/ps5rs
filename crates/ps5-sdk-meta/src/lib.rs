use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRange {
    pub from: String,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkFunction {
    pub nid: String,
    pub name: String,
    pub library: String,
    pub module: Option<String>,
    pub sdk_versions: VersionRange,
    pub category: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SdkDatabase {
    pub functions: HashMap<String, SdkFunction>,
}

impl SdkDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, func: SdkFunction) {
        self.functions.insert(func.nid.clone(), func);
    }

    pub fn get(&self, nid: &str) -> Option<&SdkFunction> {
        self.functions.get(nid)
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }
}
