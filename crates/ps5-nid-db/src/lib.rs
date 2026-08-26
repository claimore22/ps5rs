use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NidSource {
    Builtin,
    SdkStub,
    Supabase,
    Manual,
    RemuCrossRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    Verified,
    High,
    Medium,
    Low,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LibraryId(pub String);

impl From<String> for LibraryId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for LibraryId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRange {
    pub from: String,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NidRecord {
    pub nid: String,
    pub library: LibraryId,
    pub name: Option<String>,
    pub versions: Option<VersionRange>,
    pub source: NidSource,
    pub confidence: Confidence,
    pub aliases: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub struct NidDatabase {
    records: HashMap<String, NidRecord>,
}

impl NidDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_catalog(catalog: &ps5_nid::Catalog) -> Self {
        // Convert from ps5-nid Catalog by wrapping - for now, create empty and populate via Catalog's internal map is private,
        // so we just create a placeholder that can be populated via insert
        let _ = catalog;
        Self::new()
    }

    pub fn insert(&mut self, record: NidRecord) {
        self.records.insert(record.nid.clone(), record);
    }

    pub fn get(&self, nid: &str) -> Option<&NidRecord> {
        self.records.get(nid)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
