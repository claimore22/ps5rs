use std::fmt;

use crate::emulator::EmuState;

/// Errors raised by the emulator, its loader integration, and module handlers.
#[derive(Debug)]
pub enum EmuError {
    /// Failure from the `ps5-loader` pipeline.
    Loader(ps5_loader::LoaderError),
    /// Failure from the virtual memory model.
    Memory(ps5_loader::MemoryError),
    /// Host OS memory allocation / protection failed.
    Alloc(String),
    /// The eboot has no `e_entry`.
    NoEntryPoint,
    /// `start()` was called before `resolve_imports()`.
    ImportsNotResolved,
    /// An operation requires a different [`EmuState`].
    InvalidState(EmuState),
    /// No HLE handler exists for the requested symbol.
    NoHandler(String),
    /// The guest called `exit`, carrying the exit code.
    GuestExit(u64),
    /// The address falls outside every mapped module region.
    Unmapped(u64),
    /// A guest pointer of `0` was dereferenced.
    NullPointer,
    /// A guest string exceeded the read limit.
    StringTooLong(u64),
    /// A guest string was not valid UTF-8.
    InvalidUtf8(u64),
    /// A binary failed to parse.
    Parse(String),
}

impl fmt::Display for EmuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmuError::Loader(e) => write!(f, "loader: {e}"),
            EmuError::Memory(e) => write!(f, "memory: {e}"),
            EmuError::Alloc(e) => write!(f, "alloc: {e}"),
            EmuError::NoEntryPoint => write!(f, "module has no entry point"),
            EmuError::ImportsNotResolved => write!(f, "imports not resolved"),
            EmuError::InvalidState(s) => write!(f, "invalid emulator state: {s:?}"),
            EmuError::NoHandler(s) => write!(f, "no handler for {s}"),
            EmuError::GuestExit(code) => write!(f, "guest exited with code {code}"),
            EmuError::Unmapped(a) => write!(f, "address {a:#x} is not mapped"),
            EmuError::NullPointer => write!(f, "null pointer dereference"),
            EmuError::StringTooLong(a) => write!(f, "string at {a:#x} exceeds read limit"),
            EmuError::InvalidUtf8(a) => write!(f, "non-UTF-8 string at {a:#x}"),
            EmuError::Parse(s) => write!(f, "parse: {s}"),
        }
    }
}

impl std::error::Error for EmuError {}

impl From<ps5_loader::LoaderError> for EmuError {
    fn from(e: ps5_loader::LoaderError) -> Self {
        EmuError::Loader(e)
    }
}

impl From<ps5_loader::MemoryError> for EmuError {
    fn from(e: ps5_loader::MemoryError) -> Self {
        EmuError::Memory(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_error_converts() {
        let err = EmuError::from(ps5_loader::LoaderError("boom".into()));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn guest_exit_displays_code() {
        assert!(EmuError::GuestExit(3).to_string().contains("3"));
    }

    #[test]
    fn unmapped_displays_hex() {
        assert_eq!(
            EmuError::Unmapped(0x1234).to_string(),
            "address 0x1234 is not mapped"
        );
    }
}
