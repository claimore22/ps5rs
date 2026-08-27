#![allow(non_camel_case_types)]
#![allow(clippy::new_without_default)]
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("generic memory error: {0}")]
    Generic(String),
}

pub type MemoryResult<T> = Result<T, MemoryError>;

#[derive(Debug, Clone, Copy)]
pub enum MemoryProtection {
    NONE,
    READ,
    WRITE,
    EXECUTE,
    READ_WRITE,
    READ_EXECUTE,
    READ_WRITE_EXECUTE,
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub address: u64,
    pub size: u64,
    pub prot: MemoryProtection,
    pub name: String,
}

pub struct MemoryManager;

impl MemoryManager {
    pub fn new() -> Self {
        Self
    }
    // Stub implementations – safe wrappers around OS mappings.
    pub fn map_host_memory(
        &self,
        _addr: Option<u64>,
        size: u64,
        _align: u64,
        prot: MemoryProtection,
        name: &str,
    ) -> MemoryResult<MemoryRegion> {
        Ok(MemoryRegion {
            address: _addr.unwrap_or(0x1000_0000),
            size,
            prot,
            name: name.to_string(),
        })
    }
    pub fn map_program_image(&self, _base: u64, _end: u64) -> MemoryResult<()> {
        Ok(())
    }
    pub fn commit_range(&self, _base: u64, _end: u64) -> MemoryResult<()> {
        Ok(())
    }
    pub fn write(&self, _addr: u64, _data: &[u8]) -> MemoryResult<()> {
        Ok(())
    }
    pub fn read(&self, _addr: u64, buf: &mut [u8]) -> MemoryResult<()> {
        for b in buf.iter_mut() {
            *b = 0;
        }
        Ok(())
    }
    pub fn protect(&self, _addr: u64, _size: u64, _prot: MemoryProtection) -> MemoryResult<()> {
        Ok(())
    }
}
