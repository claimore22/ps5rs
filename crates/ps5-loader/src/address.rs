/// A sequential address allocator for assigning base addresses to loaded modules.
///
/// Default starting point: `0x810000000` (eboot sits at `0x800000000` under
/// PS5's standard image base).  Each allocation advances by the module's
/// size rounded up to the page granularity.
#[derive(Debug, Clone)]
pub struct LoadAddressAllocator {
    next: u64,
    page_size: u64,
}

impl LoadAddressAllocator {
    pub fn new(start: u64, page_size: u64) -> Self {
        Self {
            next: start,
            page_size,
        }
    }

    /// Allocate a base address for a module of `size` bytes.
    ///
    /// Returns the current cursor, then advances by `size` rounded up to
    /// `page_size`.
    pub fn allocate(&mut self, size: u64) -> u64 {
        let addr = self.next;
        self.next = self
            .next
            .checked_add(align_up(size, self.page_size))
            .expect("address space exhausted");
        addr
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    if align <= 1 {
        return value;
    }
    (value + align - 1) & !(align - 1)
}

impl Default for LoadAddressAllocator {
    fn default() -> Self {
        Self::new(0x810000000, 0x10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_sequential_addresses() {
        let mut alloc = LoadAddressAllocator::new(0x1000, 0x1000);
        assert_eq!(alloc.allocate(0x100), 0x1000);
        assert_eq!(alloc.allocate(0x200), 0x2000);
        assert_eq!(alloc.allocate(0x1000), 0x3000);
    }

    #[test]
    fn align_up_rounds_to_page() {
        let mut alloc = LoadAddressAllocator::new(0, 0x10000);
        assert_eq!(alloc.allocate(1), 0);
        assert_eq!(alloc.allocate(0x10000), 0x10000);
        assert_eq!(alloc.allocate(0x10001), 0x20000);
    }

    #[test]
    fn default_starts_at_810000000() {
        let mut alloc = LoadAddressAllocator::default();
        assert_eq!(alloc.allocate(0), 0x810000000);
    }

    #[test]
    fn zero_page_size_no_align() {
        let mut alloc = LoadAddressAllocator::new(0x1000, 1);
        assert_eq!(alloc.allocate(0x100), 0x1000);
        assert_eq!(alloc.allocate(0x200), 0x1100);
    }
}
