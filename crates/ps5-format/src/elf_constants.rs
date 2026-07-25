pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

pub const EI_CLASS: usize = 4;
pub const EI_DATA: usize = 5;
pub const EI_VERSION: usize = 6;
pub const EI_OSABI: usize = 7;

pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;

pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;
pub const ET_SCE_DYNEXEC: u16 = 0xfe10;
pub const ET_SCE_DYNAMIC: u16 = 0xfe18;

pub const EM_X86_64: u16 = 0x3e;

pub const PT_NULL: u32 = 0;
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_TLS: u32 = 7;
pub const PT_GNU_EH_FRAME: u32 = 0x6474e550;
pub const PT_GNU_RELRO: u32 = 0x6474e552;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

pub const DT_NULL: u64 = 0;
pub const DT_NEEDED: u64 = 1;
pub const DT_PLTRELSZ: u64 = 2;
pub const DT_PLTGOT: u64 = 3;
pub const DT_STRTAB: u64 = 5;
pub const DT_SYMTAB: u64 = 6;
pub const DT_RELA: u64 = 7;
pub const DT_RELASZ: u64 = 8;
pub const DT_STRSZ: u64 = 0xa;
pub const DT_SYMENT: u64 = 0xb;
pub const DT_INIT: u64 = 0xc;
pub const DT_FINI: u64 = 0xd;
pub const DT_JMPREL: u64 = 0x17;
pub const DT_INIT_ARRAY: u64 = 0x19;
pub const DT_INIT_ARRAYSZ: u64 = 0x1b;
pub const DT_FINI_ARRAY: u64 = 0x1a;
pub const DT_FINI_ARRAYSZ: u64 = 0x1c;
pub const DT_PREINIT_ARRAY: u64 = 0x20;
pub const DT_PREINIT_ARRAYSZ: u64 = 0x21;

pub const R_X86_64_64: u32 = 1;
pub const R_X86_64_PC32: u32 = 2;
pub const R_X86_64_PLT32: u32 = 4;
pub const R_X86_64_GLOB_DAT: u32 = 6;
pub const R_X86_64_JUMP_SLOT: u32 = 7;
pub const R_X86_64_RELATIVE: u32 = 8;
pub const R_X86_64_DTPMOD64: u32 = 16;
pub const R_X86_64_DTPOFF64: u32 = 17;
pub const R_X86_64_TPOFF64: u32 = 18;

pub const STB_LOCAL: u8 = 0;
pub const STB_GLOBAL: u8 = 1;
pub const STB_WEAK: u8 = 2;

pub const STT_NOTYPE: u8 = 0;
pub const STT_OBJECT: u8 = 1;
pub const STT_FUNC: u8 = 2;
