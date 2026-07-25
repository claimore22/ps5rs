use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod builder;
pub mod json;

pub use builder::BinaryImageBuilder;

pub const BINARY_IMAGE_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// JSON document wrapper — versions the interchange format, not the Rust struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImageDocument {
    pub schema_version: u32,
    pub tool: String,
    pub image: BinaryImage,
}

// ---------------------------------------------------------------------------
// Hex serde helpers — addresses and offsets serialize as "0x..." strings
// ---------------------------------------------------------------------------

pub(crate) mod hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:#x}", v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        parse_u64(&s).map_err(serde::de::Error::custom)
    }

    pub(crate) fn parse_u64(s: &str) -> Result<u64, String> {
        if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u64::from_str_radix(rest, 16).map_err(|e| format!("{e}"))
        } else {
            s.parse::<u64>().map_err(|e| format!("{e}"))
        }
    }
}

pub(crate) mod hex_signed {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
        if *v < 0 {
            s.serialize_str(&format!("-{:#x}", v.wrapping_neg()))
        } else {
            s.serialize_str(&format!("{:#x}", v))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        let s = String::deserialize(d)?;
        let (neg, rest) = if let Some(r) = s.strip_prefix('-') {
            (true, r)
        } else {
            (false, s.as_str())
        };
        let abs = if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
            u64::from_str_radix(hex, 16).map_err(serde::de::Error::custom)?
        } else {
            rest.parse::<u64>().map_err(serde::de::Error::custom)?
        };
        if neg {
            Ok(-(abs as i64))
        } else {
            Ok(abs as i64)
        }
    }
}

// ---------------------------------------------------------------------------
// Platform
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Ps4,
    Ps5,
    #[serde(rename = "RawELF")]
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

// ---------------------------------------------------------------------------
// TlsInfo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    #[serde(with = "hex")]
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

// ---------------------------------------------------------------------------
// LoadedSegment — custom Serialize/Deserialize for flags as ELF-style string
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoadedSegment {
    pub vaddr: u64,
    pub file_offset: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub is_executable: bool,
    pub is_writable: bool,
}

impl LoadedSegment {
    pub fn flags(&self) -> String {
        let mut s = String::with_capacity(3);
        s.push('R');
        if self.is_writable {
            s.push('W');
        }
        if self.is_executable {
            s.push('X');
        }
        s
    }
}

impl Serialize for LoadedSegment {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("LoadedSegment", 6)?;
        state.serialize_field("vaddr", &format!("{:#x}", self.vaddr))?;
        state.serialize_field("file_offset", &format!("{:#x}", self.file_offset))?;
        state.serialize_field("filesz", &self.filesz)?;
        state.serialize_field("memsz", &self.memsz)?;
        state.serialize_field("flags", &self.flags())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LoadedSegment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            vaddr: String,
            file_offset: String,
            filesz: u64,
            memsz: u64,
            flags: String,
        }

        let h = Helper::deserialize(deserializer)?;

        let vaddr = hex::parse_u64(&h.vaddr).map_err(serde::de::Error::custom)?;
        let file_offset = hex::parse_u64(&h.file_offset).map_err(serde::de::Error::custom)?;

        let mut is_writable = false;
        let mut is_executable = false;
        for ch in h.flags.chars() {
            match ch {
                'W' => is_writable = true,
                'X' => is_executable = true,
                _ => {}
            }
        }

        Ok(LoadedSegment {
            vaddr,
            file_offset,
            filesz: h.filesz,
            memsz: h.memsz,
            is_executable,
            is_writable,
        })
    }
}

// ---------------------------------------------------------------------------
// ImportEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    pub nid_hash: String,
    pub resolved_name: Option<String>,
    pub library_id: u16,
    pub library_name: String,
}

// ---------------------------------------------------------------------------
// ExportEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub nid_hash: String,
    pub resolved_name: Option<String>,
    #[serde(with = "hex")]
    pub vaddr: u64,
    pub size: u64,
}

// ---------------------------------------------------------------------------
// RelocationEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelocationEntry {
    #[serde(with = "hex")]
    pub offset: u64,
    pub info: u64,
    #[serde(with = "hex_signed")]
    pub addend: i64,
    pub r_type: u32,
    pub r_sym: u32,
    pub is_plt: bool,
}

