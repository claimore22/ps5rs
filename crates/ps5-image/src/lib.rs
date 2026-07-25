use std::collections::HashMap;

mod builder;
pub use builder::BinaryImageBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Ps4,
    Ps5,
    RawElf,
    Unknown,
}

impl Platform {
    pub fn from_self(p: ps5_self::SelfPlatform) -> Self {
        match p {
            ps5_self::SelfPlatform::Ps4 => Self::Ps4,
            ps5_self::SelfPlatform::Ps5 => Self::Ps5,
            ps5_self::SelfPlatform::RawElf => Self::RawElf,
            ps5_self::SelfPlatform::Unknown(_) => Self::Unknown,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ps4 => write!(f, "PS4"),
            Self::Ps5 => write!(f, "PS5"),
            Self::RawElf => write!(f, "RawELF"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TlsInfo {
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone)]
pub struct LoadedSegment {
    pub vaddr: u64,
    pub file_offset: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub is_executable: bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub nid_hash: String,
    pub resolved_name: Option<String>,
    pub library_id: u16,
    pub library_name: String,
}

#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub nid_hash: String,
    pub resolved_name: Option<String>,
    pub vaddr: u64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct RelocationEntry {
    pub offset: u64,
    pub info: u64,
    pub addend: i64,
    pub r_type: u32,
    pub r_sym: u32,
    pub is_plt: bool,
}

#[derive(Debug, Clone)]
pub struct BinaryImage {
    pub sha256: String,
    pub platform: Platform,
    pub is_self: bool,
    pub file_size: u64,
    pub entry_point: u64,
    pub segments: Vec<LoadedSegment>,
    pub imports: Vec<ImportEntry>,
    pub exports: Vec<ExportEntry>,
    pub relocations: Vec<RelocationEntry>,
    pub tls: Option<TlsInfo>,
    pub init_va: u64,
    pub init_array_va: u64,
    pub init_array_sz: u64,
    pub fini_va: u64,
    pub fini_array_va: u64,
    pub fini_array_sz: u64,
    pub preinit_array_va: u64,
    pub preinit_array_sz: u64,
    pub import_libs: HashMap<u16, String>,
    pub needed_files: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_format::elf_constants::*;

    fn build_synthetic_elf() -> Vec<u8> {
        let strtab = {
            let mut buf = vec![0u8];
            buf.extend_from_slice(b"hello#A"); // NID "hello" from lib with ID 0 (base64 'A')
            buf.push(0);
            buf.extend_from_slice(b"libSceFoo"); // library name at offset 8
            buf.push(0);
            buf
        };

        let symtab = {
            let mut buf = Vec::new();
            // sym[0] = null symbol
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.push(0); buf.push(0);
            buf.extend_from_slice(&0u16.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            // sym[1] = import: "hello#A" (st_shndx=0, st_value=0)
            buf.extend_from_slice(&1u32.to_le_bytes()); // st_name=1
            buf.push(0); buf.push(0); // st_info, st_other
            buf.extend_from_slice(&0u16.to_le_bytes()); // st_shndx=0 (import)
            buf.extend_from_slice(&0u64.to_le_bytes()); // st_value=0
            buf.extend_from_slice(&0u64.to_le_bytes()); // st_size
            buf
        };

        // First compute layout to get correct vaddrs
        let dynamic_entries = {
            let mut entries = Vec::new();
            // We need to compute strtab_vaddr and symtab_vaddr after knowing dynamic size
            // So build a placeholder dynamic first to measure its size
            entries.push((DT_STRTAB, 0u64)); // placeholder
            entries.push((DT_STRSZ, strtab.len() as u64));
            entries.push((DT_SYMTAB, 0u64)); // placeholder
            entries.push((DT_SYMENT, 24u64));
            entries.push((0x6100003Fu64, symtab.len() as u64)); // DT_SCE_SYMTABSZ
            // DT_SCE_NEEDED_LIB: lib_id=0 (base64 'A'), name_offset=9 for "libSceFoo"
            entries.push((0x61000049u64, 9 | (0u64 << 48)));
            entries.push((0u64, 0u64)); // DT_NULL
            entries
        };

        let dyn_byte_size = dynamic_entries.len() * 16;
        let load_vaddr = 0x1000u64;
        let strtab_vaddr = load_vaddr + dyn_byte_size as u64;
        let symtab_vaddr = strtab_vaddr + strtab.len() as u64;

        // Now build the real dynamic entries with correct vaddrs
        let dynamic = {
            let mut buf = Vec::new();
            let write_u64 = |buf: &mut Vec<u8>, v: u64| { buf.extend_from_slice(&v.to_le_bytes()); };
            write_u64(&mut buf, DT_STRTAB);
            write_u64(&mut buf, strtab_vaddr);
            write_u64(&mut buf, DT_STRSZ);
            write_u64(&mut buf, strtab.len() as u64);
            write_u64(&mut buf, DT_SYMTAB);
            write_u64(&mut buf, symtab_vaddr);
            write_u64(&mut buf, DT_SYMENT);
            write_u64(&mut buf, 24);
            write_u64(&mut buf, 0x6100003F);
            write_u64(&mut buf, symtab.len() as u64);
            write_u64(&mut buf, 0x61000049);
            write_u64(&mut buf, 9 | (0u64 << 48));
            write_u64(&mut buf, 0);
            write_u64(&mut buf, 0);
            buf
        };
        assert_eq!(dynamic.len(), dyn_byte_size);

        let load_data = {
            let mut buf = Vec::new();
            buf.extend_from_slice(&dynamic);
            buf.extend_from_slice(&strtab);
            buf.extend_from_slice(&symtab);
            buf.resize(buf.len().next_multiple_of(8), 0);
            buf
        };

        let load_offset = 0x1000u64;
        let dyn_vaddr = load_vaddr;
        let dyn_filesz = dynamic.len() as u64;
        let total_size = load_offset as usize + load_data.len();
        let phdr_count = 2u16;
        let e_phoff: u64 = 64;

        let mut file = vec![0u8; total_size.max(0x2000)];

        let write_u16 = |data: &mut [u8], off: usize, v: u16| {
            data[off..off + 2].copy_from_slice(&v.to_le_bytes());
        };
        let write_u32 = |data: &mut [u8], off: usize, v: u32| {
            data[off..off + 4].copy_from_slice(&v.to_le_bytes());
        };
        let write_u64 = |data: &mut [u8], off: usize, v: u64| {
            data[off..off + 8].copy_from_slice(&v.to_le_bytes());
        };

        // ELF header
        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;
        write_u16(&mut file, 16, ET_SCE_DYNAMIC);
        write_u16(&mut file, 18, EM_X86_64);
        write_u32(&mut file, 20, 1);
        write_u64(&mut file, 24, load_vaddr);
        write_u64(&mut file, 32, e_phoff);
        write_u16(&mut file, 52, 64);
        write_u16(&mut file, 54, 56);
        write_u16(&mut file, 56, phdr_count);

        // Phdr 0: PT_LOAD
        let off0 = e_phoff as usize;
        write_u32(&mut file, off0, PT_LOAD);
        write_u32(&mut file, off0 + 4, PF_R | PF_X);
        write_u64(&mut file, off0 + 8, load_offset);
        write_u64(&mut file, off0 + 16, load_vaddr);
        write_u64(&mut file, off0 + 24, 0);
        write_u64(&mut file, off0 + 32, load_data.len() as u64);
        write_u64(&mut file, off0 + 40, load_data.len() as u64);
        write_u64(&mut file, off0 + 48, 0x1000);

        // Phdr 1: PT_DYNAMIC
        let off1 = off0 + 56;
        write_u32(&mut file, off1, PT_DYNAMIC);
        write_u32(&mut file, off1 + 4, PF_R | PF_W);
        write_u64(&mut file, off1 + 8, dyn_vaddr);
        write_u64(&mut file, off1 + 16, dyn_vaddr);
        write_u64(&mut file, off1 + 24, 0);
        write_u64(&mut file, off1 + 32, dyn_filesz);
        write_u64(&mut file, off1 + 40, dyn_filesz);
        write_u64(&mut file, off1 + 48, 8);

        // Load data
        let data_start = load_offset as usize;
        file[data_start..data_start + load_data.len()].copy_from_slice(&load_data);

        file
    }

    #[test]
    fn platform_display() {
        assert_eq!(format!("{}", Platform::Ps5), "PS5");
        assert_eq!(format!("{}", Platform::Ps4), "PS4");
        assert_eq!(format!("{}", Platform::RawElf), "RawELF");
        assert_eq!(format!("{}", Platform::Unknown), "Unknown");
    }

    #[test]
    fn platform_from_self() {
        assert_eq!(Platform::from_self(ps5_self::SelfPlatform::Ps5), Platform::Ps5);
        assert_eq!(Platform::from_self(ps5_self::SelfPlatform::Ps4), Platform::Ps4);
        assert_eq!(Platform::from_self(ps5_self::SelfPlatform::RawElf), Platform::RawElf);
        assert_eq!(Platform::from_self(ps5_self::SelfPlatform::Unknown(99)), Platform::Unknown);
    }

    #[test]
    fn builder_produces_valid_image() {
        let catalog = ps5_nid::Catalog::new();
        let img = BinaryImageBuilder::build_from_file(
            b"not a real file".to_vec(),
            "0000000000000000000000000000000000000000000000000000000000000000",
            &catalog,
        );
        assert_eq!(img.segments.len(), 0);
        assert_eq!(img.imports.len(), 0);
        assert_eq!(img.relocations.len(), 0);
    }

    #[test]
    fn build_test_elf_to_binary_image() {
        let elf_data = build_synthetic_elf();
        let catalog = ps5_nid::Catalog::new();

        let sha256 = ps5_format::sha256_hex(&elf_data);
        let img = BinaryImageBuilder::build_from_file(elf_data, &sha256, &catalog);

        assert_eq!(img.platform, Platform::RawElf);
        assert!(!img.is_self);
        assert_eq!(img.entry_point, 0x1000);
        assert!(img.segments.len() >= 2);

        let dyn_seg = img.segments.iter().find(|s| {
            s.vaddr == 0x1000 && s.filesz < 200
        });
        assert!(dyn_seg.is_some(), "should have DYNAMIC segment");

        assert_eq!(img.imports.len(), 1);
        assert_eq!(img.imports[0].nid_hash, "hello");
        assert_eq!(img.imports[0].library_name, "libSceFoo");
        assert_eq!(img.imports[0].library_id, 0);
    }

    #[test]
    fn build_elf_no_dynamic_yields_empty() {
        let mut file = vec![0u8; 0x2000];
        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;

        let write_u16 = |data: &mut [u8], off: usize, v: u16| {
            data[off..off + 2].copy_from_slice(&v.to_le_bytes());
        };
        let write_u32 = |data: &mut [u8], off: usize, v: u32| {
            data[off..off + 4].copy_from_slice(&v.to_le_bytes());
        };
        let write_u64 = |data: &mut [u8], off: usize, v: u64| {
            data[off..off + 8].copy_from_slice(&v.to_le_bytes());
        };

        write_u16(&mut file, 16, ET_SCE_DYNAMIC);
        write_u16(&mut file, 18, EM_X86_64);
        write_u64(&mut file, 24, 0x1000);
        write_u64(&mut file, 32, 64);
        write_u16(&mut file, 52, 64);
        write_u16(&mut file, 54, 56);
        write_u16(&mut file, 56, 1);

        let off0 = 64usize;
        write_u32(&mut file, off0, PT_LOAD);
        write_u32(&mut file, off0 + 4, PF_R | PF_X);
        write_u64(&mut file, off0 + 8, 0x1000);
        write_u64(&mut file, off0 + 16, 0x1000);
        write_u64(&mut file, off0 + 32, 0x100);
        write_u64(&mut file, off0 + 40, 0x100);
        write_u64(&mut file, off0 + 48, 0x1000);
        file[0x1000..0x1100].fill(0xCC);

        let catalog = ps5_nid::Catalog::new();
        let sha256 = ps5_format::sha256_hex(&file);
        let img = BinaryImageBuilder::build_from_file(file, &sha256, &catalog);

        assert_eq!(img.platform, Platform::RawElf);
        assert!(img.imports.is_empty());
        assert!(img.exports.is_empty());
        assert!(img.relocations.is_empty());
        assert!(img.tls.is_none());
    }
}
