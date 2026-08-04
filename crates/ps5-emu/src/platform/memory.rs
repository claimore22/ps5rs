//! Executable host memory blocks and the materialized guest address space.

use std::ptr::NonNull;

use crate::error::EmuError;
use crate::modules::Host;
use crate::process::Process;
use ps5_loader::SegmentFlags;

#[cfg(not(target_os = "windows"))]
use super::unix as sys;
#[cfg(target_os = "windows")]
use super::windows as sys;

const PAGE_SIZE: u64 = 0x1000;

/// One reserved span of guest address space, tracked so it can be released
/// without querying the OS for its size.
#[derive(Debug, Clone, Copy)]
pub struct Reservation {
    pub base: NonNull<u8>,
    pub size: usize,
}

/// A host allocation that starts writable and can be promoted to executable.
///
/// Used for the HLE import stub region: stubs are written while the block is
/// writable, then the whole block is flipped to execute-only.
pub struct ExecBlock {
    ptr: NonNull<u8>,
    size: usize,
}

impl ExecBlock {
    pub fn alloc_rw(size: usize) -> Result<Self, EmuError> {
        Ok(Self {
            ptr: sys::alloc_rw(size)?,
            size,
        })
    }

    pub fn address(&self) -> u64 {
        self.ptr.as_ptr() as u64
    }

    pub fn write(&self, offset: usize, data: &[u8]) {
        debug_assert!(offset + data.len() <= self.size);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.as_ptr().add(offset), data.len());
        }
    }

    pub fn read_u64(&self, offset: usize) -> u64 {
        debug_assert!(offset + 8 <= self.size);
        unsafe { (self.ptr.as_ptr().add(offset) as *const u64).read() }
    }

    pub fn make_exec(&self) -> Result<(), EmuError> {
        sys::protect(self.ptr, self.size, true, false, true)
    }

    /// Protection of the page containing `addr` (diagnostics).
    pub fn page_protection(addr: u64) -> Option<(usize, u32, u32)> {
        sys::query_protect(addr)
    }
}

impl Drop for ExecBlock {
    fn drop(&mut self) {
        sys::free(self.ptr, self.size);
    }
}

/// One identity-mapped guest region.
#[derive(Debug)]
pub struct GuestRegion {
    /// Guest virtual address where the region is mapped.
    pub guest_start: u64,
    /// Host pointer (identical to `guest_start` under identity mapping).
    pub host_ptr: NonNull<u8>,
    /// Size in bytes.
    pub size: usize,
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

/// The guest's address space re-materialized as real OS pages.
///
/// Module regions are mapped at their exact load-bias addresses because the
/// loader's relocations bake absolute runtime addresses into GOT slots and
/// data segments.
pub struct GuestMemory {
    regions: Vec<GuestRegion>,
    reservations: Vec<Reservation>,
    output: Vec<String>,
}

impl GuestMemory {
    /// Re-create every loaded module region in real (identity-mapped) memory.
    ///
    /// The whole module space is reserved as one span (Windows reserves at
    /// 64K granularity, so adjacent segments would collide otherwise) and each
    /// region is committed at its page-aligned virtual address, with the
    /// segment's intra-page offset preserved.  Pages shared by several
    /// segments get the union of their permissions.
    pub fn materialize(process: &Process) -> Result<Self, EmuError> {
        let module_regions: Vec<_> = process
            .modules()
            .iter()
            .flat_map(|m| m.memory.regions.iter())
            .collect();

        let mut regions = Vec::new();
        let mut reservations = Vec::new();

        if let (Some(min), Some(max_end)) = (
            module_regions.iter().map(|r| r.vaddr).min(),
            module_regions.iter().map(|r| r.vaddr + r.size as u64).max(),
        ) {
            let reservation = sys::reserve_at(min, (max_end - min) as usize)?;
            reservations.push(reservation);
            let mut page_perms: std::collections::BTreeMap<u64, (bool, bool, bool)> =
                std::collections::BTreeMap::new();
            for r in &module_regions {
                let first_page = r.vaddr & !(PAGE_SIZE - 1);
                let last_page = ((r.vaddr + r.size as u64) + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
                let mut page = first_page;
                while page < last_page {
                    let e = page_perms.entry(page).or_insert((false, false, false));
                    e.0 |= r.permissions.read;
                    e.1 |= r.permissions.write;
                    e.2 |= r.permissions.execute;
                    page += PAGE_SIZE;
                }
            }
            for r in &module_regions {
                let offset = (r.vaddr % PAGE_SIZE) as usize;
                let page_start = r.vaddr - offset as u64;
                let span = offset.saturating_add(r.size);
                let span_rounded = (span + PAGE_SIZE as usize - 1) & !(PAGE_SIZE as usize - 1);
                if span_rounded == 0 {
                    continue;
                }
                let ptr = sys::commit_at(page_start, span_rounded)?;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        r.data.as_ptr(),
                        ptr.as_ptr().add(offset),
                        r.size,
                    );
                }
                let perms =
                    Self::merged_perms(&page_perms, page_start, page_start + span_rounded as u64);
                sys::protect(ptr, span_rounded, perms.read, perms.write, perms.execute)?;
                regions.push(GuestRegion {
                    guest_start: page_start,
                    host_ptr: ptr,
                    size: span_rounded,
                    read: perms.read,
                    write: perms.write,
                    exec: perms.execute,
                });
            }
        }

