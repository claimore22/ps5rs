use std::fmt;

/// ELF segment permission flags (read / write / execute).
///
/// Constructed from `p_flags` via [`from_p_flags`](Self::from_p_flags) and
/// rendered as a three-character string like `"R-X"` or `"RW-"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl SegmentFlags {
    /// Build flags from ELF `p_flags` bits (`PF_R=4`, `PF_W=2`, `PF_X=1`).
    pub fn from_p_flags(flags: u32) -> Self {
        Self {
            read: flags & 4 != 0,
            write: flags & 2 != 0,
            execute: flags & 1 != 0,
        }
    }

    pub fn is_readable(&self) -> bool {
        self.read
    }

    pub fn is_writable(&self) -> bool {
        self.write
    }

    pub fn is_executable(&self) -> bool {
        self.execute
    }
}

impl fmt::Display for SegmentFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = if self.read { 'R' } else { '-' };
        let w = if self.write { 'W' } else { '-' };
        let x = if self.execute { 'X' } else { '-' };
        write!(f, "{r}{w}{x}")
    }
}

/// A contiguous mapped region of memory inside a [`ProcessMemory`].
///
/// Uses ELF-centric naming — `vaddr` (not `address`) — and carries both the
/// file-offset and file-size for debugging.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Virtual address where the region is mapped (relative to the module base).
    pub vaddr: u64,
    /// Total size in bytes (includes zero-filled `.bss` after `file_size`).
    pub size: usize,
    /// Offset within the ELF file where segment data begins.
    pub file_offset: u64,
    /// Number of bytes of file data (the rest up to `size` is zero-filled `.bss`).
    pub file_size: usize,
    /// Segment permissions (read / write / execute).
    pub permissions: SegmentFlags,
    /// Raw memory contents (file data followed by zeros for `.bss`).
    pub data: Vec<u8>,
}

/// Classifies the kind of a [`MemoryError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryErrorKind {
    UnmappedAddress,
    CrossRegionBoundary,
    OutOfBounds,
}

/// Error returned when a memory read or write cannot be completed.
#[derive(Debug, Clone)]
pub struct MemoryError {
    pub kind: MemoryErrorKind,
    pub vaddr: u64,
    pub size: usize,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            MemoryErrorKind::UnmappedAddress => {
                write!(f, "address 0x{:x} is not mapped", self.vaddr)
            }
            MemoryErrorKind::CrossRegionBoundary => {
                write!(
                    f,
                    "access at 0x{:x} size {} crosses a region boundary",
                    self.vaddr, self.size
                )
            }
            MemoryErrorKind::OutOfBounds => {
                write!(
                    f,
                    "access at 0x{:x} size {} exceeds region bounds",
                    self.vaddr, self.size
                )
            }
        }
    }
}

impl std::error::Error for MemoryError {}

/// Virtual address space composed of ordered non-overlapping [`MemoryRegion`]s.
///
/// Provides safe read / write access with boundary checking: reads return
/// `&[u8]` slices, writes reject cross-region accesses.
#[derive(Debug, Clone)]
pub struct ProcessMemory {
    pub regions: Vec<MemoryRegion>,
}

impl ProcessMemory {
    pub fn new(regions: Vec<MemoryRegion>) -> Self {
        Self { regions }
    }

    /// Find the region containing `vaddr`, if any.
    pub fn find_region(&self, vaddr: u64) -> Option<&MemoryRegion> {
        self.regions
            .iter()
            .find(|r| r.vaddr <= vaddr && vaddr < r.vaddr + r.size as u64)
    }

    pub fn find_region_mut(&mut self, vaddr: u64) -> Option<&mut MemoryRegion> {
        self.regions
            .iter_mut()
            .find(|r| r.vaddr <= vaddr && vaddr < r.vaddr + r.size as u64)
    }

    /// Read `size` bytes starting at `vaddr`.
    ///
    /// Returns an error if `vaddr` is unmapped or the read would cross a region
    /// boundary.
    pub fn read(&self, vaddr: u64, size: usize) -> Result<&[u8], MemoryError> {
        let region = self.find_region(vaddr).ok_or(MemoryError {
            kind: MemoryErrorKind::UnmappedAddress,
            vaddr,
            size,
        })?;
        let offset = (vaddr - region.vaddr) as usize;
        let end = vaddr.checked_add(size as u64).ok_or(MemoryError {
            kind: MemoryErrorKind::OutOfBounds,
            vaddr,
            size,
        })?;
        if end > region.vaddr + region.size as u64 {
            return Err(MemoryError {
                kind: MemoryErrorKind::CrossRegionBoundary,
                vaddr,
                size,
            });
        }
        Ok(&region.data[offset..offset + size])
    }

