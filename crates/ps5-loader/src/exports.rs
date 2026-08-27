use std::collections::HashMap;

use crate::mapper::LoadedModule;
use crate::nid::NidResolver;
use ps5_elf::ElfImage;

/// A single export entry — a symbol defined by a loaded module.
#[derive(Debug, Clone)]
pub struct ExportEntry {
    /// Runtime address of the exported symbol (load_bias + st_value).
    pub address: u64,
    /// Library tag (e.g. "libkernel").
    pub library: String,
    /// Human-readable name, if resolvable.
    pub name: Option<String>,
    /// Canonical name of the module that exports this symbol.
    pub module_name: String,
}

/// The process-level export table, keyed by numeric NID.
///
/// Built by iterating each [`LoadedModule`]'s defined (non-import) symbols
/// and mapping NID → runtime address.
#[derive(Debug, Clone, Default)]
pub struct ExportTable {
    exports: HashMap<u64, ExportEntry>,
}

impl ExportTable {
    pub fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }

    /// Register exports using the default [`SymbolNidResolver`](crate::nid::SymbolNidResolver).
    ///
    /// See [`register_module_with`](Self::register_module_with) for the full version
    /// that accepts a custom [`NidResolver`].
    pub fn register_module(&mut self, module: &LoadedModule, elf: &ElfImage) {
        self.register_module_with(module, elf, &crate::nid::SymbolNidResolver)
    }

    /// Register exports from a module's ELF symbol table.
    ///
    /// Symbols where `!is_import && st_value != 0` are treated as exports.
    /// The NID is resolved via [`NidResolver::resolve`] — supporting both
    /// `#NID` format (e.g. `J6h9iA2kL7M#libkernel`) and readable SCE names
    /// (e.g. `sceKernelGetModuleInfoFromAddr`).
    ///
    /// The runtime address is `module.load_bias + sym.st_value`.
    pub fn register_module_with(
        &mut self,
        module: &LoadedModule,
        elf: &ElfImage,
        resolver: &dyn NidResolver,
    ) {
        for sym in &elf.symbols {
            if sym.is_import || sym.st_value == 0 {
                continue;
            }
            let Some(nid) = resolver.resolve(&sym.resolved_name) else {
                continue;
            };
            let address = module.load_bias.wrapping_add(sym.st_value);
            let (library, name) =
                if let Some((name_part, lib_part)) = sym.resolved_name.split_once('#') {
                    (lib_part.to_string(), Some(name_part.to_string()))
                } else {
                    (String::new(), Some(sym.resolved_name.clone()))
                };
            self.exports.insert(
                nid,
                ExportEntry {
                    address,
                    library,
                    name,
                    module_name: module.canonical_name().to_string(),
                },
            );
        }
    }

    /// Look up an export by numeric NID.
    pub fn get_by_nid(&self, nid: u64) -> Option<&ExportEntry> {
        self.exports.get(&nid)
    }

    /// Total number of registered exports.
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }

    /// Iterate all exports.
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &ExportEntry)> {
        self.exports.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapper::{ModuleNameSource, ModuleType};
    use crate::memory::{MemoryRegion, ProcessMemory, SegmentFlags};

    fn make_module_with_bias(name: &str, load_bias: u64) -> LoadedModule {
        LoadedModule {
            name: name.to_string(),
            name_source: ModuleNameSource::Filename,
            module_type: ModuleType::Prx,
            memory: ProcessMemory::new(vec![MemoryRegion {
                vaddr: 0,
                size: 0x1000,
                file_offset: 0,
                file_size: 0x1000,
                permissions: SegmentFlags::from_p_flags(6),
                data: vec![0; 0x1000],
            }]),
            preferred_base: 0,
            load_bias,
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
        }
    }

    fn make_sym(name: &str, st_value: u64, shndx: u16) -> ps5_elf::SymEntry {
        let is_import = shndx == 0 && st_value == 0 && name.contains('#');
        ps5_elf::SymEntry {
            st_name: 0,
            st_info: 0,
            st_other: 0,
            st_shndx: shndx,
            st_value,
            st_size: 0,
            resolved_name: name.to_string(),
            is_import,
        }
    }

    fn make_elf(symbols: Vec<ps5_elf::SymEntry>) -> ps5_elf::ElfImage<'static> {
        ps5_elf::ElfImage {
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
            symbols,
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
        }
    }

    #[test]
    fn export_from_defined_symbol() {
        let module = make_module_with_bias("libtest.prx", 0x1000000);
        let elf = make_elf(vec![make_sym("J6h9iA2kL7M#libtest", 0x500, 1)]);
        let mut table = ExportTable::new();
        table.register_module(&module, &elf);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn export_from_readable_name() {
        let module = make_module_with_bias("libtest.prx", 0x1000000);
        let elf = make_elf(vec![make_sym("memcpy", 0x500, 1)]);
        let mut table = ExportTable::new();
        table.register_module(&module, &elf);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn import_symbol_not_exported() {
        let module = make_module_with_bias("libtest.prx", 0);
        let elf = make_elf(vec![make_sym("sceKernelSleep#libkernel", 0, 0)]);
        let mut table = ExportTable::new();
        table.register_module(&module, &elf);
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn export_address_includes_load_bias() {
        let module = make_module_with_bias("libtest.prx", 0x800000000);
        let elf = make_elf(vec![make_sym("J6h9iA2kL7M#libtest", 0x1000, 1)]);
        let mut table = ExportTable::new();
        table.register_module(&module, &elf);
        let nid = crate::nid::nid_to_u64("J6h9iA2kL7M").unwrap();
        let entry = table.get_by_nid(nid).unwrap();
        assert_eq!(entry.address, 0x800001000);
    }

    #[test]
    fn get_by_nid_returns_none_for_unknown() {
        let table = ExportTable::new();
        assert!(table.get_by_nid(999).is_none());
    }

    #[test]
    fn empty_table_is_empty() {
        let table = ExportTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn export_has_module_name() {
        let module = make_module_with_bias("libSceFoo.prx", 0);
        let elf = make_elf(vec![make_sym("A1b2C3d4E5F#libSceFoo", 0x200, 1)]);
        let mut table = ExportTable::new();
        table.register_module(&module, &elf);
        let nid = crate::nid::nid_to_u64("A1b2C3d4E5F").unwrap();
        let entry = table.get_by_nid(nid).unwrap();
        assert_eq!(entry.module_name, "libSceFoo.prx");
    }
}
