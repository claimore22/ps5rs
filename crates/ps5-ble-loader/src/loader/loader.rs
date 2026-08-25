//! Types describing a loaded module: section/symbol/program-header summaries,
//! relocation entries and the final `LoadedElf` output, plus page-rounding
//! helpers used by ELF mapping.

const PAGE_SIZE: u64 = 0x1000;

fn page_down(value: u64) -> u64 {
    value & !(PAGE_SIZE - 1)
}

pub(super) fn page_up(value: u64) -> u64 {
    (value + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

#[derive(Debug, Clone)]
pub struct ElfSection {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub flags: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ElfSymbol {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub binding: u8,
    pub sym_type: u8,
}

#[derive(Debug, Clone)]
pub struct ProgramHeader {
    pub ptype: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone)]
pub struct DynamicSymbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    pub defined: bool,
    pub bind: u8,
    pub sym_type: u8,
}

#[derive(Debug, Clone)]
pub struct RelocationEntry {
    pub r_type: u32,
    pub r_offset: u64,
    pub r_addend: i64,
    pub symbol: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct LoadedElf {
    pub entry_point: u64,
    pub base_address: u64,
    pub size: u64,
    /// DT_INIT (module_start) virtual address relative to base_address.
    pub init_vaddr: u64,
    /// DT_FINI (module_stop) virtual address relative to base_address.
    pub fini_vaddr: u64,
    pub sections: Vec<ElfSection>,
    pub symbols: Vec<ElfSymbol>,
    pub dynamic_linking: bool,
    pub soname: String,
    pub needed_libs: Vec<String>,
    pub program_headers: Vec<ProgramHeader>,
    pub eboot_path: String,
    pub is_self: bool,
    pub dynamic_symbols: Vec<DynamicSymbol>,
    pub relocations: Vec<RelocationEntry>,
    pub rela_count: usize,
    pub jmprela_count: usize,
}
