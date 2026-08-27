use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FirmwareExportTable {
    pub by_nid: HashMap<String, Vec<String>>,
    pub by_library: HashMap<String, Vec<String>>,
}

impl FirmwareExportTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, nid: String, library: String) {
        self.by_nid.entry(nid.clone()).or_default().push(library.clone());
        self.by_library.entry(library).or_default().push(nid);
    }

    pub fn get_libraries_for_nid(&self, nid: &str) -> Option<&Vec<String>> {
        self.by_nid.get(nid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_lookup() {
        let mut t = FirmwareExportTable::new();
        t.insert("ABC".to_string(), "libA".to_string());
        assert_eq!(t.get_libraries_for_nid("ABC").unwrap()[0], "libA");
    }
}
