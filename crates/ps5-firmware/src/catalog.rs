#![allow(clippy::collapsible_if)]
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exports::FirmwareExportTable;
use crate::libraries::FirmwareLibrary;
use crate::modules::FirmwareModule;
use crate::version::FirmwareVersion;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryAvailability {
    Compatible,
    Insufficient { required: String, available: String },
    NotFound,
    Unknown { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareCatalog {
    pub version: FirmwareVersion,
    pub modules: Vec<FirmwareModule>,
    pub libraries: Vec<FirmwareLibrary>,
    pub exports: FirmwareExportTable,
}

impl FirmwareCatalog {
    pub fn new(version: FirmwareVersion) -> Self {
        Self {
            version,
            modules: Vec::new(),
            libraries: Vec::new(),
            exports: FirmwareExportTable::new(),
        }
    }

    pub fn with_version_str(s: &str) -> Option<Self> {
        FirmwareVersion::parse(s).map(Self::new)
    }

    pub fn populate_from_roms(&mut self, roms_path: &Path) {
        if !roms_path.exists() {
            return;
        }
        let mut libs: HashMap<String, Vec<String>> = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(roms_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // try to parse as ELF to extract soname
                    if let Ok(data) = std::fs::read(path.clone()) {
                        if let Ok(img) = ps5_elf::ElfImage::parse(&data, None) {
                            if let Some(soname) = img.soname.clone() {
                                self.modules.push(FirmwareModule::new(
                                    soname.clone(),
                                    path.to_string_lossy().to_string(),
                                    "1.0",
                                    img.symbols.iter().filter(|s| !s.is_import).count(),
                                ));
                                libs.entry(soname.clone()).or_default().push(soname.clone());
                                for sym in &img.symbols {
                                    if !sym.is_import && !sym.resolved_name.is_empty() {
                                        let nid = sym
                                            .resolved_name
                                            .split('#')
                                            .next()
                                            .unwrap_or("")
                                            .to_string();
                                        if !nid.is_empty() {
                                            self.exports.insert(nid, soname.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    // recurse one level
                    if let Ok(sub) = std::fs::read_dir(&path) {
                        for sub_entry in sub.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_file() {
                                if let Ok(data) = std::fs::read(&sub_path) {
                                    if let Ok(img) = ps5_elf::ElfImage::parse(&data, None) {
                                        if let Some(soname) = img.soname.clone() {
                                            self.modules.push(FirmwareModule::new(
                                                soname.clone(),
                                                sub_path.to_string_lossy().to_string(),
                                                "1.0",
                                                img.symbols.iter().filter(|s| !s.is_import).count(),
                                            ));
                                            libs.entry(soname.clone())
                                                .or_default()
                                                .push(soname.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for (name, mods) in libs {
            self.libraries.push(FirmwareLibrary::new(name, "1.0", mods));
        }
    }

    pub fn find_module(&self, name: &str) -> Option<&FirmwareModule> {
        self.modules.iter().find(|m| m.name == name)
    }

    pub fn is_library_available(&self, lib: &str, required_version: &str) -> bool {
        if let Some(l) = self.libraries.iter().find(|l| l.name == lib) {
            if let (Some(req), Some(have)) = (
                FirmwareVersion::parse(required_version),
                FirmwareVersion::parse(&l.version),
            ) {
                return have >= req;
            }
            return l.version == required_version;
        }
        false
    }

    pub fn check_library(&self, lib: &str, required_version: &str) -> LibraryAvailability {
        if let Some(l) = self.libraries.iter().find(|l| l.name == lib) {
            match (
                FirmwareVersion::parse(required_version),
                FirmwareVersion::parse(&l.version),
            ) {
                (Some(req), Some(have)) => {
                    if have >= req {
                        LibraryAvailability::Compatible
                    } else {
                        LibraryAvailability::Insufficient {
                            required: required_version.to_string(),
                            available: l.version.clone(),
                        }
                    }
                }
                _ => LibraryAvailability::Unknown {
                    reason: format!(
                        "unparseable version: required={required_version} available={}",
                        l.version
                    ),
                },
            }
        } else {
            LibraryAvailability::NotFound
        }
    }

    pub fn load_exports_from_dir(&mut self, dir: &Path) -> usize {
        if !dir.is_dir() {
            return 0;
        }
        let mut total = 0usize;
        let mut libs: HashMap<String, Vec<String>> = HashMap::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !file_name.ends_with(".exports.json") {
                continue;
            }
            let data = match std::fs::read_to_string(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let parsed: ExportFile = match serde_json::from_str(&data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let module_name = parsed.module.clone();
            let library = module_name
                .strip_suffix(".prx")
                .unwrap_or(&module_name)
                .to_string();
            self.modules.push(FirmwareModule::new(
                module_name.clone(),
                path.to_string_lossy().to_string(),
                "1.0",
                parsed.exports.len(),
            ));
            libs.entry(library.clone())
                .or_default()
                .push(module_name.clone());
            for exp in parsed.exports {
                self.exports.insert(exp.nid.clone(), library.clone());
                total += 1;
            }
        }
        for (name, mods) in libs {
            if self.libraries.iter().any(|l| l.name == name) {
                continue;
            }
            self.libraries.push(FirmwareLibrary::new(name, "1.0", mods));
        }
        total
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Deserialize)]
struct ExportFile {
    module: String,
    exports: Vec<ExportEntry>,
}

#[derive(Debug, Deserialize)]
struct ExportEntry {
    nid: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    address: String,
    #[allow(dead_code)]
    size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty() {
        let cat = FirmwareCatalog::new(FirmwareVersion::new(10, 0, 0));
        assert!(cat.modules.is_empty());
    }

    #[test]
    fn json_roundtrip() {
        let mut cat = FirmwareCatalog::new(FirmwareVersion::new(9, 0, 0));
        cat.modules
            .push(FirmwareModule::new("libTest.prx", "/tmp", "1.0", 5));
        let s = cat.to_json().unwrap();
        let de = FirmwareCatalog::from_json(&s).unwrap();
        assert_eq!(de.modules.len(), 1);
    }

    #[test]
    fn populate_nonexistent_is_noop() {
        let mut cat = FirmwareCatalog::new(FirmwareVersion::new(10, 0, 0));
        cat.populate_from_roms(Path::new("/nonexistent/path/xyz"));
        assert!(cat.modules.is_empty());
    }
}
