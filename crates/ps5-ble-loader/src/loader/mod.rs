//! Game loading: directory discovery, SELF/ELF parsing, module loading and
//! byte-patch application.

pub mod elf;
pub mod game_dir;
pub mod loader;
pub mod patch;

pub use elf::{ElfLoader, SelfSegment};
pub use game_dir::GameDirectory;
pub use loader::{DynamicSymbol, ElfSection, ElfSymbol, LoadedElf, ProgramHeader, RelocationEntry};
pub use patch::{GamePatch, PatchEntry};
