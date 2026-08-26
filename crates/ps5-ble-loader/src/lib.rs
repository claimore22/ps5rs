#![forbid(unsafe_code)]

pub mod loader;
pub mod error;
pub mod log_targets;
use loader::{ElfLoader, LoadedElf};

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_memory_safe::MemoryManager;

    #[test]
    fn load_libdbg_basic() {
        let path = r"../../data/test/generated_elfs/libdbg_basic.elf";
        let loader = ElfLoader::new();
        let mem = MemoryManager::new();
        let result = loader.load(path, &mem);
        assert!(result.is_ok(), "Failed to load ELF: {:?}", result.err());
        let loaded: LoadedElf = result.unwrap();
        assert!(loaded.entry_point != 0, "Entry point should not be zero");
    }
}

