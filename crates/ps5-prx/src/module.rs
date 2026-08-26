use serde::{Deserialize, Serialize};

use crate::dependencies::{Dependency, extract_dependencies};
use crate::error::PrxError;
use crate::exports::{ExportEntry, extract_exports};
use crate::imports::{ImportEntry, extract_imports};
use crate::metadata::PrxMetadata;
use crate::versions::{LibVersion, extract_versions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    Eboot,
    Prx,
    Sprx,
    SelfModule,
    Unknown,
}

impl ModuleType {
    pub fn from_elf_type(elf_type: u16) -> Self {
        match elf_type {
            0xFE00 => Self::Eboot,
            0xFE01 => Self::Prx,
            0xFE04 => Self::Sprx,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrxModule {
    pub name: String,
    pub module_type: ModuleType,
    pub metadata: PrxMetadata,
    pub dependencies: Vec<Dependency>,
    pub imports: Vec<ImportEntry>,
    pub exports: Vec<ExportEntry>,
    pub versions: Vec<LibVersion>,
}

impl PrxModule {
    pub fn from_elf_bytes(
        name: &str,
        data: &[u8],
        catalog: &ps5_nid::Catalog,
    ) -> Result<Self, PrxError> {
        let self_img =
            ps5_self::SelfImage::parse(data).map_err(|e| PrxError::SelfError(e.to_string()))?;
        Self::from_elf(name, &self_img.elf, catalog)
    }

    pub fn from_elf(
        name: &str,
        elf: &ps5_elf::ElfImage,
        catalog: &ps5_nid::Catalog,
    ) -> Result<Self, PrxError> {
        let metadata = PrxMetadata::from_elf(elf);
        let module_type = ModuleType::from_elf_type(elf.header.e_type);
        let dependencies = extract_dependencies(elf, name);
        let imports = extract_imports(elf, catalog);
        let exports = extract_exports(elf, catalog);
        let versions = extract_versions(elf);
        Ok(Self {
            name: name.to_string(),
            module_type,
            metadata,
            dependencies,
            imports,
            exports,
            versions,
        })
    }

    pub fn is_system(&self) -> bool {
        self.name.starts_with("libSce") || self.name.starts_with("libkernel")
    }

    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    pub fn export_count(&self) -> usize {
        self.exports.len()
    }
}
