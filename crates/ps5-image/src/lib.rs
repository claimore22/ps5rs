use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod builder;
pub mod json;

pub use builder::BinaryImageBuilder;

pub const BINARY_IMAGE_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Detection — value + evidence for reverse engineering datasets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Detection {
    pub value: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub confidence: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

// ---------------------------------------------------------------------------
// StringAnalysis — string-based fingerprints extracted from raw binary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StringAnalysis {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sce_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_libs: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_system: Option<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depot: Option<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk_hints: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_versions: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_paths: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_forks: Vec<Detection>,
}

// ---------------------------------------------------------------------------
// Image type — distinguishes eboot from PRX / SPRX / other modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageType {
    Eboot,
    Prx,
    Sprx,
    SelfModule,
    Unknown,
}

impl Default for ImageType {
    fn default() -> Self {
        Self::Eboot
    }
}

// ---------------------------------------------------------------------------
// JSON document wrapper — versions the interchange format, not the Rust struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImageDocument {
    pub schema_version: u32,
    pub tool: String,
    pub image: BinaryImage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_analysis: Option<StringAnalysis>,
    #[serde(default)]
    pub image_type: ImageType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_image: Option<String>,
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

pub(crate) mod hex_u32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u32, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:#x}", v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
        let s = String::deserialize(d)?;
        if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            u32::from_str_radix(rest, 16).map_err(serde::de::Error::custom)
        } else {
            s.parse::<u32>().map_err(serde::de::Error::custom)
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
// SegmentType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(non_camel_case_types)]
pub enum SegmentType {
    Null,
    Load,
    Dynamic,
    Tls,
    GNU_EhFrame,
    GNU_Relro,
    SCE_Dynlibdata,
    SCE_Procparam,
    SCE_Comment,
    SCE_Libversion,
    SCE_Relro,
    SCE_Rela,
    Note,
    Phdr,
    Interp,
    Other(u32),
}

impl SegmentType {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::Null,
            1 => Self::Load,
            2 => Self::Dynamic,
            7 => Self::Tls,
            4 => Self::Note,
            6 => Self::Phdr,
            3 => Self::Interp,
            0x6474e550 => Self::GNU_EhFrame,
            0x6474e552 => Self::GNU_Relro,
            0x61000000 => Self::SCE_Dynlibdata,
            0x61000001 => Self::SCE_Procparam,
            0x61000002 => Self::SCE_Comment,
            0x61000003 => Self::SCE_Libversion,
            0x61000010 => Self::SCE_Relro,
            0x60000000 => Self::SCE_Rela,
            other => Self::Other(other),
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            Self::Null => 0,
            Self::Load => 1,
            Self::Dynamic => 2,
            Self::Tls => 7,
            Self::Note => 4,
            Self::Phdr => 6,
            Self::Interp => 3,
            Self::GNU_EhFrame => 0x6474e550,
            Self::GNU_Relro => 0x6474e552,
            Self::SCE_Dynlibdata => 0x61000000,
            Self::SCE_Procparam => 0x61000001,
            Self::SCE_Comment => 0x61000002,
            Self::SCE_Libversion => 0x61000003,
            Self::SCE_Relro => 0x61000010,
            Self::SCE_Rela => 0x60000000,
            Self::Other(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// SymbolBinding / SymbolType / SymbolVisibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
    Other(u8),
}

impl SymbolBinding {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Local,
            1 => Self::Global,
            2 => Self::Weak,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolType {
    Notype,
    Object,
    Func,
    Section,
    File,
    Other(u8),
}

impl SymbolType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Notype,
            1 => Self::Object,
            2 => Self::Func,
            3 => Self::Section,
            4 => Self::File,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolVisibility {
    Default,
    Internal,
    Hidden,
    Protected,
    Other(u8),
}

impl SymbolVisibility {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Default,
            1 => Self::Internal,
            2 => Self::Hidden,
            3 => Self::Protected,
            other => Self::Other(other),
        }
    }
}

// ---------------------------------------------------------------------------
// RelocationKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum RelocationKind {
    None,
    _64,
    PC32,
    GOT32,
    PLT32,
    Copy,
    GlobDat,
    JumpSlot,
    Relative,
    Direct32,
    Direct32S,
    Direct16,
    PC16,
    Direct8,
    PC8,
    TPOff64,
    TPOff32,
    DTPMod64,
    DTPOff64,
    TLSDESC,
    TlsModOff,
    TlsOffset,
    Other(u32),
}

