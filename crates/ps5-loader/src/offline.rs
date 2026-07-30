use std::collections::HashMap;
use std::path::Path;

/// An entry in the offline export table — a known system export.
#[derive(Debug, Clone)]
pub struct OfflineExportEntry {
    /// Human-readable name of the export.
    pub name: String,
    /// Module that provides this export (e.g. "libc.prx").
    pub module_name: String,
}

/// Offline SDK export table, keyed by numeric NID.
///
/// Loaded from `system_modules/*.exports.json` files produced by
/// `ps5rs exports --json`.  Provides NID → name resolution for system
/// PRXes that aren't loaded at runtime.
///
/// Used by [`CrossModuleResolver`](crate::CrossModuleResolver) to
/// distinguish "known system function (not loaded)" from "unknown NID"
/// during import resolution.
#[derive(Debug, Clone, Default)]
pub struct OfflineExportTable {
    entries: HashMap<u64, OfflineExportEntry>,
}

/// JSON structure matching `ps5rs exports --json` output.
#[derive(serde::Deserialize)]
struct ExportsFile {
    #[serde(rename = "module")]
    module: String,
    exports: Vec<ExportRow>,
}

#[derive(serde::Deserialize)]
struct ExportRow {
    nid: String,
    name: String,
}

impl OfflineExportTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load exports from a directory of `*.exports.json` files.
    ///
    /// Silently returns an empty table if `dir` does not exist or any
    /// individual file fails to parse — partial loads are better than
    /// hard failures at this stage.
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut table = Self::new();
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return table;
        };
        for entry in read_dir {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Ok(data) = std::fs::read_to_string(&path) else { continue };
            let Ok(file) = serde_json::from_str::<ExportsFile>(&data) else { continue };
            let module_name = file.module;
            for row in file.exports {
                let Some(nid) = crate::nid::nid_to_u64(&row.nid) else { continue };
                table.entries.insert(
                    nid,
                    OfflineExportEntry {
                        name: row.name,
                        module_name: module_name.clone(),
                    },
                );
            }
        }
        table
    }

    /// Look up an export by numeric NID.
    pub fn get_by_nid(&self, nid: u64) -> Option<&OfflineExportEntry> {
        self.entries.get(&nid)
    }

    /// Total number of known exports across all loaded system modules.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
