use std::collections::HashMap;

use crate::exports::ExportTable;
use crate::imports::{ImportError, ImportRequest, ImportResolver, ResolveResult, StubAllocator};
use crate::mapper::LibraryImportCounts;
use crate::offline::OfflineExportTable;

/// An [`ImportResolver`] that checks three sources in order:
///
/// 1. **Runtime exports** — modules actually loaded in the current process.
/// 2. **Offline exports** — system PRX exports indexed from `system_modules/*.exports.json`.
/// 3. **Stub allocator** — synthetic addresses for truly unknown imports.
pub struct CrossModuleResolver<'a> {
    exports: &'a ExportTable,
    offline: Option<&'a OfflineExportTable>,
    stubs: &'a mut StubAllocator,
    resolved_count: u32,
    known_count: u32,
    stubbed_count: u32,
    per_library: HashMap<String, [u32; 3]>,
}

impl<'a> CrossModuleResolver<'a> {
    pub fn new(
        exports: &'a ExportTable,
        offline: Option<&'a OfflineExportTable>,
        stubs: &'a mut StubAllocator,
    ) -> Self {
        Self {
            exports,
            offline,
            stubs,
            resolved_count: 0,
            known_count: 0,
            stubbed_count: 0,
            per_library: HashMap::new(),
        }
    }

    pub fn resolved_count(&self) -> u32 {
        self.resolved_count
    }

    pub fn known_count(&self) -> u32 {
        self.known_count
    }

    pub fn stubbed_count(&self) -> u32 {
        self.stubbed_count
    }

    pub fn per_library_imports(&self) -> Vec<LibraryImportCounts> {
        let mut result: Vec<LibraryImportCounts> = self
            .per_library
            .iter()
            .map(|(lib, counts)| LibraryImportCounts {
                library: lib.clone(),
                resolved: counts[0],
                known: counts[1],
                stubbed: counts[2],
            })
            .collect();
        result.sort_by(|a, b| a.library.cmp(&b.library));
        result
    }
}

