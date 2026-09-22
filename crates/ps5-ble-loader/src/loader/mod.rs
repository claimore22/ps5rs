//! Game loading: directory discovery, SELF/ELF parsing, module loading and
//! byte-patch application.

pub mod elf;
pub mod game_dir;
pub mod patch;
pub mod types;

pub use elf::{ElfLoader, SelfSegment};
pub use game_dir::GameDirectory;
pub use patch::{GamePatch, PatchEntry};
pub use types::{DynamicSymbol, ElfSection, ElfSymbol, LoadedElf, ProgramHeader, RelocationEntry};
