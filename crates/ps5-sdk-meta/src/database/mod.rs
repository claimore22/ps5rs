use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::functions::SdkFunction;
use crate::versions::VersionRange;

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

    pub fn get_by_name(&self, name: &str) -> Option<&SdkFunction> {
        self.functions.values().find(|f| f.name == name)
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    pub fn query_by_library(&self, lib: &str) -> Vec<&SdkFunction> {
        self.functions.values().filter(|f| f.library == lib).collect()
    }

    pub fn query_by_category(&self, cat: &str) -> Vec<&SdkFunction> {
        self.functions.values().filter(|f| f.category == cat).collect()
    }

    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let funcs: Vec<SdkFunction> = serde_json::from_str(json)?;
        let mut db = Self::new();
        for f in funcs {
            db.insert(f);
        }
        Ok(db)
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        let v: Vec<&SdkFunction> = self.functions.values().collect();
        serde_json::to_string_pretty(&v)
    }

    pub fn load_from_path(path: &Path) -> Result<Self, serde_json::Error> {
        let data = std::fs::read_to_string(path).map_err(serde_json::Error::io)?;
        Self::from_json_str(&data)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), serde_json::Error> {
        let s = self.to_json_string()?;
        std::fs::write(path, s).map_err(serde_json::Error::io)?;
        Ok(())
    }

    pub fn populate_from_catalog(&mut self, catalog: &ps5_nid::Catalog) {
        let _ = catalog;
    }

    pub fn populate_from_stubs_dir(&mut self, stubs_path: &Path) {
        if !stubs_path.exists() {
            return;
        }
        let entries = match std::fs::read_dir(stubs_path) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("txt") {
                continue;
            }
            let library = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let category = infer_category(&library);
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let mut parts = line.split_whitespace();
                    let nid = match parts.next() {
                        Some(n) => n,
                        None => continue,
                    };
                    let name = match parts.next() {
                        Some(n) => n,
                        None => continue,
                    };
                    let func = SdkFunction::new(
                        nid,
                        name,
                        library.clone(),
                        Some(format!("{}.prx", library)),
                        VersionRange::single("1.00"),
                        category.clone(),
                    );
                    self.insert(func);
                }
            }
        }
    }

    pub fn populate_from_nids_csv(&mut self, csv_path: &Path) {
        if !csv_path.exists() {
            return;
        }
        if let Ok(data) = std::fs::read_to_string(csv_path) {
            for line in data.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with("nid,") || line.starts_with("nid_hex") {
                    continue;
                }
                let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if cols.len() < 3 {
                    continue;
                }
                let nid = cols[0];
                let name = cols[2];
                let library = cols.get(3).filter(|s| !s.is_empty()).unwrap_or(&"unknown").to_string();
                if nid.is_empty() || name.is_empty() {
                    continue;
                }
                let func = SdkFunction::new(
                    nid,
                    name,
                    library.clone(),
                    Some(format!("{}.prx", library)),
                    VersionRange::single("1.00"),
                    infer_category(&library),
                );
                self.insert(func);
            }
        }
    }
}

fn infer_category(library: &str) -> String {
    let lower = library.to_ascii_lowercase();
    if lower.contains("kernel") || lower.contains("posix") || lower.contains("pthread") {
        "kernel".to_string()
    } else if lower.contains("audio") || lower.contains("acm") || lower.contains("ngs") {
        "audio".to_string()
    } else if lower.contains("video") || lower.contains("gnm") || lower.contains("agc") {
        "graphics".to_string()
    } else if lower.contains("pad") || lower.contains("camera") {
        "input".to_string()
    } else if lower.contains("net") || lower.contains("http") || lower.contains("ssl") {
        "network".to_string()
    } else if lower.contains("save") || lower.contains("pfs") {
        "storage".to_string()
    } else {
        "system".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("ps5_sdk_db_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&p);
        let _ = std::fs::remove_dir_all(&p);
        let _ = std::fs::create_dir_all(&p);
        p
    }

    #[test]
    fn insert_and_query() {
        let mut db = SdkDatabase::new();
        let f = SdkFunction::new("NID123", "myFunc", "libTest", None, VersionRange::single("9.00"), "kernel");
        db.insert(f);
        assert_eq!(db.len(), 1);
        assert!(db.get("NID123").is_some());
        assert!(db.get_by_name("myFunc").is_some());
        assert_eq!(db.query_by_library("libTest").len(), 1);
    }

    #[test]
    fn json_roundtrip() {
        let mut db = SdkDatabase::new();
        db.insert(SdkFunction::new("N1", "f1", "libA", None, VersionRange::single("1.00"), "audio"));
        let s = db.to_json_string().unwrap();
        let db2 = SdkDatabase::from_json_str(&s).unwrap();
        assert_eq!(db2.len(), 1);
    }

    #[test]
    fn populate_from_stubs_tmp() {
        let dir = tmp_dir();
        let file = dir.join("libTest.txt");
        std::fs::write(&file, "ABC123 myTestFunc\nDEF456 otherFunc\n").unwrap();
        let mut db = SdkDatabase::new();
        db.populate_from_stubs_dir(&dir);
        assert_eq!(db.len(), 2);
        assert!(db.get("ABC123").is_some());
        let _ = std::fs::remove_dir_all(dir);
    }
}


