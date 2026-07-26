pub const SELF_MAGIC_PS4: u32 = 0x4F153D1D;
pub const SELF_MAGIC_PS5: u32 = 0x5414F5EE;

pub const SELF_SEGMENT_FLAG_ENCRYPTED: u64 = 0x2;
pub const SELF_SEGMENT_FLAG_COMPRESSED: u64 = 0x8;
pub const SELF_SEGMENT_FLAG_DATA: u64 = 0x800;

pub const DT_SCE_JMPREL: u64 = 0x61000029;
pub const DT_SCE_PLTRELSZ: u64 = 0x6100002D;
pub const DT_SCE_RELA: u64 = 0x6100002F;
pub const DT_SCE_RELASZ: u64 = 0x61000031;
pub const DT_SCE_STRTAB: u64 = 0x61000035;
pub const DT_SCE_STRSZ: u64 = 0x61000037;
pub const DT_SCE_SYMTAB: u64 = 0x61000039;
pub const DT_SCE_SYMTABSZ: u64 = 0x6100003F;
pub const DT_SCE_NEEDED_LIB: u64 = 0x61000049;
pub const DT_SCE_NEEDED_MOD: u64 = 0x61000045;

pub const PS5_IMAGE_BASE: u64 = 0x0000_0008_0000_0000;
pub const PS4_IMAGE_BASE: u64 = 0x0000_0000_0040_0000;

pub fn self_segment_key(flags: u64) -> u64 {
    (flags >> 20) & 0xFFF
}
