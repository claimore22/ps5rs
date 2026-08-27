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
    by_library: HashMap<LibraryId, Vec<String>>,
    by_name: HashMap<String, Vec<String>>,
}

impl NidDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_catalog(catalog: &ps5_nid::Catalog) -> Self {
        let _ = catalog;
        Self::new()
    }

    pub fn from_records(records: Vec<NidRecord>) -> Self {
        let mut db = Self::new();
        for r in records {
            db.insert(r);
        }
        db
    }

    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let records: Vec<NidRecord> = serde_json::from_str(json)?;
        Ok(Self::from_records(records))
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        let records: Vec<&NidRecord> = self.records.values().collect();
        serde_json::to_string_pretty(&records)
    }

    pub fn insert(&mut self, record: NidRecord) {
        let nid = record.nid.clone();
        let lib = record.library.clone();
        if let Some(name) = record.name.clone() {
            self.by_name.entry(name).or_default().push(nid.clone());
            for alias in &record.aliases {
                self.by_name.entry(alias.clone()).or_default().push(nid.clone());
            }
        } else {
            for alias in &record.aliases {
                self.by_name.entry(alias.clone()).or_default().push(nid.clone());
            }
        }
        self.by_library.entry(lib).or_default().push(nid.clone());
        self.records.insert(nid, record);
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

    pub fn query_by_library(&self, lib: &str) -> Vec<&NidRecord> {
        let key = LibraryId(lib.to_string());
        if let Some(nids) = self.by_library.get(&key) {
            nids.iter().filter_map(|nid| self.records.get(nid)).collect()
        } else {
            self.records
                .values()
                .filter(|r| r.library.0 == lib)
                .collect()
        }
    }

    pub fn query_by_name(&self, name: &str) -> Vec<&NidRecord> {
        if let Some(nids) = self.by_name.get(name) {
            let mut seen = std::collections::BTreeSet::new();
            let mut out = Vec::new();
            for nid in nids {
                if seen.insert(nid) {
                    if let Some(rec) = self.records.get(nid) {
                        out.push(rec);
                    }
                }
            }
            out
        } else {
            Vec::new()
        }
    }

    pub fn all_records(&self) -> Vec<&NidRecord> {
        self.records.values().collect()
    }

    pub fn libraries(&self) -> Vec<LibraryId> {
        self.by_library.keys().cloned().collect()
    }
}

pub fn load_from_path(path: &std::path::Path) -> Result<NidDatabase, serde_json::Error> {
    let data = std::fs::read_to_string(path).map_err(serde_json::Error::io)?;
    let records: Vec<NidRecord> = serde_json::from_str(&data)?;
    Ok(NidDatabase::from_records(records))
}

pub fn save_to_path(db: &NidDatabase, path: &std::path::Path) -> Result<(), serde_json::Error> {
    let json = db.to_json_string()?;
    std::fs::write(path, json).map_err(serde_json::Error::io)?;
    Ok(())
}

