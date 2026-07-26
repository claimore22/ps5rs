use std::io::{Read, Write};

use crate::{BinaryImage, BinaryImageDocument, BINARY_IMAGE_VERSION};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum JsonError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Serde(e) => write!(f, "JSON parse error: {e}"),
            Self::UnsupportedVersion(v) => {
                write!(f, "Unsupported schema version {v} (max supported: {BINARY_IMAGE_VERSION})")
            }
        }
    }
}

impl std::error::Error for JsonError {}

impl From<std::io::Error> for JsonError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

pub fn export_json(image: &BinaryImage, writer: &mut dyn Write) -> Result<(), JsonError> {
    let doc = BinaryImageDocument {
        schema_version: BINARY_IMAGE_VERSION,
        tool: "ps5rs".to_string(),
        image: image.clone(),
    };
    let json = serde_json::to_string_pretty(&doc)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

pub fn import_json(reader: &mut dyn Read) -> Result<BinaryImageDocument, JsonError> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    let doc: BinaryImageDocument = serde_json::from_str(&buf)?;
    if doc.schema_version > BINARY_IMAGE_VERSION {
        return Err(JsonError::UnsupportedVersion(doc.schema_version));
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryMetadata, LoadedSegment, Platform, RelocationKind, SegmentType};

    fn make_test_image() -> BinaryImage {
        BinaryImage {
            sha256: "abcdef0123456789".repeat(4),
            platform: Platform::RawElf,
            is_self: false,
            file_size: 4096,
            entry_point: 0x1000,
            metadata: BinaryMetadata::default(),
            segments: vec![
                LoadedSegment {
                    vaddr: 0x1000,
                    file_offset: 0x1000,
                    filesz: 256,
                    memsz: 512,
                    is_executable: true,
                    is_writable: false,
                    seg_type: SegmentType::Load,
                    p_paddr: 0,
                    p_align: 0x1000,
                    is_encrypted: false,
                    is_compressed: false,
                    phdr_index: None,
                },
                LoadedSegment {
                    vaddr: 0x2000,
                    file_offset: 0x2000,
                    filesz: 1024,
                    memsz: 1024,
                    is_executable: false,
                    is_writable: true,
                    seg_type: SegmentType::Load,
                    p_paddr: 0,
                    p_align: 0x1000,
                    is_encrypted: false,
                    is_compressed: false,
                    phdr_index: None,
                },
            ],
            imports: vec![],
            exports: vec![],
            relocations: vec![],
            tls: None,
            init_va: 0x1000,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: Vec::new(),
            version_defs: Vec::new(),
        }
    }

    #[test]
    fn roundtrip_empty_image() {
        let img = make_test_image();
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let doc = import_json(&mut buf.as_slice()).unwrap();
        assert_eq!(doc.schema_version, BINARY_IMAGE_VERSION);
        assert_eq!(doc.tool, "ps5rs");
        assert_eq!(doc.image.platform, Platform::RawElf);
        assert_eq!(doc.image.entry_point, 0x1000);
        assert_eq!(doc.image.segments.len(), 2);
        assert!(doc.image.imports.is_empty());
    }

    #[test]
    fn golden_json_fields() {
        let img = make_test_image();
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();

        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["tool"], "ps5rs");
        assert_eq!(parsed["image"]["platform"], "RawELF");
        assert_eq!(parsed["image"]["entry_point"], "0x1000");
        assert_eq!(parsed["image"]["is_self"], false);
        assert_eq!(parsed["image"]["file_size"], 4096);
    }

    #[test]
    fn golden_json_segments() {
        let img = make_test_image();
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();

        let segs = parsed["image"]["segments"].as_array().unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0]["vaddr"], "0x1000");
        assert_eq!(segs[0]["flags"], "RX");
        assert_eq!(segs[0]["filesz"], 256);
        assert_eq!(segs[0]["memsz"], 512);
        assert_eq!(segs[1]["vaddr"], "0x2000");
        assert_eq!(segs[1]["flags"], "RW");
    }

    #[test]
    fn golden_json_hex_addresses() {
        let mut img = make_test_image();
        img.init_va = 0x4000;
        img.init_array_va = 0x8000;
        img.fini_va = 0xC000;
        img.entry_point = 0xDEAD_BEEF_CAFE;
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();

        assert_eq!(parsed["image"]["entry_point"], "0xdeadbeefcafe");
        assert_eq!(parsed["image"]["init_va"], "0x4000");
        assert_eq!(parsed["image"]["init_array_va"], "0x8000");
        assert_eq!(parsed["image"]["fini_va"], "0xc000");
    }

    #[test]
    fn empty_image_roundtrip() {
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
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: Vec::new(),
            version_defs: Vec::new(),
        };
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let doc = import_json(&mut buf.as_slice()).unwrap();
        assert_eq!(doc.image.platform, Platform::Unknown);
        assert_eq!(doc.image.entry_point, 0);
        assert!(doc.image.segments.is_empty());
    }

    #[test]
    fn empty_image_skips_optional_fields() {
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
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: Vec::new(),
            version_defs: Vec::new(),
        };
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();

        // skipped when empty
        assert!(parsed["image"]["tls"].is_null());
        assert!(parsed["image"]["imports"].is_null());
        assert!(parsed["image"]["exports"].is_null());
        assert!(parsed["image"]["relocations"].is_null());
        assert!(parsed["image"]["needed_files"].is_null());
        assert!(parsed["image"]["import_libs"].is_null());
    }

    #[test]
    fn invalid_json_returns_serde_error() {
        let input = b"{not valid json";
        let result = import_json(&mut input.as_slice());
        assert!(matches!(result, Err(JsonError::Serde(_))));
    }

    #[test]
    fn rejects_future_schema_version() {
        let input = r#"{"schema_version": 999, "tool": "test", "image": {"sha256": "aa", "platform": "Unknown", "is_self": false, "file_size": 0, "entry_point": "0x0", "segments": [], "init_va": "0x0", "init_array_va": "0x0", "init_array_sz": 0, "fini_va": "0x0", "fini_array_va": "0x0", "fini_array_sz": 0, "preinit_array_va": "0x0", "preinit_array_sz": 0}}"#;
        let result = import_json(&mut input.as_bytes());
        assert!(matches!(result, Err(JsonError::UnsupportedVersion(999))));
    }

    #[test]
    fn hex_zero_roundtrip() {
        let img = BinaryImage {
            sha256: "0".repeat(64),
            platform: Platform::Ps5,
            is_self: true,
            file_size: 1024,
            entry_point: 0,
            metadata: BinaryMetadata::default(),
            segments: vec![LoadedSegment {
                vaddr: 0,
                file_offset: 0,
                filesz: 0,
                memsz: 0,
                is_executable: false,
                is_writable: false,
                seg_type: SegmentType::Load,
                p_paddr: 0,
                p_align: 0x1000,
                is_encrypted: false,
                is_compressed: false,
                phdr_index: None,
            }],
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
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: Vec::new(),
            version_defs: Vec::new(),
        };
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let doc = import_json(&mut buf.as_slice()).unwrap();
        assert_eq!(doc.image.entry_point, 0);
        assert_eq!(doc.image.segments[0].vaddr, 0);
        assert_eq!(doc.image.segments[0].flags(), "R");

        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"entry_point\": \"0x0\""));
    }

    #[test]
    fn relocation_hex_roundtrip() {
        let img = BinaryImage {
            sha256: "0".repeat(64),
            platform: Platform::RawElf,
            is_self: false,
            file_size: 0,
            entry_point: 0,
            metadata: BinaryMetadata::default(),
            segments: vec![],
            imports: vec![],
            exports: vec![],
            relocations: vec![crate::RelocationEntry {
                offset: 0x1000,
                kind: RelocationKind::Relative,
                addend: -16,
                symbol_index: 1,
                is_plt: false,
            }],
            tls: None,
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: Vec::new(),
            version_defs: Vec::new(),
        };
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        let doc = import_json(&mut buf.as_slice()).unwrap();
        assert_eq!(doc.image.relocations[0].offset, 0x1000);
        assert_eq!(doc.image.relocations[0].addend, -16);
    }

    #[test]
    fn ends_with_newline() {
        let img = make_test_image();
        let mut buf = Vec::new();
        export_json(&img, &mut buf).unwrap();
        assert_eq!(*buf.last().unwrap(), b'\n');
    }
}
