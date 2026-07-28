use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidMagic {
        expected: u32,
        actual: u32,
    },
    UnsupportedVersion(u16),
    Truncated {
        offset: u64,
        needed: u64,
        available: u64,
    },
    UnsupportedElfClass(u8),
    UnsupportedElfEndian(u8),
    NotX86_64(u16),
    MissingProgramHeader(&'static str),
    NoDynamicSection,
    InvalidStringTable,
    InvalidSymbolTable,
    InvalidRelocationTable,
    MalformedDynamic,
    Custom(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { expected, actual } => {
                write!(f, "invalid magic: expected {expected:#x}, got {actual:#x}")
            }
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            Self::Truncated {
                offset,
                needed,
                available,
            } => write!(
                f,
                "file truncated: need {needed} bytes at offset {offset}, have {available}"
            ),
            Self::UnsupportedElfClass(c) => write!(f, "unsupported ELF class: {c}"),
            Self::UnsupportedElfEndian(e) => write!(f, "unsupported ELF endianness: {e}"),
            Self::NotX86_64(m) => write!(f, "not x86-64 (machine: {m:#x})"),
            Self::MissingProgramHeader(name) => {
                write!(f, "missing required program header: {name}")
            }
            Self::NoDynamicSection => write!(f, "no PT_DYNAMIC program header found"),
            Self::InvalidStringTable => write!(f, "invalid or missing string table"),
            Self::InvalidSymbolTable => write!(f, "invalid or missing symbol table"),
            Self::InvalidRelocationTable => write!(f, "invalid or missing relocation table"),
            Self::MalformedDynamic => write!(f, "malformed dynamic section"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

pub type Result<T> = std::result::Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_magic_display() {
        let e = ParseError::InvalidMagic {
            expected: 0x7f454c46,
            actual: 0x4f153d1d,
        };
        let msg = format!("{e}");
        assert!(msg.contains("0x7f454c46"), "got: {msg}");
        assert!(msg.contains("0x4f153d1d"), "got: {msg}");
    }

    #[test]
    fn truncated_display() {
        let e = ParseError::Truncated {
            offset: 64,
            needed: 128,
            available: 80,
        };
        let msg = format!("{e}");
        assert!(msg.contains("64"), "got: {msg}");
        assert!(msg.contains("128"), "got: {msg}");
        assert!(msg.contains("80"), "got: {msg}");
    }

    #[test]
    fn custom_display() {
        let e = ParseError::Custom("test error".into());
        assert_eq!(format!("{e}"), "test error");
    }

    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ParseError>();
    }

    #[test]
    fn error_is_clone() {
        let e = ParseError::Custom("clone me".into());
        let e2 = e.clone();
        assert_eq!(format!("{e}"), format!("{e2}"));
    }

    #[test]
    fn elf_class_error() {
        let e = ParseError::UnsupportedElfClass(1);
        assert!(format!("{e}").contains("1"));
    }

    #[test]
    fn not_x86_64_error() {
        let e = ParseError::NotX86_64(0x28);
        assert!(format!("{e}").contains("0x28"));
    }
}