        Ok(Self {
            regions,
            reservations,
            output: Vec::new(),
        })
    }

    fn merged_perms(
        page_perms: &std::collections::BTreeMap<u64, (bool, bool, bool)>,
        start: u64,
        end: u64,
    ) -> SegmentFlags {
        let mut merged = (false, false, false);
        for (_page, perms) in page_perms.range(start..end) {
            merged.0 |= perms.0;
            merged.1 |= perms.1;
            merged.2 |= perms.2;
        }
        SegmentFlags {
            read: merged.0,
            write: merged.1,
            execute: merged.2,
        }
    }

    /// Find the region containing `addr`.
    pub fn find_region(&self, addr: u64) -> Option<&GuestRegion> {
        self.regions
            .iter()
            .find(|r| r.guest_start <= addr && addr < r.guest_start + r.size as u64)
    }

    /// Identity-map a writable guest stack region below the module space.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn add_guest_stack(&mut self, va: u64, size: u64) -> Result<(), EmuError> {
        if va % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
            return Err(EmuError::Alloc(format!(
                "guest stack {va:#x}+{size:#x} not page aligned"
            )));
        }
        let reservation = sys::reserve_at(va, (size + 0x2000) as usize)?;
        self.reservations.push(reservation);
        let ptr = sys::commit_at(va, (size + 0x2000) as usize)?;
        sys::protect(ptr, size as usize, true, true, false)?;
        self.regions.push(GuestRegion {
            guest_start: va,
            host_ptr: ptr,
            size: size as usize,
            read: true,
            write: true,
            exec: false,
        });
        Ok(())
    }

    /// Host pointer for a guest address inside a readable region.
    pub fn ptr(&self, addr: u64) -> Result<*const u8, EmuError> {
        let region = self.find_region(addr).ok_or(EmuError::Unmapped(addr))?;
        if !region.read {
            return Err(EmuError::Unmapped(addr));
        }
        Ok(unsafe {
            region
                .host_ptr
                .as_ptr()
                .add((addr - region.guest_start) as usize)
        })
    }

    pub fn regions(&self) -> &[GuestRegion] {
        &self.regions
    }

    /// Take the chunks the guest emitted through [`Host::emit`], clearing the
    /// buffer.
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Chunks the guest emitted through [`Host::emit`], in order.
    pub fn output_lines(&self) -> &[String] {
        &self.output
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        for reservation in &self.reservations {
            sys::free_reservation(*reservation);
        }
    }
}

impl Host for GuestMemory {
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError> {
        let region = self.find_region(addr).ok_or(EmuError::Unmapped(addr))?;
        if !region.read {
            return Err(EmuError::Unmapped(addr));
        }
        let offset = (addr - region.guest_start) as usize;
        let end = addr
            .checked_add(len as u64)
            .ok_or(EmuError::Unmapped(addr))?;
        if end > region.guest_start + region.size as u64 {
            return Err(EmuError::Unmapped(addr));
        }
        let src = unsafe { region.host_ptr.as_ptr().add(offset) };
        Ok(unsafe { std::slice::from_raw_parts(src, len) }.to_vec())
    }

    fn read_string(&self, addr: u64) -> Result<String, EmuError> {
        if addr == 0 {
            return Err(EmuError::NullPointer);
        }
        let region = self.find_region(addr).ok_or(EmuError::Unmapped(addr))?;
        if !region.read {
            return Err(EmuError::Unmapped(addr));
        }
        let start = (addr - region.guest_start) as usize;
        let max = region.size.saturating_sub(start).min(4096);
        let base = region.host_ptr.as_ptr();
        let mut end = start;
        while end < start + max {
            if unsafe { base.add(end).read() } == 0 {
                let bytes = unsafe { std::slice::from_raw_parts(base.add(start), end - start) };
                return String::from_utf8(bytes.to_vec()).map_err(|_| EmuError::InvalidUtf8(addr));
            }
            end += 1;
        }
        Err(EmuError::StringTooLong(addr))
    }

    fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError> {
        if data.is_empty() {
            return Ok(());
        }
        let region = self.find_region(addr).ok_or(EmuError::Unmapped(addr))?;
        if !region.write {
            return Err(EmuError::Unmapped(addr));
        }
        let offset = (addr - region.guest_start) as usize;
        let end = addr
            .checked_add(data.len() as u64)
            .ok_or(EmuError::Unmapped(addr))?;
        if end > region.guest_start + region.size as u64 {
            return Err(EmuError::Unmapped(addr));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                region.host_ptr.as_ptr().add(offset),
                data.len(),
            );
        }
        Ok(())
    }

    fn emit(&mut self, chunk: &str) {
        self.output.push(chunk.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_block_roundtrip() {
        let block = ExecBlock::alloc_rw(4096).unwrap();
        let payload = [0x90u8; 32];
        block.write(0, &payload);
        assert_eq!(&block.read_u64(0).to_le_bytes(), &payload[..8]);
        block.make_exec().unwrap();
        assert_eq!(block.address() & 0xFFF, 0);
    }

    fn two_segment_elf() -> Vec<u8> {
        let mut buf = vec![0u8; 0x2100];
        buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        buf[4] = 2;
        buf[5] = 1;
        buf[6] = 1;
        buf[16..18].copy_from_slice(&0xFE10u16.to_le_bytes());
        buf[18..20].copy_from_slice(&62u16.to_le_bytes());
        buf[20..24].copy_from_slice(&1u32.to_le_bytes());
        buf[24..32].copy_from_slice(&0u64.to_le_bytes());
        buf[32..40].copy_from_slice(&64u64.to_le_bytes());
        buf[52..54].copy_from_slice(&64u16.to_le_bytes());
        buf[54..56].copy_from_slice(&56u16.to_le_bytes());
        buf[56..58].copy_from_slice(&2u16.to_le_bytes());

        let phoff = 64usize;
        buf[phoff..phoff + 4].copy_from_slice(&1u32.to_le_bytes());
        buf[phoff + 4..phoff + 8].copy_from_slice(&5u32.to_le_bytes());
        buf[phoff + 8..phoff + 16].copy_from_slice(&0x1000u64.to_le_bytes());
        buf[phoff + 16..phoff + 24].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 24..phoff + 32].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 32..phoff + 40].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 40..phoff + 48].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 48..phoff + 56].copy_from_slice(&0x1000u64.to_le_bytes());

        let ph2 = phoff + 56;
        buf[ph2..ph2 + 4].copy_from_slice(&1u32.to_le_bytes());
        buf[ph2 + 4..ph2 + 8].copy_from_slice(&6u32.to_le_bytes());
        buf[ph2 + 8..ph2 + 16].copy_from_slice(&0x2000u64.to_le_bytes());
        buf[ph2 + 16..ph2 + 24].copy_from_slice(&0x100000u64.to_le_bytes());
        buf[ph2 + 24..ph2 + 32].copy_from_slice(&0x100000u64.to_le_bytes());
        buf[ph2 + 32..ph2 + 40].copy_from_slice(&0x100u64.to_le_bytes());
        buf[ph2 + 40..ph2 + 48].copy_from_slice(&0x100u64.to_le_bytes());
        buf[ph2 + 48..ph2 + 56].copy_from_slice(&0x1000u64.to_le_bytes());

        buf[0x1000..0x1100].fill(0xCC);
        buf[0x2000..0x2100].fill(0xDD);
        buf
    }

    #[test]
    fn guest_memory_materialize_and_host_roundtrip() {
        let process = Process::load("eboot.elf", two_segment_elf(), |_| None, None).unwrap();
        let mut mem = GuestMemory::materialize(&process).unwrap();
        assert_eq!(mem.regions().len(), 2);

        for region in mem.regions() {
            assert_eq!(region.host_ptr.as_ptr() as u64, region.guest_start);
        }

        let eboot = process.eboot().unwrap();
        let data_base = eboot.load_bias + 0x100000;
        assert_eq!(mem.read_bytes(data_base, 0x100).unwrap(), vec![0xDD; 0x100]);
        assert_eq!(
            mem.read_bytes(eboot.load_bias + 0x20, 4).unwrap(),
            vec![0xCC; 4]
        );

        mem.write(data_base + 0x10, &[0x42; 8]).unwrap();
        assert_eq!(mem.read_bytes(data_base + 0x10, 8).unwrap(), vec![0x42; 8]);

        let err = mem.write(eboot.load_bias + 0x20, &[0x00]);
        assert!(matches!(err, Err(EmuError::Unmapped(_))));
    }
}