pub fn from_nid_csv_str(csv: &str) -> NidDatabase {
    let mut db = NidDatabase::new();
    let mut catalog = ps5_nid::Catalog::new();
    catalog.load_nids_csv(csv);
    // Catalog is private map; re-parse CSV directly into NidRecords for richer mapping
    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("nid") {
            continue;
        }
        // Try 5-col rich: nid,name,library,tag,source
        let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if cols.len() >= 2 && !cols[0].is_empty() && !cols[1].is_empty() {
            let nid = cols[0].to_string();
            let name = cols[1].to_string();
            let library = cols.get(2).filter(|s| !s.is_empty()).map(|s| LibraryId(s.to_string())).unwrap_or(LibraryId("unknown".to_string()));
            let source = match cols.get(4).map(|s| *s).unwrap_or("Builtin") {
                s if s.eq_ignore_ascii_case("sdk") => NidSource::SdkStub,
                s if s.eq_ignore_ascii_case("supabase") => NidSource::Supabase,
                s if s.eq_ignore_ascii_case("manual") => NidSource::Manual,
                _ => NidSource::Builtin,
            };
            let rec = NidRecord {
                nid: nid.clone(),
                library,
                name: Some(name.clone()),
                versions: None,
                source,
                confidence: Confidence::Medium,
                aliases: BTreeSet::from([name]),
            };
            db.insert(rec);
        }
    }
    db
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn sample_record(nid: &str, lib: &str, name: &str) -> NidRecord {
        NidRecord {
            nid: nid.to_string(),
            library: LibraryId(lib.to_string()),
            name: Some(name.to_string()),
            versions: Some(VersionRange {
                from: "9.00".to_string(),
                to: Some("11.00".to_string()),
            }),
            source: NidSource::Builtin,
            confidence: Confidence::Verified,
            aliases: BTreeSet::from([name.to_string()]),
        }
    }

    #[test]
    fn insert_and_get() {
        let mut db = NidDatabase::new();
        let rec = sample_record("AAAABBBBCCCC", "libSceKernel", "sceKernelSleep");
        db.insert(rec.clone());
        assert_eq!(db.len(), 1);
        let got = db.get("AAAABBBBCCCC").unwrap();
        assert_eq!(got.name, Some("sceKernelSleep".to_string()));
        assert_eq!(got.library.0, "libSceKernel");
    }

    #[test]
    fn query_by_library() {
        let mut db = NidDatabase::new();
        db.insert(sample_record("NID1", "libA", "funcA"));
        db.insert(sample_record("NID2", "libA", "funcB"));
        db.insert(sample_record("NID3", "libB", "funcC"));
        let lib_a = db.query_by_library("libA");
        assert_eq!(lib_a.len(), 2);
        let lib_b = db.query_by_library("libB");
        assert_eq!(lib_b.len(), 1);
        let missing = db.query_by_library("libMissing");
        assert!(missing.is_empty());
    }

    #[test]
    fn query_by_name_via_alias() {
        let mut db = NidDatabase::new();
        let mut rec = sample_record("NID99", "libTest", "myFunc");
        rec.aliases.insert("aliasFunc".to_string());
        db.insert(rec);
        assert_eq!(db.query_by_name("myFunc").len(), 1);
        assert_eq!(db.query_by_name("aliasFunc").len(), 1);
        assert!(db.query_by_name("nope").is_empty());
    }

    #[test]
    fn json_roundtrip() {
        let mut db = NidDatabase::new();
        db.insert(sample_record("NID_JSON1", "libJson", "jsonFunc"));
        let json = db.to_json_string().unwrap();
        let db2 = NidDatabase::from_json_str(&json).unwrap();
        assert_eq!(db2.len(), 1);
        assert_eq!(db2.get("NID_JSON1").unwrap().name, Some("jsonFunc".to_string()));
    }

    #[test]
    fn load_from_path_json() {
        let dir = std::env::temp_dir().join("ps5_nid_db_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_catalog.json");
        let mut db = NidDatabase::new();
        db.insert(sample_record("LOAD123", "libLoad", "loadFunc"));
        save_to_path(&db, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("LOAD123").unwrap().library.0, "libLoad");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_range_serialization() {
        let rec = sample_record("VR_NID", "libVR", "vrFunc");
        let json = serde_json::to_string(&rec).unwrap();
        let de: NidRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(de.versions.unwrap().from, "9.00");
    }

    #[test]
    fn confidence_and_source_roundtrip() {
        let rec = NidRecord {
            nid: "SRC_TEST".to_string(),
            library: LibraryId("libSrc".to_string()),
            name: None,
            versions: None,
            source: NidSource::SdkStub,
            confidence: Confidence::High,
            aliases: BTreeSet::new(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let de: NidRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(de.source, NidSource::SdkStub);
        assert_eq!(de.confidence, Confidence::High);
        assert!(de.name.is_none());
    }

    #[test]
    fn libraries_enumeration() {
        let mut db = NidDatabase::new();
        db.insert(sample_record("L1", "libX", "f1"));
        db.insert(sample_record("L2", "libY", "f2"));
        let libs = db.libraries();
        assert_eq!(libs.len(), 2);
        assert!(libs.contains(&LibraryId("libX".to_string())));
        assert!(libs.contains(&LibraryId("libY".to_string())));
    }
}
