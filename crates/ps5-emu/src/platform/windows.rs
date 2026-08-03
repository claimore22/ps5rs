//! Windows memory primitives (VirtualAlloc / VirtualProtect / VirtualFree).

use std::ptr::NonNull;

use super::memory::Reservation;
use crate::error::EmuError;

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_READONLY: u32 = 0x02;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_EXECUTE_READ: u32 = 0x20;

unsafe extern "system" {
    fn VirtualAlloc(
        lp_address: *mut core::ffi::c_void,
        dw_size: usize,
        fl_allocation_type: u32,
        fl_protect: u32,
    ) -> *mut core::ffi::c_void;
    fn VirtualProtect(
        lp_address: *mut core::ffi::c_void,
        dw_size: usize,
        fl_new_protect: u32,
        lpfl_old_protect: *mut u32,
    ) -> i32;
    fn VirtualFree(lp_address: *mut core::ffi::c_void, dw_size: usize, dw_free_type: u32) -> i32;
    fn VirtualQuery(
        lp_address: *const core::ffi::c_void,
        lp_buffer: *mut MEMORY_BASIC_INFORMATION,
        dw_length: usize,
    ) -> usize;
}

#[repr(C)]
pub struct MEMORY_BASIC_INFORMATION {
    pub base_address: *mut core::ffi::c_void,
    pub allocation_base: *mut core::ffi::c_void,
    pub allocation_protect: u32,
    pub partition_id: u16,
    pub region_size: usize,
    pub state: u32,
    pub protect: u32,
    pub r#type: u32,
}

pub fn query_protect(addr: u64) -> Option<(usize, u32, u32)> {
    let mut mbi = MEMORY_BASIC_INFORMATION {
        base_address: core::ptr::null_mut(),
        allocation_base: core::ptr::null_mut(),
        allocation_protect: 0,
        partition_id: 0,
        region_size: 0,
        state: 0,
        protect: 0,
        r#type: 0,
    };
    let n = unsafe { VirtualQuery(addr as *const _, &mut mbi, core::mem::size_of_val(&mbi)) };
    if n == 0 {
        None
    } else {
        Some((mbi.region_size, mbi.state, mbi.protect))
    }
}

pub fn alloc_rw(size: usize) -> Result<NonNull<u8>, EmuError> {
    unsafe {
        let ptr = VirtualAlloc(
            std::ptr::null_mut(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if ptr.is_null() {
            return Err(EmuError::Alloc(format!(
                "VirtualAlloc({size:#x}, RW) failed (error {})",
                std::io::Error::last_os_error()
            )));
        }
        Ok(NonNull::new_unchecked(ptr.cast()))
    }
}

/// Reserve a span of address space covering `[addr, addr + size)`, rounded
/// down to the allocation granularity.  Commit sub-ranges with [`commit_at`].
pub fn reserve_at(addr: u64, size: usize) -> Result<Reservation, EmuError> {
    const GRANULARITY: u64 = 0x1_0000;
    unsafe {
        let base = addr & !(GRANULARITY - 1);
        let end = addr + size as u64;
        let end_rounded = (end + GRANULARITY - 1) & !(GRANULARITY - 1);
        let span = end_rounded.saturating_sub(base);
        let ptr = VirtualAlloc(
            base as *mut core::ffi::c_void,
            span as usize,
            MEM_RESERVE,
            PAGE_NOACCESS,
        );
        if ptr.is_null() {
            return Err(EmuError::Alloc(format!(
                "VirtualAlloc reserve({base:#x}, {span:#x}) failed — identity mapping unavailable (error {})",
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
pub fn commit_at(addr: u64, size: usize) -> Result<NonNull<u8>, EmuError> {
    unsafe {
        let ptr = VirtualAlloc(
            addr as *mut core::ffi::c_void,
            size,
            MEM_COMMIT,
            PAGE_READWRITE,
        );
        if ptr.is_null() {
            return Err(EmuError::Alloc(format!(
                "VirtualAlloc commit({addr:#x}, {size:#x}) failed (error {})",
                std::io::Error::last_os_error()
            )));
        }
        Ok(NonNull::new_unchecked(ptr.cast()))
    }
}

pub fn free_reservation(reservation: Reservation) {
    unsafe {
        VirtualFree(reservation.base.as_ptr().cast(), 0, MEM_RELEASE);
    }
}

pub fn protect(
    ptr: NonNull<u8>,
    size: usize,
    read: bool,
    write: bool,
    exec: bool,
) -> Result<(), EmuError> {
    let flags = if exec {
        PAGE_EXECUTE_READ
    } else if write {
        PAGE_READWRITE
    } else if read {
        PAGE_READONLY
    } else {
        PAGE_NOACCESS
    };
    let mut old = 0u32;
    let ok = unsafe { VirtualProtect(ptr.as_ptr().cast(), size, flags, &mut old) };
    if ok == 0 {
        return Err(EmuError::Alloc(format!(
            "VirtualProtect({:#x}, {size:#x}, flags {flags:#x}) failed (error {})",
            ptr.as_ptr() as usize,
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

pub fn free(ptr: NonNull<u8>, _size: usize) {
    unsafe {
        VirtualFree(ptr.as_ptr().cast(), 0, MEM_RELEASE);
    }
}
