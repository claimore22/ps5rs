/// SCE_LIBVERSION segment parser.
///
/// The segment at PT_SCE_LIBVERSION (0x61000003) contains a sequence of
/// length-prefixed entries. Each entry's payload is an ASCII string of the
/// form `"library_name:XXXXXXXX"` where the trailing 4 bytes are a big-endian
/// uint32 encoding the SDK version.
///
/// SDK version encoding (hypothesis — confirmed across PS4 tooling):
///   major = value >> 24
///   minor = (value >> 12) & 0xFFF
///   patch = value & 0xFFF
///
/// We keep the raw bytes alongside the parsed fields so the dataset remains
/// valuable even if the format diverges on PS5.
use std::fmt;

#[derive(Debug, Clone)]
pub struct LibVersionEntry {
    pub name: String,
    pub version_raw: u32,
    /// Raw payload bytes including the colon and version encoding.
    /// Retained for forward compatibility if PS5 adds extra fields.
    pub raw: Vec<u8>,
}

impl LibVersionEntry {
    /// Best-guess version string from the big-endian version field.
    /// Named `guessed_version_string` until confirmed across multiple binaries.
    pub fn guessed_version_string(&self) -> String {
        let major = self.version_raw >> 24;
        let minor = (self.version_raw >> 12) & 0xFFF;
        let patch = self.version_raw & 0xFFF;
        format!("{major}.{minor}.{patch}")
    }
}

impl fmt::Display for LibVersionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.guessed_version_string())
    }
}

/// Parse the raw bytes of a PT_SCE_LIBVERSION segment into a list of entries.
///
/// Each entry is: `{ u8 length, u8[length] payload }`.
/// The parser is tolerant of truncation — it returns whatever it could parse.
pub fn parse_libversion(data: &[u8]) -> Vec<LibVersionEntry> {
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let length = data[offset] as usize;
        offset += 1;

        if length == 0 {
            break;
        }

        if offset + length > data.len() {
            break;
        }

        let payload = &data[offset..offset + length];
        offset += length;

        let payload_str = std::str::from_utf8(payload).unwrap_or("");

        // Expected format: "libname:XXXXXXXX"
        let (name, version_raw) = if let Some(colon_pos) = payload_str.rfind(':') {
            let name = payload_str[..colon_pos].to_string();
            let version_str = &payload_str[colon_pos + 1..];

            // Try to decode the version part. It might be:
            // 1. A hex string like "05008001"
            // 2. Raw 4 bytes at the end of the payload
            let version_raw = if version_str.len() == 8 {
                u32::from_str_radix(version_str, 16).unwrap_or(0)
            } else if payload.len() >= 4 {
                u32::from_be_bytes([
                    payload[payload.len() - 4],
                    payload[payload.len() - 3],
                    payload[payload.len() - 2],
                    payload[payload.len() - 1],
                ])
            } else {
                0
            };

            (name, version_raw)
        } else {
            // No colon found — treat entire payload as the name
            (payload_str.to_string(), 0)
        };

        entries.push(LibVersionEntry {
            name,
            version_raw,
            raw: payload.to_vec(),
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_libversion_segment(entries: &[(&str, u32)]) -> Vec<u8> {
        let mut buf = Vec::new();
        for &(name, version) in entries {
            let version_hex = format!("{:08x}", version);
            let payload = format!("{name}:{version_hex}");
            buf.push(payload.len() as u8);
            buf.extend_from_slice(payload.as_bytes());
        }
        buf
    }

    #[test]
    fn parse_single_entry() {
        let data = build_libversion_segment(&[("libkernel", 0x05008001)]);
        let entries = parse_libversion(&data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "libkernel");
        assert_eq!(entries[0].version_raw, 0x05008001);
        assert_eq!(entries[0].guessed_version_string(), "5.8.1");
    }

    #[test]
    fn parse_multiple_entries() {
        let data = build_libversion_segment(&[
            ("libkernel", 0x05008001),
            ("libSceLibcInternal", 0x05008001),
            ("libSceSysmodule", 0x04000031),
        ]);
        let entries = parse_libversion(&data);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "libkernel");
        assert_eq!(entries[1].name, "libSceLibcInternal");
        assert_eq!(entries[2].name, "libSceSysmodule");
        assert_eq!(entries[2].version_raw, 0x04000031);
        assert_eq!(entries[2].guessed_version_string(), "4.0.49");
    }

    #[test]
    fn parse_empty_segment() {
        let entries = parse_libversion(&[]);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_truncated_entry_returns_partial() {
        let mut data = build_libversion_segment(&[("libA", 0x01000001)]);
        data.truncate(data.len() - 2);
        let entries = parse_libversion(&data);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_zero_length_terminates() {
        let mut data = build_libversion_segment(&[("libA", 0x01000001)]);
        data.push(0); // zero-length terminator
        data.extend_from_slice(b"libB:deadbeef");
        let entries = parse_libversion(&data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "libA");
    }

    #[test]
    fn parse_no_colon_falls_back() {
        let mut buf = Vec::new();
        buf.push(5);
        buf.extend_from_slice(b"hello");
        let entries = parse_libversion(&buf);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello");
        assert_eq!(entries[0].version_raw, 0);
    }

    #[test]
    fn version_string_formats() {
        let entry = LibVersionEntry {
            name: "test".to_string(),
            version_raw: 0x0A00B00C,
            raw: Vec::new(),
        };
        assert_eq!(entry.guessed_version_string(), "10.11.12");
    }

    #[test]
    fn raw_bytes_preserved() {
        let data = build_libversion_segment(&[("libFoo", 0xDEADBEEF)]);
        let entries = parse_libversion(&data);
        assert_eq!(entries[0].raw, b"libFoo:deadbeef");
    }

    #[test]
    fn display_trait() {
        let entry = LibVersionEntry {
            name: "libkernel".to_string(),
            version_raw: 0x05008001,
            raw: Vec::new(),
        };
        assert_eq!(format!("{entry}"), "libkernel 5.8.1");
    }
}
