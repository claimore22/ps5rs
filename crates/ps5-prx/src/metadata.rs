use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrxMetadata {
    pub soname: Option<String>,
    pub needed_files: Vec<String>,
    pub import_libs: Vec<String>,
    pub build_id: Option<String>,
    pub elf_type: u16,
    pub entry_point: u64,
}

impl PrxMetadata {
    pub fn from_elf(elf: &ps5_elf::ElfImage) -> Self {
        Self {
            soname: elf.soname.clone(),
            needed_files: elf.needed_files.clone(),
            import_libs: elf.import_libs.values().cloned().collect(),
            build_id: ps5_elf::section::find_build_id(elf.data, &elf.section_headers),
            elf_type: elf.header.e_type,
            entry_point: elf.header.e_entry,
        }
    }
}
