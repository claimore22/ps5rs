//! Byte-patch support: address/expected-bytes/patch-bytes entries applied
//! against guest memory after the game image is loaded.

use crate::error::BleResult;
use ps5_memory_safe::MemoryManager;

#[derive(Debug, Clone)]
pub struct PatchEntry {
    pub address: u64,
    pub original: Vec<u8>,
    pub patched: Vec<u8>,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct GamePatch {
    pub entries: Vec<PatchEntry>,
    pub description: String,
}

impl GamePatch {
    pub fn new(description: &str) -> Self {
        Self {
            entries: Vec::new(),
            description: description.to_string(),
        }
    }

    pub fn add_entry(
        &mut self,
        address: u64,
        original: Vec<u8>,
        patched: Vec<u8>,
        description: &str,
    ) {
        self.entries.push(PatchEntry {
            address,
            original,
            patched,
            enabled: true,
            description: description.to_string(),
        });
    }

    pub fn apply(&self, memory: &MemoryManager) -> BleResult<usize> {
        let mut applied = 0;
        for entry in &self.entries {
            if !entry.enabled {
                continue;
            }

            let mut current = vec![0u8; entry.original.len()];
            memory.read(entry.address, &mut current)?;

            if current == entry.original {
                memory.write(entry.address, &entry.patched)?;
                applied += 1;
            }
        }
        Ok(applied)
    }
}