impl Serialize for RelocationKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::None => serializer.serialize_str("none"),
            Self::_64 => serializer.serialize_str("_64"),
            Self::PC32 => serializer.serialize_str("pc32"),
            Self::GOT32 => serializer.serialize_str("got32"),
            Self::PLT32 => serializer.serialize_str("plt32"),
            Self::Copy => serializer.serialize_str("copy"),
            Self::GlobDat => serializer.serialize_str("glob_dat"),
            Self::JumpSlot => serializer.serialize_str("jump_slot"),
            Self::Relative => serializer.serialize_str("relative"),
            Self::Direct32 => serializer.serialize_str("direct32"),
            Self::Direct32S => serializer.serialize_str("direct32s"),
            Self::Direct16 => serializer.serialize_str("direct16"),
            Self::PC16 => serializer.serialize_str("pc16"),
            Self::Direct8 => serializer.serialize_str("direct8"),
            Self::PC8 => serializer.serialize_str("pc8"),
            Self::TPOff64 => serializer.serialize_str("tpoff64"),
            Self::TPOff32 => serializer.serialize_str("tpoff32"),
            Self::DTPMod64 => serializer.serialize_str("dtpmod64"),
            Self::DTPOff64 => serializer.serialize_str("dtpoff64"),
            Self::TLSDESC => serializer.serialize_str("tlsdesc"),
            Self::TlsModOff => serializer.serialize_str("tls_mod_off"),
            Self::TlsOffset => serializer.serialize_str("tls_offset"),
            Self::Other(v) => serializer.serialize_u32(*v),
        }
    }
}

impl<'de> Deserialize<'de> for RelocationKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RelocationKindVisitor;

        impl<'de> serde::de::Visitor<'de> for RelocationKindVisitor {
            type Value = RelocationKind;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string relocation name or integer relocation type")
            }

            fn visit_u64<E>(self, v: u64) -> Result<RelocationKind, E>
            where
                E: serde::de::Error,
            {
                Ok(RelocationKind::from_u32(v as u32))
            }

            fn visit_u32<E>(self, v: u32) -> Result<RelocationKind, E>
            where
                E: serde::de::Error,
            {
                Ok(RelocationKind::from_u32(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<RelocationKind, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "none" => Ok(RelocationKind::None),
                    "_64" => Ok(RelocationKind::_64),
                    "pc32" => Ok(RelocationKind::PC32),
                    "got32" => Ok(RelocationKind::GOT32),
                    "plt32" => Ok(RelocationKind::PLT32),
                    "copy" => Ok(RelocationKind::Copy),
                    "glob_dat" => Ok(RelocationKind::GlobDat),
                    "jump_slot" => Ok(RelocationKind::JumpSlot),
                    "relative" => Ok(RelocationKind::Relative),
                    "direct32" => Ok(RelocationKind::Direct32),
                    "direct32s" => Ok(RelocationKind::Direct32S),
                    "direct16" => Ok(RelocationKind::Direct16),
                    "pc16" => Ok(RelocationKind::PC16),
                    "direct8" => Ok(RelocationKind::Direct8),
                    "pc8" => Ok(RelocationKind::PC8),
                    "tpoff64" => Ok(RelocationKind::TPOff64),
                    "tpoff32" => Ok(RelocationKind::TPOff32),
                    "dtpmod64" => Ok(RelocationKind::DTPMod64),
                    "dtpoff64" => Ok(RelocationKind::DTPOff64),
                    "tlsdesc" => Ok(RelocationKind::TLSDESC),
                    "tls_mod_off" => Ok(RelocationKind::TlsModOff),
                    "tls_offset" => Ok(RelocationKind::TlsOffset),
                    _ => Ok(RelocationKind::Other(0)),
                }
            }
        }

        deserializer.deserialize_any(RelocationKindVisitor)
    }
}

