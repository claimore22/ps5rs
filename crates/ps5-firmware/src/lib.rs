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
}
