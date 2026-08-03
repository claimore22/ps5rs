use std::collections::HashSet;

use ps5_loader::{DEFAULT_LOAD_BASE, LoadedModule, ModuleContext, OfflineExportTable};

use crate::error::EmuError;
use crate::imports::ImportTable;

/// The raw bytes of a module image, kept for metadata re-parsing.
#[derive(Debug, Clone)]
pub struct ModuleBytes {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// A loaded PS5 process: mapped modules plus the resolved import table.
pub struct Process {
    ctx: ModuleContext,
    sources: Vec<ModuleBytes>,
    imports: Option<ImportTable>,
}

impl Process {
    /// Load an eboot and its PRX dependencies through the loader pipeline.
    ///
    /// `prx_provider` is asked for each `DT_NEEDED` module by name; the bytes
    /// it returns are also retained so import metadata can be re-derived.
    pub fn load(
        eboot_name: &str,
        eboot_bytes: Vec<u8>,
        prx_provider: impl FnMut(&str) -> Option<Vec<u8>>,
        offline: Option<&OfflineExportTable>,
    ) -> Result<Self, EmuError> {
        Self::load_at(
            eboot_name,
            eboot_bytes,
            prx_provider,
            offline,
            DEFAULT_LOAD_BASE,
        )
    }

    /// Like [`Self::load`], but the eboot loads at `base_address` instead of
    /// the default.  Tests that run as separate processes use distinct bases
    /// so their identity-mapped reservations never collide.
    pub fn load_at(
        eboot_name: &str,
        eboot_bytes: Vec<u8>,
        mut prx_provider: impl FnMut(&str) -> Option<Vec<u8>>,
        offline: Option<&OfflineExportTable>,
        base_address: u64,
    ) -> Result<Self, EmuError> {
        let mut sources: Vec<ModuleBytes> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut provider = |name: &str| -> Option<Vec<u8>> {
            let bytes = prx_provider(name)?;
            if seen.insert(name.to_string()) {
                sources.push(ModuleBytes {
                    name: name.to_string(),
                    bytes: bytes.clone(),
                });
            }
            Some(bytes)
        };
        let ctx = ps5_loader::load_modules_at(
            eboot_name,
            &eboot_bytes,
            &mut provider,
            offline,
            base_address,
        )?;
        tracing::debug!(eboot_name, modules = ctx.modules.len(), "process loaded");
        sources.push(ModuleBytes {
            name: eboot_name.to_string(),
            bytes: eboot_bytes,
        });
        Ok(Self {
            ctx,
            sources,
            imports: None,
        })
    }

    pub fn context(&self) -> &ModuleContext {
        &self.ctx
    }

    pub fn modules(&self) -> &[LoadedModule] {
        &self.ctx.modules
    }

    pub fn sources(&self) -> &[ModuleBytes] {
        &self.sources
    }

    pub fn eboot(&self) -> Option<&LoadedModule> {
        self.ctx.eboot()
    }

    pub fn imports(&self) -> Option<&ImportTable> {
        self.imports.as_ref()
    }

    pub fn set_imports(&mut self, table: ImportTable) {
        self.imports = Some(table);
    }

    /// The entry address: `load_bias + e_entry` of the eboot.
    pub fn entry(&self) -> Result<(String, u64), EmuError> {
        let eboot = self.ctx.eboot().ok_or(EmuError::NoEntryPoint)?;
        let entry_point = eboot.entry_point.ok_or(EmuError::NoEntryPoint)?;
        let address = eboot.load_bias.wrapping_add(entry_point);
        Ok(("_start".to_string(), address))
    }

    /// Read `len` bytes from the first module whose memory maps `addr`.
    pub fn read(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError> {
        for module in &self.ctx.modules {
            if let Ok(bytes) = module.memory.read(addr, len) {
                return Ok(bytes.to_vec());
            }
        }
        Err(EmuError::Unmapped(addr))
    }

    /// Write `data` into the first module whose memory maps `addr`.
    pub fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError> {
        for module in &mut self.ctx.modules {
            if module.memory.write(addr, data).is_ok() {
                return Ok(());
            }
        }
        Err(EmuError::Unmapped(addr))
    }

    /// Read a NUL-terminated string starting at `addr`.
    pub fn read_string(&self, addr: u64) -> Result<String, EmuError> {
        if addr == 0 {
            return Err(EmuError::NullPointer);
        }
        let mut out = Vec::new();
        for i in 0..4096u64 {
            let byte = self
                .read(addr.wrapping_add(i), 1)?
                .first()
                .copied()
                .unwrap_or(0);
            if byte == 0 {
                return String::from_utf8(out).map_err(|_| EmuError::InvalidUtf8(addr));
            }
            out.push(byte);
        }
        Err(EmuError::StringTooLong(addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_elf(entry: u64) -> Vec<u8> {
        let mut buf = vec![0u8; 0x1100];
        buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        buf[4] = 2;
        buf[5] = 1;
        buf[6] = 1;
        buf[16..18].copy_from_slice(&0xFE10u16.to_le_bytes());
        buf[18..20].copy_from_slice(&62u16.to_le_bytes());
        buf[20..24].copy_from_slice(&1u32.to_le_bytes());
        buf[24..32].copy_from_slice(&entry.to_le_bytes());
        buf[32..40].copy_from_slice(&64u64.to_le_bytes());
        buf[52..54].copy_from_slice(&64u16.to_le_bytes());
        buf[54..56].copy_from_slice(&56u16.to_le_bytes());
        buf[56..58].copy_from_slice(&1u16.to_le_bytes());

        let phoff = 64usize;
        buf[phoff..phoff + 4].copy_from_slice(&1u32.to_le_bytes());
        buf[phoff + 4..phoff + 8].copy_from_slice(&5u32.to_le_bytes());
        buf[phoff + 8..phoff + 16].copy_from_slice(&0x1000u64.to_le_bytes());
        buf[phoff + 16..phoff + 24].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 24..phoff + 32].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 32..phoff + 40].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 40..phoff + 48].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 48..phoff + 56].copy_from_slice(&0x1000u64.to_le_bytes());

        buf[0x1000..0x1100].fill(0xCC);
        buf
    }

    #[test]
    fn entry_uses_load_bias() {
        let elf = minimal_elf(0x70);
        let process = Process::load("eboot.elf", elf, |_| None, None).unwrap();
        let (name, address) = process.entry().unwrap();
        assert_eq!(name, "_start");
        let eboot = process.eboot().unwrap();
        assert_eq!(address, eboot.load_bias + 0x70);
    }

    #[test]
    fn read_mapped_and_unmapped() {
        let elf = minimal_elf(0);
        let process = Process::load("eboot.elf", elf, |_| None, None).unwrap();
        let base = process.eboot().unwrap().load_bias;
        let bytes = process.read(base + 0x20, 4).unwrap();
        assert_eq!(bytes, [0xCC; 4]);
        let err = process.read(0x1, 1).unwrap_err();
        assert!(matches!(err, EmuError::Unmapped(_)));
    }

    #[test]
    fn no_prx_provider_leaves_libc_out() {
        let elf = minimal_elf(0);
        let process = Process::load("eboot.elf", elf, |_| None, None).unwrap();
        assert_eq!(process.modules().len(), 1);
    }
}