    /// Write `data` at `vaddr`.
    ///
    /// Returns an error if `vaddr` is unmapped or the write would cross a region
    /// boundary.  Empty writes are a no-op.
    pub fn write(&mut self, vaddr: u64, data: &[u8]) -> Result<(), MemoryError> {
        let size = data.len();
        if size == 0 {
            return Ok(());
        }
        let end = vaddr.checked_add(size as u64).ok_or(MemoryError {
            kind: MemoryErrorKind::OutOfBounds,
            vaddr,
            size,
        })?;

        let idx = self
            .regions
            .iter()
            .position(|r| r.vaddr <= vaddr && vaddr < r.vaddr + r.size as u64)
            .ok_or(MemoryError {
                kind: MemoryErrorKind::UnmappedAddress,
                vaddr,
                size,
            })?;

        let region = &self.regions[idx];
        let offset = (vaddr - region.vaddr) as usize;

        if end > region.vaddr + region.size as u64 {
            return Err(MemoryError {
                kind: MemoryErrorKind::CrossRegionBoundary,
                vaddr,
                size,
            });
        }

        self.regions[idx].data[offset..offset + size].copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(vaddr: u64, size: usize, perms: SegmentFlags) -> MemoryRegion {
        MemoryRegion {
            vaddr,
            size,
            file_offset: vaddr,
            file_size: size,
            permissions: perms,
            data: vec![0xAA; size],
        }
    }

    #[test]
    fn from_p_flags_rx() {
        let f = SegmentFlags::from_p_flags(5);
        assert!(f.read);
        assert!(!f.write);
        assert!(f.execute);
    }

    #[test]
    fn from_p_flags_rw() {
        let f = SegmentFlags::from_p_flags(6);
        assert!(f.read);
        assert!(f.write);
        assert!(!f.execute);
    }

    #[test]
    fn from_p_flags_rwx() {
        let f = SegmentFlags::from_p_flags(7);
        assert!(f.read);
        assert!(f.write);
        assert!(f.execute);
    }

    #[test]
    fn from_p_flags_none() {
        let f = SegmentFlags::from_p_flags(0);
        assert!(!f.read);
        assert!(!f.write);
        assert!(!f.execute);
    }

    #[test]
    fn find_region_exact_match() {
        let pm = ProcessMemory::new(vec![
            make_region(0x800000000, 0x1000, SegmentFlags::from_p_flags(5)),
            make_region(0x800001000, 0x2000, SegmentFlags::from_p_flags(6)),
        ]);
        let r = pm.find_region(0x800000000).unwrap();
        assert_eq!(r.vaddr, 0x800000000);
    }

    #[test]
    fn find_region_inside() {
        let pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x1000,
            SegmentFlags::from_p_flags(5),
        )]);
        let r = pm.find_region(0x800000500).unwrap();
        assert_eq!(r.vaddr, 0x800000000);
    }

    #[test]
    fn find_region_unmapped() {
        let pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x1000,
            SegmentFlags::from_p_flags(5),
        )]);
        assert!(pm.find_region(0x800001000).is_none());
    }

    #[test]
    fn read_within_region() {
        let data = vec![0xAB; 0x100];
        let pm = ProcessMemory::new(vec![MemoryRegion {
            vaddr: 0x800000000,
            size: 0x100,
            file_offset: 0,
            file_size: 0x100,
            permissions: SegmentFlags::from_p_flags(4),
            data,
        }]);
        let bytes = pm.read(0x800000010, 0x10).unwrap();
        assert_eq!(bytes, &[0xAB; 0x10]);
    }

    #[test]
    fn read_unmapped_address() {
        let pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x100,
            SegmentFlags::from_p_flags(4),
        )]);
        let err = pm.read(0x800000200, 1).unwrap_err();
        assert_eq!(err.kind, MemoryErrorKind::UnmappedAddress);
    }

    #[test]
    fn read_crosses_region_boundary() {
        let pm = ProcessMemory::new(vec![
            make_region(0x800000000, 0x100, SegmentFlags::from_p_flags(4)),
            make_region(0x800000100, 0x100, SegmentFlags::from_p_flags(4)),
        ]);
        let err = pm.read(0x8000000f0, 0x20).unwrap_err();
        assert_eq!(err.kind, MemoryErrorKind::CrossRegionBoundary);
    }

    #[test]
    fn write_within_region() {
        let mut pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x100,
            SegmentFlags::from_p_flags(6),
        )]);
        pm.write(0x800000010, &[0xFF; 0x10]).unwrap();
        let bytes = pm.read(0x800000010, 0x10).unwrap();
        assert_eq!(bytes, &[0xFF; 0x10]);
    }

    #[test]
    fn write_unmapped_address() {
        let mut pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x100,
            SegmentFlags::from_p_flags(6),
        )]);
        let err = pm.write(0x800000200, &[0xFF]).unwrap_err();
        assert_eq!(err.kind, MemoryErrorKind::UnmappedAddress);
    }

    #[test]
    fn write_crosses_region_boundary() {
        let mut pm = ProcessMemory::new(vec![
            make_region(0x800000000, 0x100, SegmentFlags::from_p_flags(6)),
            make_region(0x800000100, 0x100, SegmentFlags::from_p_flags(6)),
        ]);
        let err = pm.write(0x8000000f0, &[0xFF; 0x20]).unwrap_err();
        assert_eq!(err.kind, MemoryErrorKind::CrossRegionBoundary);
    }

    #[test]
    fn write_empty_is_noop() {
        let mut pm = ProcessMemory::new(vec![make_region(
            0x800000000,
            0x100,
            SegmentFlags::from_p_flags(6),
        )]);
        pm.write(0x800000000, &[]).unwrap();
        let bytes = pm.read(0x800000000, 0x10).unwrap();
        assert_eq!(bytes, &[0xAA; 0x10]);
    }

    #[test]
    fn segment_flags_display() {
        assert_eq!(SegmentFlags::from_p_flags(7).to_string(), "RWX");
        assert_eq!(SegmentFlags::from_p_flags(5).to_string(), "R-X");
        assert_eq!(SegmentFlags::from_p_flags(4).to_string(), "R--");
        assert_eq!(SegmentFlags::from_p_flags(0).to_string(), "---");
    }
}
