use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibVersion {
    pub name: String,
    pub version: String,
    pub raw: u32,
}

pub fn extract_versions(elf: &ps5_elf::ElfImage) -> Vec<LibVersion> {
    elf.lib_versions
        .iter()
        .map(|lv| LibVersion {
            name: lv.name.clone(),
            version: lv.guessed_version_string(),
            raw: lv.version_raw,
        })
        .collect()
}
