pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

pub const EI_CLASS: usize = 4;
pub const EI_DATA: usize = 5;
pub const EI_VERSION: usize = 6;
pub const EI_OSABI: usize = 7;
pub const EI_ABIVERSION: usize = 8;

pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;

pub const ELFOSABI_NONE: u8 = 0;
pub const ELFOSABI_SYSV: u8 = 0;
pub const ELFOSABI_HPUX: u8 = 1;
pub const ELFOSABI_NETBSD: u8 = 2;
pub const ELFOSABI_LINUX: u8 = 3;
pub const ELFOSABI_FREEBSD: u8 = 9;
pub const ELFOSABI_OPENBSD: u8 = 12;

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

pub const PT_SCE_DYNLIBDATA: u32 = 0x61000000;
pub const PT_SCE_PROCPARAM: u32 = 0x61000001;
pub const PT_SCE_COMMENT: u32 = 0x61000002;
pub const PT_SCE_LIBVERSION: u32 = 0x61000003;
pub const PT_SCE_RELRO: u32 = 0x61000010;
pub const PT_SCE_RELA: u32 = 0x60000000;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

pub const DT_NULL: u64 = 0;
pub const DT_NEEDED: u64 = 1;
pub const DT_SONAME: u64 = 0xe;
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

pub const STV_DEFAULT: u8 = 0;
pub const STV_INTERNAL: u8 = 1;
pub const STV_HIDDEN: u8 = 2;
pub const STV_PROTECTED: u8 = 3;

pub const STT_NOTYPE: u8 = 0;
pub const STT_OBJECT: u8 = 1;
pub const STT_FUNC: u8 = 2;
pub const STT_SECTION: u8 = 3;
pub const STT_FILE: u8 = 4;

pub const SHT_NULL: u32 = 0;
pub const SHT_PROGBITS: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;
pub const SHT_STRTAB: u32 = 3;
pub const SHT_RELA: u32 = 4;
pub const SHT_HASH: u32 = 5;
pub const SHT_DYNAMIC: u32 = 6;
pub const SHT_NOTE: u32 = 7;
pub const SHT_NOBITS: u32 = 8;
pub const SHT_REL: u32 = 9;
pub const SHT_DYNSYM: u32 = 11;

pub const SHF_WRITE: u64 = 0x1;
pub const SHF_ALLOC: u64 = 0x2;
pub const SHF_EXECINSTR: u64 = 0x4;
pub const SHF_MERGE: u64 = 0x10;
pub const SHF_STRINGS: u64 = 0x20;
pub const SHF_INFO_LINK: u64 = 0x40;
pub const SHF_LINK_ORDER: u64 = 0x80;

pub const DT_VERSYM: u64 = 0x6FFFFFF0;
pub const DT_RELACOUNT: u64 = 0x6FFFFFF9;
pub const DT_VERNEED: u64 = 0x6FFFFFFE;
pub const DT_VERNEEDNUM: u64 = 0x6FFFFFFF;

pub const NT_GNU_BUILD_ID: u32 = 3;
