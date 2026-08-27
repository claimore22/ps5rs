#![allow(clippy::collapsible_if)]
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::exports::FirmwareExportTable;
use crate::libraries::FirmwareLibrary;
use crate::modules::FirmwareModule;
use crate::version::FirmwareVersion;

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
            // simple version check: if required version <= catalog version string? For now check equality or availability
            let _ = required_version;
            let _ = &l.version;
            true
        } else {
            false
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
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
