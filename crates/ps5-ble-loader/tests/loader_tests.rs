use ps5_ble_loader::loader::{ElfLoader, LoadedElf};
use ps5_memory_safe::MemoryManager;

#[test]
fn load_libdbg_basic() {
    // Path is relative to workspace root
    let path = r"../../data/test/generated_elfs/libdbg_basic.elf";
    let loader = ElfLoader::new();

    // Create a default empty memory manager
    let mem = MemoryManager::new();

    // Load the ELF
    let result = loader.load(path, &mem);
    assert!(result.is_ok(), "Failed to load ELF: {:?}", result.err());

    let loaded: LoadedElf = result.unwrap();
    assert!(loaded.entry_point != 0, "Entry point should not be zero");
}
