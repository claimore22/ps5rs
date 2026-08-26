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

    pub fn populate_from_catalog(&mut self, catalog: &ps5_nid::Catalog) {
        // Example: populate from catalog's builtins (if available via iteration - for now just demo)
        let _ = catalog;
    }

    pub fn populate_from_roms(&mut self, roms_path: &str) {
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten().take(2) {
                let name = entry.file_name().to_string_lossy().to_string();
                let func = SdkFunction {
                    nid: ps5_nid::hash(&name),
                    name: name.clone(),
                    library: "unknown".to_string(),
                    module: Some(name),
                    sdk_versions: VersionRange {
                        from: "10.00".to_string(),
                        to: None,
                    },
                    category: "game".to_string(),
                };
                self.insert(func);
            }
        }
    }
}