impl RelocationKind {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::None,
            1 => Self::_64,
            2 => Self::PC32,
            3 => Self::GOT32,
            4 => Self::PLT32,
            5 => Self::Copy,
            6 => Self::GlobDat,
            7 => Self::JumpSlot,
            8 => Self::Relative,
            10 => Self::Direct32,
            11 => Self::Direct32S,
            12 => Self::Direct16,
            13 => Self::PC16,
            14 => Self::Direct8,
            15 => Self::PC8,
            16 => Self::DTPMod64,
            17 => Self::DTPOff64,
            18 => Self::TPOff64,
            23 => Self::TPOff32,
            36 => Self::TLSDESC,
            42 => Self::TlsModOff,
            43 => Self::TlsOffset,
            other => Self::Other(other),
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            Self::None => 0,
            Self::_64 => 1,
            Self::PC32 => 2,
            Self::GOT32 => 3,
            Self::PLT32 => 4,
            Self::Copy => 5,
            Self::GlobDat => 6,
            Self::JumpSlot => 7,
            Self::Relative => 8,
            Self::Direct32 => 10,
            Self::Direct32S => 11,
            Self::Direct16 => 12,
            Self::PC16 => 13,
            Self::Direct8 => 14,
            Self::PC8 => 15,
            Self::DTPMod64 => 16,
            Self::DTPOff64 => 17,
            Self::TPOff64 => 18,
            Self::TPOff32 => 23,
            Self::TLSDESC => 36,
            Self::TlsModOff => 42,
            Self::TlsOffset => 43,
            Self::Other(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// SectionHeader
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionHeader {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(with = "hex_u32")]
    pub sh_type: u32,
    #[serde(with = "hex")]
    pub sh_addr: u64,
    #[serde(with = "hex")]
    pub sh_offset: u64,
    #[serde(with = "hex")]
    pub sh_size: u64,
    pub sh_flags: u64,
    pub sh_flags_str: String,
    pub sh_info: u32,
    pub sh_link: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}

impl SectionHeader {
    pub fn flags_string(flags: u64) -> String {
        let mut s = String::with_capacity(3);
        if flags & 0x2 != 0 {
            s.push('A');
        } // SHF_ALLOC
        if flags & 0x1 != 0 {
            s.push('W');
        } // SHF_WRITE
        if flags & 0x4 != 0 {
            s.push('X');
        } // SHF_EXECINSTR
        if flags & 0x10 != 0 {
            s.push('M');
        } // SHF_MERGE
        if flags & 0x20 != 0 {
            s.push('S');
        } // SHF_STRINGS
        if s.is_empty() {
            s.push(' ');
        }
        s
    }
}

// ---------------------------------------------------------------------------
// DynamicEntry — tag serialized as hex
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicEntry {
    #[serde(with = "hex")]
    pub tag: u64,
    #[serde(with = "hex")]
    pub value: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_tag: Option<String>,
}

impl DynamicEntry {
    pub fn tag_name(tag: u64) -> Option<&'static str> {
        match tag {
            0 => Some("DT_NULL"),
            1 => Some("DT_NEEDED"),
            2 => Some("DT_PLTRELSZ"),
            3 => Some("DT_PLTGOT"),
            5 => Some("DT_STRTAB"),
            6 => Some("DT_SYMTAB"),
            7 => Some("DT_RELA"),
            8 => Some("DT_RELASZ"),
            0xa => Some("DT_STRSZ"),
            0xb => Some("DT_SYMENT"),
            0xc => Some("DT_INIT"),
            0xd => Some("DT_FINI"),
            0x17 => Some("DT_JMPREL"),
            0x19 => Some("DT_INIT_ARRAY"),
            0x1a => Some("DT_FINI_ARRAY"),
            0x1b => Some("DT_INIT_ARRAYSZ"),
            0x1c => Some("DT_FINI_ARRAYSZ"),
            0x20 => Some("DT_PREINIT_ARRAY"),
            0x21 => Some("DT_PREINIT_ARRAYSZ"),
            0x61000029 => Some("DT_SCE_JMPREL"),
            0x6100002d => Some("DT_SCE_PLTRELSZ"),
            0x6100002f => Some("DT_SCE_RELA"),
            0x61000031 => Some("DT_SCE_RELASZ"),
            0x61000035 => Some("DT_SCE_STRTAB"),
            0x61000037 => Some("DT_SCE_STRSZ"),
            0x61000039 => Some("DT_SCE_SYMTAB"),
            0x6100003f => Some("DT_SCE_SYMTABSZ"),
            0x61000045 => Some("DT_SCE_NEEDED_MOD"),
            0x61000049 => Some("DT_SCE_NEEDED_LIB"),
            0x6fffffff0 => Some("DT_VERSYM"),
            0x6fffffffe => Some("DT_VERNEED"),
            0x6fffffff => Some("DT_VERNEEDNUM"),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// VersionDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub hash: u32,
    pub flags: u16,
    pub index: u16,
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
// LibVersionEntry — from PT_SCE_LIBVERSION segment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibVersionEntry {
    pub name: String,
    #[serde(with = "hex_u32")]
    pub version_raw: u32,
    pub version_string: String,
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
    pub seg_type: SegmentType,
    pub p_paddr: u64,
    pub p_align: u64,
    pub is_encrypted: bool,
    pub is_compressed: bool,
    pub phdr_index: Option<u16>,
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

        let mut state = serializer.serialize_struct("LoadedSegment", 11)?;
        state.serialize_field("vaddr", &format!("{:#x}", self.vaddr))?;
        state.serialize_field("file_offset", &format!("{:#x}", self.file_offset))?;
        state.serialize_field("filesz", &self.filesz)?;
        state.serialize_field("memsz", &self.memsz)?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("seg_type", &self.seg_type)?;
        if self.p_paddr != 0 {
            state.serialize_field("p_paddr", &format!("{:#x}", self.p_paddr))?;
        }
        state.serialize_field("p_align", &self.p_align)?;
        if self.is_encrypted {
            state.serialize_field("is_encrypted", &true)?;
        }
        if self.is_compressed {
            state.serialize_field("is_compressed", &true)?;
        }
        if let Some(idx) = self.phdr_index {
            state.serialize_field("phdr_index", &idx)?;
        }
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
            #[serde(default)]
            seg_type: Option<SegmentType>,
            #[serde(default)]
            p_paddr: Option<String>,
            #[serde(default)]
            p_align: Option<u64>,
            #[serde(default)]
            is_encrypted: Option<bool>,
            #[serde(default)]
            is_compressed: Option<bool>,
            #[serde(default)]
            phdr_index: Option<u16>,
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

        let p_paddr = h
            .p_paddr
            .map(|s| hex::parse_u64(&s).map_err(serde::de::Error::custom))
            .transpose()?
            .unwrap_or(0);

        Ok(LoadedSegment {
            vaddr,
            file_offset,
            filesz: h.filesz,
            memsz: h.memsz,
            is_executable,
            is_writable,
            seg_type: h.seg_type.unwrap_or(SegmentType::Load),
            p_paddr,
            p_align: h.p_align.unwrap_or(0),
            is_encrypted: h.is_encrypted.unwrap_or(false),
            is_compressed: h.is_compressed.unwrap_or(false),
            phdr_index: h.phdr_index,
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
    #[serde(default)]
    pub value: u64,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub shndx: u16,
    #[serde(default = "default_binding")]
    pub binding: SymbolBinding,
    #[serde(default = "default_sym_type")]
    pub sym_type: SymbolType,
    #[serde(default = "default_visibility")]
    pub visibility: SymbolVisibility,
    #[serde(default)]
    pub ordinal: u32,
}

fn default_binding() -> SymbolBinding {
    SymbolBinding::Global
}

fn default_sym_type() -> SymbolType {
    SymbolType::Func
}

fn default_visibility() -> SymbolVisibility {
    SymbolVisibility::Default
}

fn is_default_metadata(m: &BinaryMetadata) -> bool {
    m.build_id.is_none()
        && m.elf_type == 0
        && m.elf_flags == 0
        && m.osabi == 0
        && m.self_key_type.is_none()
        && m.self_attr.is_none()
        && m.self_mode.is_none()
        && m.self_endian.is_none()
        && m.self_version.is_none()
        && m.self_flags.is_none()
        && m.sections.is_empty()
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
    #[serde(with = "hex_signed")]
    pub addend: i64,
    #[serde(alias = "r_type")]
    pub kind: RelocationKind,
    #[serde(default, alias = "r_sym")]
    pub symbol_index: u32,
    pub is_plt: bool,
}

// ---------------------------------------------------------------------------
// BinaryMetadata — ELF identity, build ID, sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_id: Option<String>,
    pub elf_type: u16,
    pub elf_flags: u32,
    pub osabi: u8,
    pub ei_abi_version: u8,
    pub e_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_key_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_attr: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_mode: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_endian: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_version: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_flags: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<SectionHeader>,
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
    #[serde(default, skip_serializing_if = "is_default_metadata")]
    pub metadata: BinaryMetadata,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dynamic_entries: Vec<DynamicEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_defs: Vec<VersionDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib_versions: Vec<LibVersionEntry>,
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
            buf.push(0);
            buf.push(0);
            buf.extend_from_slice(&0u16.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.push(0);
            buf.push(0);
            buf.extend_from_slice(&0u16.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
            buf
        };

        let dynamic_entries = {
            vec![
                (DT_STRTAB, 0u64),
                (DT_STRSZ, strtab.len() as u64),
                (DT_SYMTAB, 0u64),
                (DT_SYMENT, 24u64),
                (0x6100003Fu64, symtab.len() as u64),
                (0x61000049u64, 9),
                (0u64, 0u64),
            ]
        };

        let dyn_byte_size = dynamic_entries.len() * 16;
        let load_vaddr = 0x1000u64;
        let strtab_vaddr = load_vaddr + dyn_byte_size as u64;
        let symtab_vaddr = strtab_vaddr + strtab.len() as u64;

        let dynamic = {
            let mut buf = Vec::new();
            let write_u64 = |buf: &mut Vec<u8>, v: u64| {
                buf.extend_from_slice(&v.to_le_bytes());
            };
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
            write_u64(&mut buf, 9);
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
        assert_eq!(
            Platform::from_self(ps5_self::SelfPlatform::Ps5),
            Platform::Ps5
        );
        assert_eq!(
            Platform::from_self(ps5_self::SelfPlatform::Ps4),
            Platform::Ps4
        );
        assert_eq!(
            Platform::from_self(ps5_self::SelfPlatform::RawElf),
            Platform::RawElf
        );
        assert_eq!(
            Platform::from_self(ps5_self::SelfPlatform::Unknown(99)),
            Platform::Unknown
        );
    }

    #[test]
    fn builder_produces_valid_image() {
        let catalog = ps5_nid::Catalog::new();
        let img = BinaryImageBuilder::build_from_file(
            b"not a real file",
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
        let img = BinaryImageBuilder::build_from_file(&elf_data, &sha256, &catalog);

        assert_eq!(img.platform, Platform::RawElf);
        assert!(!img.is_self);
        assert_eq!(img.entry_point, 0x1000);
        assert!(img.segments.len() >= 2);

        let dyn_seg = img
            .segments
            .iter()
            .find(|s| s.vaddr == 0x1000 && s.filesz < 200);
        assert!(dyn_seg.is_some(), "should have DYNAMIC segment");

        assert_eq!(img.imports.len(), 1);
        assert_eq!(img.imports[0].nid_hash, "hello");
        assert_eq!(img.imports[0].library_name, "libSceFoo");
        assert_eq!(img.imports[0].library_id, 0);

        assert_eq!(img.metadata.elf_type, ET_SCE_DYNAMIC);
        assert_eq!(img.metadata.elf_flags, 0);
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
        let img = BinaryImageBuilder::build_from_file(&file, &sha256, &catalog);

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
            seg_type: SegmentType::Load,
            p_paddr: 0,
            p_align: 0x1000,
            is_encrypted: false,
            is_compressed: false,
            phdr_index: None,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"flags\":\"RX\""));
        let back: LoadedSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.vaddr, 0x100000);
        assert!(back.is_executable);
        assert!(!back.is_writable);
        assert_eq!(back.seg_type, SegmentType::Load);
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
            seg_type: SegmentType::Load,
            p_paddr: 0,
            p_align: 0,
            is_encrypted: false,
            is_compressed: false,
            phdr_index: None,
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
            seg_type: SegmentType::Load,
            p_paddr: 0,
            p_align: 0,
            is_encrypted: false,
            is_compressed: false,
            phdr_index: None,
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(json.contains("\"flags\":\"R\""));
    }

    #[test]
    fn segment_type_roundtrip() {
        let cases = [
            SegmentType::Null,
            SegmentType::Load,
            SegmentType::Dynamic,
            SegmentType::Tls,
            SegmentType::SCE_Dynlibdata,
            SegmentType::Other(0x1234),
        ];
        for st in cases {
            let json = serde_json::to_string(&st).unwrap();
            let back: SegmentType = serde_json::from_str(&json).unwrap();
            assert_eq!(st, back);
        }
    }

    #[test]
    fn relocation_kind_roundtrip() {
        let cases = [
            RelocationKind::None,
            RelocationKind::_64,
            RelocationKind::Relative,
            RelocationKind::GlobDat,
            RelocationKind::JumpSlot,
            RelocationKind::Copy,
            RelocationKind::TPOff64,
            RelocationKind::Other(0x999),
        ];
        for rk in cases {
            let json = serde_json::to_string(&rk).unwrap();
            let back: RelocationKind = serde_json::from_str(&json).unwrap();
            assert_eq!(rk, back);
        }
    }

    #[test]
    fn relocation_backward_compat_r_type() {
        let json =
            r#"{"offset":"0x1000","addend":"0x0","r_type":7,"symbol_index":3,"is_plt":true}"#;
        let rel: RelocationEntry = serde_json::from_str(json).unwrap();
        assert_eq!(rel.kind, RelocationKind::JumpSlot);
        assert_eq!(rel.symbol_index, 3);
        assert!(rel.is_plt);
    }

    #[test]
    fn import_entry_defaults() {
        let json = r#"{"nid_hash":"abc","library_id":1,"library_name":"lib"}"#;
        let imp: ImportEntry = serde_json::from_str(json).unwrap();
        assert_eq!(imp.binding, SymbolBinding::Global);
        assert_eq!(imp.sym_type, SymbolType::Func);
        assert_eq!(imp.visibility, SymbolVisibility::Default);
        assert_eq!(imp.value, 0);
        assert_eq!(imp.ordinal, 0);
    }

    #[test]
    fn section_header_flags_string() {
        assert_eq!(SectionHeader::flags_string(0x2 | 0x4), "AX");
        assert_eq!(SectionHeader::flags_string(0x2 | 0x1), "AW");
        assert_eq!(SectionHeader::flags_string(0x2 | 0x1 | 0x4), "AWX");
        assert_eq!(SectionHeader::flags_string(0), " ");
    }

    #[test]
    fn dynamic_entry_tag_names() {
        assert_eq!(DynamicEntry::tag_name(0), Some("DT_NULL"));
        assert_eq!(DynamicEntry::tag_name(1), Some("DT_NEEDED"));
        assert_eq!(
            DynamicEntry::tag_name(0x61000049),
            Some("DT_SCE_NEEDED_LIB")
        );
        assert_eq!(DynamicEntry::tag_name(0xDEAD), None);
    }

    #[test]
    fn binary_metadata_defaults() {
        let m = BinaryMetadata::default();
        assert_eq!(m.elf_type, 0);
        assert!(m.sections.is_empty());
        assert!(m.build_id.is_none());
    }

    #[test]
    fn binary_metadata_skip_empty() {
        let img = BinaryImage {
            sha256: "0".repeat(64),
            platform: Platform::Unknown,
            is_self: false,
            file_size: 0,
            entry_point: 0,
            metadata: BinaryMetadata::default(),
            segments: vec![],
            imports: vec![],
            exports: vec![],
            relocations: vec![],
            tls: None,
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            import_libs: HashMap::new(),
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions: vec![],
        };
        let json = serde_json::to_string(&img).unwrap();
        assert!(!json.contains("metadata"));
    }

    #[test]
    fn backward_compat_no_new_fields() {
        let json = r#"{"sha256":"aa","platform":"Unknown","is_self":false,"file_size":0,"entry_point":"0x0","segments":[],"init_va":"0x0","init_array_va":"0x0","init_array_sz":0,"fini_va":"0x0","fini_array_va":"0x0","fini_array_sz":0,"preinit_array_va":"0x0","preinit_array_sz":0}"#;
        let img: BinaryImage = serde_json::from_str(json).unwrap();
        assert!(img.dynamic_entries.is_empty());
        assert!(img.version_defs.is_empty());
        assert_eq!(img.metadata.elf_type, 0);
    }
}