impl ImportResolver for CrossModuleResolver<'_> {
    fn resolve(&mut self, request: &ImportRequest) -> Result<ResolveResult, ImportError> {
        let lib = request.library.as_deref().unwrap_or("?");
        // 1. Runtime exports (modules actually loaded)
        if let Some(nid) = request.nid
            && let Some(entry) = self.exports.get_by_nid(nid)
        {
            self.resolved_count += 1;
            let e = self.per_library.entry(lib.to_string()).or_insert([0, 0, 0]);
            e[0] += 1;
            return Ok(ResolveResult::Resolved(entry.address));
        }
        // 2. Offline exports (known system functions, not loaded)
        if let Some(nid) = request.nid
            && let Some(offline) = self.offline
            && offline.get_by_nid(nid).is_some()
        {
            self.known_count += 1;
            let e = self.per_library.entry(lib.to_string()).or_insert([0, 0, 0]);
            e[1] += 1;
            let addr = self.stubs.resolve(request)?.address();
            return Ok(ResolveResult::Known(addr));
        }
        // 3. Fallback to stub
        let result = self.stubs.resolve(request)?;
        let e = self.per_library.entry(lib.to_string()).or_insert([0, 0, 0]);
        e[2] += 1;
        self.stubbed_count += 1;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::STUB_REGION_BASE;

    fn test_nid(s: &str) -> u64 {
        const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
        let mut val: u64 = 0;
        for &c in s.as_bytes() {
            let idx = B64.iter().position(|&b| b == c).unwrap();
            val = val.wrapping_mul(64).wrapping_add(idx as u64);
        }
        val
    }

    fn make_request(nid: Option<u64>, name: Option<&str>) -> ImportRequest {
        ImportRequest {
            symbol_index: 0,
            nid,
            library: None,
            name: name.map(|s| s.to_string()),
        }
    }

    #[test]
    fn stubs_unknown_nid() {
        let exports = ExportTable::new();
        let mut stubs = StubAllocator::new(STUB_REGION_BASE);
        let mut resolver = CrossModuleResolver::new(&exports, None, &mut stubs);
        let request = make_request(Some(999), Some("unknown"));
        let result = resolver.resolve(&request).unwrap();
        match result {
            ResolveResult::Stubbed(_) => {}
            _ => panic!("expected stubbed"),
        }
        assert_eq!(resolver.stubbed_count(), 1);
        assert_eq!(resolver.resolved_count(), 0);
    }

    #[test]
    fn counts_tracked_correctly() {
        let exports = ExportTable::new();
        let mut stubs = StubAllocator::new(STUB_REGION_BASE);
        let mut resolver = CrossModuleResolver::new(&exports, None, &mut stubs);
        let r1 = make_request(Some(1), Some("a"));
        let r2 = make_request(Some(2), Some("b"));
        let _ = resolver.resolve(&r1);
        let _ = resolver.resolve(&r2);
        assert_eq!(resolver.stubbed_count(), 2);
        assert_eq!(resolver.resolved_count(), 0);
    }

    #[test]
    fn mixed_resolved_and_stubbed() {
        let mut et = ExportTable::new();

        // Build a minimal export via register_module
        let module = crate::mapper::LoadedModule {
            name: "libtest.prx".to_string(),
            name_source: crate::mapper::ModuleNameSource::Filename,
            module_type: crate::mapper::ModuleType::Prx,
            memory: crate::memory::ProcessMemory::new(vec![crate::memory::MemoryRegion {
                vaddr: 0,
                size: 0x1000,
                file_offset: 0,
                file_size: 0x1000,
                permissions: crate::memory::SegmentFlags::from_p_flags(6),
                data: vec![0; 0x1000],
            }]),
            preferred_base: 0,
            load_bias: 0,
            entry_point: None,
            imports: Vec::new(),
            relocations: Vec::new(),
            relocation_summary: None,
            soname: None,
            aliases: Vec::new(),
            state: crate::mapper::ModuleState::Mapped,
            tls: None,
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            exports_count: 0,
            imports_resolved: 0,
            imports_known: 0,
            imports_stubbed: 0,
            per_library_imports: Vec::new(),
            prx_module: None,
        };
        let elf = ps5_elf::ElfImage {
            data: &[],
            elf_base: 0,
            header: ps5_elf::ElfHeader {
                class: 2,
                endian: 1,
                ei_version: 1,
                ei_osabi: 0,
                ei_abi_version: 0,
                e_type: 0xFE10,
                e_machine: 62,
                e_version: 1,
                e_entry: 0,
                e_phoff: 0,
                e_shoff: 0,
                e_flags: 0,
                e_ehsize: 64,
                phentsize: 56,
                phnum: 0,
                shentsize: 0,
                shnum: 0,
                shstrndx: 0,
            },
            program_headers: vec![],
            section_headers: vec![],
            dynamic_entries: vec![],
            relocations: vec![],
            symbols: vec![ps5_elf::SymEntry {
                st_name: 0,
                st_info: 0,
                st_other: 0,
                st_shndx: 1,
                st_value: 0x500,
                st_size: 0,
                resolved_name: "J6h9iA2kL7M#libtest".to_string(),
                is_import: false,
            }],
            tls: None,
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            strtab_offset: 0,
            strtab_size: 0,
            symtab_offset: 0,
            symtab_size: 0,
            soname: None,
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            lib_versions: vec![],
            rela_count: 0,
        };
        et.register_module(&module, &elf);

        let mut stubs = StubAllocator::new(STUB_REGION_BASE);
        let mut resolver = CrossModuleResolver::new(&et, None, &mut stubs);

        let nid = test_nid("J6h9iA2kL7M");
        let r1 = make_request(Some(nid), Some("known"));
        let result = resolver.resolve(&r1).unwrap();
        match result {
            ResolveResult::Resolved(addr) => assert_eq!(addr, 0x500),
            _ => panic!("expected resolved"),
        }

        let r2 = make_request(Some(999), Some("unknown"));
        let _ = resolver.resolve(&r2).unwrap();

        assert_eq!(resolver.resolved_count(), 1);
        assert_eq!(resolver.stubbed_count(), 1);
    }
}
