use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareModule {
    pub name: String,
    pub path: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareLibrary {
    pub name: String,
    pub version: String,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareCatalog {
    pub version: FirmwareVersion,
    pub modules: Vec<FirmwareModule>,
    pub libraries: Vec<FirmwareLibrary>,
    pub exports: HashMap<String, Vec<String>>,
}

impl FirmwareCatalog {
    pub fn new(version: FirmwareVersion) -> Self {
        Self {
            version,
            modules: Vec::new(),
            libraries: Vec::new(),
            exports: HashMap::new(),
        }
    }

    pub fn populate_from_roms(&mut self, roms_path: &str) {
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten().take(2) {
                let p = entry.path().join("sce_module");
                if p.exists() {
                    if let Ok(mods) = std::fs::read_dir(&p) {
                        for m in mods.flatten().take(2) {
                            let name = m.file_name().to_string_lossy().to_string();
                            self.modules.push(FirmwareModule {
                                name: name.clone(),
                                path: m.path().to_string_lossy().to_string(),
                                version: "1.0".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
}