// ---------------------------------------------------------------------------
// BinaryImage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImage {
    pub sha256: String,
    pub platform: Platform,
    pub is_self: bool,
    pub file_size: u64,
    #[serde(with = "hex")]
    pub entry_point: u64,
    pub segments: Vec<LoadedSegment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ImportEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<ExportEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relocations: Vec<RelocationEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsInfo>,
    #[serde(with = "hex")]
    pub init_va: u64,
    #[serde(with = "hex")]
    pub init_array_va: u64,
    pub init_array_sz: u64,
    #[serde(with = "hex")]
    pub fini_va: u64,
    #[serde(with = "hex")]
    pub fini_array_va: u64,
    pub fini_array_sz: u64,
    #[serde(with = "hex")]
    pub preinit_array_va: u64,
    pub preinit_array_sz: u64,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub import_libs: HashMap<u16, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needed_files: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_format::elf_constants::*;

    fn build_synthetic_elf() -> Vec<u8> {
        let strtab = {
            let mut buf = vec![0u8];
            buf.extend_from_slice(b"hello#A");
            buf.push(0);
            buf.extend_from_slice(b"libSceFoo");
            buf.push(0);
            buf
        };

        let symtab = {
            let mut buf = Vec::new();
            buf.extend_from_slice(&0u32.to_le_bytes());
            buf.push(0); buf.push(0);
            buf.extend_from_slice(&0u16.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.push(0); buf.push(0);
            buf.extend_from_slice(&0u16.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf
        };

        let dynamic_entries = {
            let mut entries = Vec::new();
            entries.push((DT_STRTAB, 0u64));
            entries.push((DT_STRSZ, strtab.len() as u64));
            entries.push((DT_SYMTAB, 0u64));
            entries.push((DT_SYMENT, 24u64));
            entries.push((0x6100003Fu64, symtab.len() as u64));
            entries.push((0x61000049u64, 9 | (0u64 << 48)));
            entries.push((0u64, 0u64));
            entries
        };

        let dyn_byte_size = dynamic_entries.len() * 16;
        let load_vaddr = 0x1000u64;
        let strtab_vaddr = load_vaddr + dyn_byte_size as u64;
        let symtab_vaddr = strtab_vaddr + strtab.len() as u64;

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

        let off0 = e_phoff as usize;
        write_u32(&mut file, off0, PT_LOAD);
        write_u32(&mut file, off0 + 4, PF_R | PF_X);
        write_u64(&mut file, off0 + 8, load_offset);
        write_u64(&mut file, off0 + 16, load_vaddr);
        write_u64(&mut file, off0 + 24, 0);
        write_u64(&mut file, off0 + 32, load_data.len() as u64);
        write_u64(&mut file, off0 + 40, load_data.len() as u64);
        write_u64(&mut file, off0 + 48, 0x1000);

        let off1 = off0 + 56;
        write_u32(&mut file, off1, PT_DYNAMIC);
        write_u32(&mut file, off1 + 4, PF_R | PF_W);
        write_u64(&mut file, off1 + 8, dyn_vaddr);
        write_u64(&mut file, off1 + 16, dyn_vaddr);
        write_u64(&mut file, off1 + 24, 0);
        write_u64(&mut file, off1 + 32, dyn_filesz);
        write_u64(&mut file, off1 + 40, dyn_filesz);
        write_u64(&mut file, off1 + 48, 8);

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

    #[test]
    fn loaded_segment_roundtrip() {
        let seg = LoadedSegment {
            vaddr: 0x100000,
            file_offset: 0x1000,
            filesz: 4096,
            memsz: 8192,
            is_executable: true,
            is_writable: false,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"flags\":\"RX\""));
        let back: LoadedSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.vaddr, 0x100000);
        assert!(back.is_executable);
        assert!(!back.is_writable);
    }

    #[test]
    fn loaded_segment_flags_rwx() {
        let seg = LoadedSegment {
            vaddr: 0,
            file_offset: 0,
            filesz: 0,
            memsz: 0,
            is_executable: true,
            is_writable: true,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"flags\":\"RWX\""));
    }

    #[test]
    fn loaded_segment_flags_r_only() {
        let seg = LoadedSegment {
            vaddr: 0,
            file_offset: 0,
            filesz: 100,
            memsz: 100,
            is_executable: false,
            is_writable: false,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"flags\":\"R\""));
    }
}
