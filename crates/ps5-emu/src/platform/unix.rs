//! Unix memory primitives (mmap / mprotect / munmap).

use std::ptr::NonNull;

use super::memory::Reservation;
use crate::error::EmuError;

const PAGE_MASK: usize = 0xFFF;
const GRANULARITY: u64 = 0x1_0000;

fn align_up(value: usize) -> usize {
    (value + PAGE_MASK) & !PAGE_MASK
}

pub fn alloc_rw(size: usize) -> Result<NonNull<u8>, EmuError> {
    unsafe {
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            align_up(size),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return Err(EmuError::Alloc(format!(
                "mmap({size:#x}, RW) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(NonNull::new_unchecked(ptr.cast()))
    }
}

/// Reserve a span of address space covering `[addr, addr + size)`, rounded
/// down to the allocation granularity.  Commit sub-ranges with [`commit_at`].
///
/// This uses a `PROT_NONE` anonymous mapping so the span is reserved but not
/// accessible until committed (mirroring Windows `MEM_RESERVE`).
pub fn reserve_at(addr: u64, size: usize) -> Result<Reservation, EmuError> {
    unsafe {
        let base = addr & !(GRANULARITY - 1);
        let end = addr + size as u64;
        let end_rounded = (end + GRANULARITY - 1) & !(GRANULARITY - 1);
        let span = end_rounded.saturating_sub(base);
        let ptr = libc::mmap(
            base as *mut libc::c_void,
            span as usize,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return Err(EmuError::Alloc(format!(
                "mmap reserve({base:#x}, {span:#x}) failed — identity mapping unavailable: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(Reservation {
            base: NonNull::new_unchecked(ptr.cast()),
            size: span as usize,
        })
    }
}

/// Commit a page-aligned range inside a previously reserved span as RW.
///
/// Re-maps the pages with `MAP_FIXED`, which is the Unix equivalent of
/// Windows `MEM_COMMIT` on an existing reservation.
pub fn commit_at(addr: u64, size: usize) -> Result<NonNull<u8>, EmuError> {
    unsafe {
        let ptr = libc::mmap(
            addr as *mut libc::c_void,
            align_up(size),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return Err(EmuError::Alloc(format!(
                "mmap commit({addr:#x}, {size:#x}, RW, MAP_FIXED) failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(NonNull::new_unchecked(ptr.cast()))
    }
}

pub fn free_reservation(reservation: Reservation) {
    unsafe {
        libc::munmap(reservation.base.as_ptr().cast(), align_up(reservation.size));
    }
}

pub fn protect(
    ptr: NonNull<u8>,
    size: usize,
    read: bool,
    write: bool,
    exec: bool,
) -> Result<(), EmuError> {
    let mut prot = 0;
    if read {
        prot |= libc::PROT_READ;
    }
    if write {
        prot |= libc::PROT_WRITE;
    }
    if exec {
        prot |= libc::PROT_EXEC;
    }
    let base = ptr.as_ptr() as usize & !PAGE_MASK;
    let len = align_up((ptr.as_ptr() as usize - base) + size);
    let ok = unsafe { libc::mprotect(base as *mut libc::c_void, len, prot) };
    if ok != 0 {
        return Err(EmuError::Alloc(format!(
            "mprotect({base:#x}, {len:#x}, prot {prot:#x}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

pub fn free(ptr: NonNull<u8>, size: usize) {
    unsafe {
        libc::munmap(ptr.as_ptr().cast(), align_up(size));
    }
}

/// Query the protection of the page containing `addr`.
///
/// Diagnostics-only today; reading `/proc/self/maps` / `mincore` here is
/// deferred, so this reports the pages as readable, writable, executable.
pub fn query_protect(addr: u64) -> Option<(usize, u32, u32)> {
    let _ = addr;
    None
}
