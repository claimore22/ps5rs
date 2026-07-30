use crate::imports::{ImportRequest, ImportResolver, ResolveResult};
use crate::mapper::LoadedModule;
use crate::nid::{NidResolver, SymbolNidResolver};

/// Classification of an x86-64 ELF relocation type used by PS5 binaries.
///
/// Phase 1 handles [`Relative`](Self::Relative).  Phase 2 handles
/// [`GlobDat`](Self::GlobDat) and [`JumpSlot`](Self::JumpSlot) via
/// an [`ImportResolver`].  Remaining types are counted but not applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelocationKind {
    Relative,
    GlobDat,
    JumpSlot,
    Abs64,
    Copy,
    Tls,
    Ifunc,
    Unknown(u32),
}

impl RelocationKind {
    /// Convert an ELF `r_type` constant to a `RelocationKind`.
    ///
    /// Handles the x86-64 relocation types found in PS5 binaries:
    ///
    /// | Value | Name           | Kind      |
    /// |-------|----------------|-----------|
    /// | 1     | R_X86_64_64    | `Abs64`   |
    /// | 5     | R_X86_64_COPY  | `Copy`    |
    /// | 6     | R_X86_64_GLOB_DAT | `GlobDat` |
    /// | 7     | R_X86_64_JUMP_SLOT | `JumpSlot` |
    /// | 8     | R_X86_64_RELATIVE | `Relative` |
    /// | 16-18 | DTPMOD/DTPOFF/TPOFF | `Tls` |
    pub fn from_type(r_type: u32) -> Self {
        match r_type {
            1 => Self::Abs64,
            5 => Self::Copy,
            6 => Self::GlobDat,
            7 => Self::JumpSlot,
            8 => Self::Relative,
            16 | 17 | 18 => Self::Tls,
            _ => Self::Unknown(r_type),
        }
    }

    /// Human-readable name (e.g. `"RELATIVE"`, `"GLOB_DAT"`).
    pub fn name(&self) -> &'static str {
        match self {
            Self::Relative => "RELATIVE",
            Self::GlobDat => "GLOB_DAT",
            Self::JumpSlot => "JUMP_SLOT",
            Self::Abs64 => "ABS64",
            Self::Copy => "COPY",
            Self::Tls => "TLS",
            Self::Ifunc => "IFUNC",
            Self::Unknown(_) => "Unknown",
        }
    }
}

/// Record of a single relocation operation applied to a [`LoadedModule`].
#[derive(Debug, Clone)]
pub struct RelocationRecord {
    /// Target virtual address (already adjusted by `load_bias`).
    pub address: u64,
    /// Classified relocation type.
    pub kind: RelocationKind,
    /// Whether the relocation was actually patched into memory.
    pub applied: bool,
}

/// Aggregate statistics from a relocation pass.
///
/// Tracks per-type counts plus import resolution outcomes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelocationSummary {
    pub relative: u32,
    pub relative_fast_path: u32,
    pub glob_dat: u32,
    pub jump_slot: u32,
    pub resolved_imports: u32,
    pub known_imports: u32,
    pub stubbed_imports: u32,
    pub missing_imports: u32,
    pub abs64: u32,
    pub copy: u32,
    pub tls: u32,
    pub ifunc: u32,
    pub unknown: u32,
}

#[derive(Debug)]
pub struct RelocationError(pub String);

impl std::fmt::Display for RelocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RelocationError {}

pub type Result<T> = std::result::Result<T, RelocationError>;

/// Apply `R_X86_64_RELATIVE` relocations without an import resolver.
///
/// Convenience wrapper around [`apply_relocations_with(module, elf, None)`].
pub fn apply_relocations(
    module: &mut LoadedModule,
    elf: &ps5_elf::ElfImage,
) -> Result<RelocationSummary> {
    apply_relocations_with(module, elf, None)
}

/// Build an [`ImportRequest`] from a relocation's ELF symbol.
///
/// Shared by [`GLOB_DAT`](RelocationKind::GlobDat),
/// [`JUMP_SLOT`](RelocationKind::JumpSlot), and [`ABS64`](RelocationKind::Abs64)
/// so that import-request construction stays in one place.
fn build_import_request(
    elf: &ps5_elf::ElfImage,
    reloc: &ps5_elf::RelaEntry,
) -> Result<ImportRequest> {
    let sym = elf
        .symbols
        .get(reloc.r_sym() as usize)
        .ok_or_else(|| RelocationError(format!("symbol index {} not found", reloc.r_sym())))?;

    let (library, import_name) =
        if let Some((name_part, lib_part)) = sym.resolved_name.split_once('#') {
            (Some(lib_part.to_string()), Some(name_part.to_string()))
        } else {
            (None, Some(sym.resolved_name.clone()))
        };

    let nid = SymbolNidResolver.resolve(&sym.resolved_name);

    Ok(ImportRequest {
        symbol_index: reloc.r_sym(),
        nid,
        library,
        name: import_name,
    })
}

/// Apply relocations with an optional [`ImportResolver`].
///
/// # Phases
///
/// 1. **Fast path** — the first `rela_count` entries (from `DT_RELACOUNT`)
///    are applied as `R_X86_64_RELATIVE` without per-entry type dispatch.
///    In debug builds each entry is verified with `debug_assert_eq!`.
///
/// 2. **Slow path** — remaining relocations are classified individually.
///    `RELATIVE` entries are applied directly.  `GLOB_DAT` and `JUMP_SLOT`
///    are resolved via the resolver if one is provided, or counted as missing.
///    All other types (`ABS64`, `COPY`, `TLS`, `IFUNC`, `Unknown`) are counted
///    and left unpatched.
///
/// # Errors
///
/// Returns [`RelocationError`] if a memory write fails or if the resolver
/// encounters a problem.
pub fn apply_relocations_with(
    module: &mut LoadedModule,
    elf: &ps5_elf::ElfImage,
    mut resolver: Option<&mut dyn ImportResolver>,
) -> Result<RelocationSummary> {
    let total = elf.relocations.len();
    let rela_count = usize::min(elf.rela_count as usize, total);

    tracing::info!(
        total,
        rela_count,
        has_resolver = resolver.is_some(),
        "apply_relocations_with: start",
    );

    let mut records = Vec::with_capacity(total);
    let mut summary = RelocationSummary::default();

    // Fast path: DT_RELACOUNT — linker guarantees these are all R_X86_64_RELATIVE.
    for reloc in &elf.relocations[..rela_count] {
        debug_assert_eq!(reloc.r_type(), 8);
        let address = module.load_bias.wrapping_add(reloc.r_offset);
        let value = module.load_bias.wrapping_add(reloc.r_addend as u64);
        module
            .memory
            .write(address, &value.to_le_bytes())
            .map_err(|e| {
                RelocationError(format!("RELATIVE write failed at 0x{:x}: {}", address, e))
            })?;
        records.push(RelocationRecord {
            address,
            kind: RelocationKind::Relative,
            applied: true,
        });
        summary.relative += 1;
        summary.relative_fast_path += 1;
    }

    // Slow path: individual type dispatch.
    for reloc in &elf.relocations[rela_count..] {
        let kind = RelocationKind::from_type(reloc.r_type());
        let address = module.load_bias.wrapping_add(reloc.r_offset);
        let mut applied = false;

        if kind == RelocationKind::Relative {
            let value = module.load_bias.wrapping_add(reloc.r_addend as u64);
            module
                .memory
                .write(address, &value.to_le_bytes())
                .map_err(|e| {
                    RelocationError(format!("RELATIVE write failed at 0x{:x}: {}", address, e))
                })?;
            applied = true;
        } else if kind == RelocationKind::Abs64 {
            if reloc.r_sym() != 0 {
                if let Some(sym) = elf.symbols.get(reloc.r_sym() as usize) {
                    if !sym.is_import && sym.st_value != 0 {
                        // R_X86_64_64: *(S + A) = S + A, S = load_bias + st_value
                        let s = module.load_bias.wrapping_add(sym.st_value);
                        let value = s.wrapping_add(reloc.r_addend as u64);
                        module
                            .memory
                            .write(address, &value.to_le_bytes())
                            .map_err(|e| {
                                RelocationError(format!(
                                    "ABS64 write failed at 0x{:x}: {}",
                                    address, e
                                ))
                            })?;
                        applied = true;
                        tracing::debug!(
                            address,
                            value,
                            symbol = %sym.resolved_name,
                            "ABS64 applied (local)",
                        );
                    } else if let Some(ref mut res) = resolver {
                        let request = build_import_request(elf, reloc)?;
                        let result = res.resolve(&request).map_err(|e| {
                            RelocationError(format!("ABS64 import resolve failed: {e}"))
                        })?;
                        let value = result.address();
                        module
                            .memory
                            .write(address, &value.to_le_bytes())
                            .map_err(|e| {
                                RelocationError(format!(
                                    "ABS64 import write at 0x{:x}: {}",
                                    address, e
                                ))
                            })?;
                        applied = true;
                        tracing::debug!(
                            address,
                            value,
                            symbol = %sym.resolved_name,
                            kind = ?result,
                            "ABS64 applied (import)",
                        );
                        match result {
                            ResolveResult::Resolved(_) => summary.resolved_imports += 1,
                            ResolveResult::Known(_) => summary.known_imports += 1,
                            ResolveResult::Stubbed(_) => summary.stubbed_imports += 1,
                        }
                    }
                }
            }
        } else if let Some(ref mut res) = resolver {
            if kind == RelocationKind::GlobDat || kind == RelocationKind::JumpSlot {
                let request = build_import_request(elf, reloc)?;
                let result = res
                    .resolve(&request)
                    .map_err(|e| RelocationError(format!("import resolve failed: {e}")))?;
                let value = result.address();
                module
                    .memory
                    .write(address, &value.to_le_bytes())
                    .map_err(|e| {
                        RelocationError(format!("import write at 0x{:x}: {}", address, e))
                    })?;
                applied = true;
                tracing::debug!(
                    address,
                    value,
                    kind = ?result,
                    "import resolved",
                );
                match result {
                    ResolveResult::Resolved(_) => summary.resolved_imports += 1,
                    ResolveResult::Known(_) => summary.known_imports += 1,
                    ResolveResult::Stubbed(_) => summary.stubbed_imports += 1,
                }
            }
        }

        match kind {
            RelocationKind::Relative => summary.relative += 1,
            RelocationKind::GlobDat => {
                summary.glob_dat += 1;
                if resolver.is_none() {
                    summary.missing_imports += 1;
                }
            }
            RelocationKind::JumpSlot => {
                summary.jump_slot += 1;
                if resolver.is_none() {
                    summary.missing_imports += 1;
                }
            }
            RelocationKind::Abs64 => {
                summary.abs64 += 1;
                tracing::warn!(address, "ABS64 skipped — requires symbol resolution");
            }
            RelocationKind::Copy => {
                summary.copy += 1;
                tracing::trace!(address, "COPY skipped");
            }
            RelocationKind::Tls => {
                summary.tls += 1;
                tracing::trace!(address, "TLS skipped");
            }
            RelocationKind::Ifunc => {
                summary.ifunc += 1;
                tracing::trace!(address, "IFUNC skipped");
            }
            RelocationKind::Unknown(raw) => {
                summary.unknown += 1;
                tracing::warn!(address, raw, "unknown relocation type");
            }
        }

        records.push(RelocationRecord {
            address,
            kind,
            applied,
        });
    }

    module.relocations = records;
    module.relocation_summary = Some(summary.clone());

    tracing::info!(
        relative = summary.relative,
        fast_path = summary.relative_fast_path,
        stubbed = summary.stubbed_imports,
        missing = summary.missing_imports,
        skipped_abs64 = summary.abs64,
        "apply_relocations_with: complete",
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StubAllocator;

    fn make_module(load_bias: u64, vaddr: u64, size: usize, val: u8) -> LoadedModule {
        LoadedModule {
            name: "test".into(),
            name_source: crate::mapper::ModuleNameSource::Filename,
            module_type: crate::mapper::ModuleType::Eboot,
            memory: crate::memory::ProcessMemory::new(vec![crate::memory::MemoryRegion {
                vaddr,
                size,
                file_offset: vaddr,
                file_size: size,
                permissions: crate::memory::SegmentFlags::from_p_flags(6),
                data: vec![val; size],
            }]),
            preferred_base: vaddr,
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
        }
    }

    fn make_elf(relocs: Vec<ps5_elf::RelaEntry>) -> ps5_elf::ElfImage<'static> {
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
            relocations: relocs,
            symbols: vec![],
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

    fn make_elf_with_rela_count(
        relocs: Vec<ps5_elf::RelaEntry>,
        rela_count: u64,
    ) -> ps5_elf::ElfImage<'static> {
        let mut elf = make_elf(relocs);
        elf.rela_count = rela_count;
        elf
    }

    fn make_reloc_with_sym(
        r_offset: u64,
        r_type: u32,
        r_sym: u32,
        r_addend: i64,
    ) -> ps5_elf::RelaEntry {
        ps5_elf::RelaEntry {
            r_offset,
            r_info: (r_sym as u64) << 32 | r_type as u64,
            r_addend,
            is_plt: false,
        }
    }

    fn make_elf_sym(
        relocs: Vec<ps5_elf::RelaEntry>,
        symbols: Vec<ps5_elf::SymEntry>,
    ) -> ps5_elf::ElfImage<'static> {
        let mut elf = make_elf(relocs);
        elf.symbols = symbols;
        elf
    }

    fn make_sym(name: &str) -> ps5_elf::SymEntry {
        ps5_elf::SymEntry {
            st_name: 0,
            st_info: 0,
            st_other: 0,
            st_shndx: 0,
            st_value: 0,
            st_size: 0,
            resolved_name: name.to_string(),
            is_import: name.contains('#'),
        }
    }

    fn make_local_sym(name: &str, st_value: u64) -> ps5_elf::SymEntry {
        ps5_elf::SymEntry {
            st_name: 0,
            st_info: 0,
            st_other: 0,
            st_shndx: 1,
            st_value,
            st_size: 0,
            resolved_name: name.to_string(),
            is_import: false,
        }
    }

    fn make_reloc(r_offset: u64, r_type: u32, r_addend: i64) -> ps5_elf::RelaEntry {
        ps5_elf::RelaEntry {
            r_offset,
            r_info: r_type as u64,
            r_addend,
            is_plt: false,
        }
    }

    #[test]
    fn apply_relative_patches_memory() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xAA);
        let elf = make_elf(vec![make_reloc(0x800000100, 8, 0xDEAD)]);

        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 1);
        assert_eq!(summary.glob_dat, 0);
        assert_eq!(summary.unknown, 0);

        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &0xDEADu64.to_le_bytes());

        assert_eq!(module.relocations.len(), 1);
        assert!(module.relocations[0].applied);
        assert_eq!(module.relocations[0].kind, RelocationKind::Relative);
        assert_eq!(module.relocations[0].address, 0x800000100);
    }

    #[test]
    fn apply_relative_with_addend_uses_base() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xAA);
        let elf = make_elf(vec![make_reloc(0x800000200, 8, 0x1234)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 1);
        let bytes = module.memory.read(0x800000200, 8).unwrap();
        assert_eq!(bytes, &0x1234u64.to_le_bytes());
    }

    #[test]
    fn abs64_is_skipped_not_applied() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xBB);
        let elf = make_elf(vec![make_reloc(0x800000100, 1, 0xCAFE)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.abs64, 1);
        assert_eq!(summary.relative, 0);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &[0xBB; 8]);
        assert_eq!(module.relocations.len(), 1);
        assert!(!module.relocations[0].applied);
        assert_eq!(module.relocations[0].kind, RelocationKind::Abs64);
    }

    fn make_null_sym() -> ps5_elf::SymEntry {
        ps5_elf::SymEntry {
            st_name: 0,
            st_info: 0,
            st_other: 0,
            st_shndx: 0,
            st_value: 0,
            st_size: 0,
            resolved_name: String::new(),
            is_import: false,
        }
    }

    #[test]
    fn abs64_local_defined_applies_patch() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 1, 1, 8)],
            vec![make_null_sym(), make_local_sym("g_data", 0x300)],
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.abs64, 1);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let val = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(val, 0x308);
        assert!(module.relocations[0].applied);
    }

    #[test]
    fn abs64_local_nonzero_load_bias() {
        let load_bias = 0x800000000u64;
        let pref_vaddr = 0x800000000u64;
        let actual_vaddr = pref_vaddr.wrapping_add(load_bias);
        let mut module = make_module(load_bias, actual_vaddr, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(pref_vaddr + 0x500, 1, 1, 0x10)],
            vec![make_null_sym(), make_local_sym("g_bss", 0x1000)],
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.abs64, 1);
        let target = load_bias.wrapping_add(pref_vaddr + 0x500);
        let bytes = module.memory.read(target, 8).unwrap();
        let val = u64::from_le_bytes(bytes.try_into().unwrap());
        let s = load_bias.wrapping_add(0x1000);
        assert_eq!(val, s.wrapping_add(0x10));
        assert!(module.relocations[0].applied);
    }

    #[test]
    fn abs64_local_stn_undef_skipped() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xDD);
        let elf = make_elf(vec![make_reloc_with_sym(0x800000100, 1, 0, 0)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.abs64, 1);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &[0xDD; 8]);
        assert!(!module.relocations[0].applied);
    }

    #[test]
    fn abs64_import_resolved() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 1, 1, 0)],
            vec![make_null_sym(), make_sym("sceKernelSleep#libkernel")],
        );
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let summary = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap();
        assert_eq!(summary.abs64, 1);
        assert_eq!(summary.stubbed_imports, 1);
        assert_eq!(summary.resolved_imports, 0);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let addr = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(addr, 0xFFFF_0000_0000_0000);
        assert!(module.relocations[0].applied);
    }

    #[test]
    fn abs64_import_without_resolver_counts_only() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xEE);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 1, 1, 0)],
            vec![make_null_sym(), make_sym("sceKernelSleep#libkernel")],
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.abs64, 1);
        assert_eq!(summary.missing_imports, 0);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &[0xEE; 8]);
        assert!(!module.relocations[0].applied);
    }

    #[test]
    fn glob_dat_is_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![make_reloc(0x800000100, 6, 0)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.glob_dat, 1);
        assert_eq!(summary.relative, 0);
        assert!(!module.relocations[0].applied);
    }

    #[test]
    fn jump_slot_is_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![make_reloc(0x800000100, 7, 0)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.jump_slot, 1);
    }

    #[test]
    fn unknown_type_is_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![make_reloc(0x800000100, 99, 0)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.unknown, 1);
        assert_eq!(summary.relative, 0);
    }

    #[test]
    fn mixed_relocations_all_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![
            make_reloc(0x800000100, 8, 0x100),
            make_reloc(0x800000108, 8, 0x200),
            make_reloc(0x800000110, 6, 0),
            make_reloc(0x800000118, 7, 0),
            make_reloc(0x800000120, 1, 0),
            make_reloc(0x800000128, 99, 0),
        ]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 2);
        assert_eq!(summary.glob_dat, 1);
        assert_eq!(summary.jump_slot, 1);
        assert_eq!(summary.abs64, 1);
        assert_eq!(summary.unknown, 1);
        assert_eq!(module.relocations.len(), 6);
        assert!(module.relocations[0].applied);
        assert!(module.relocations[1].applied);
        assert!(!module.relocations[2].applied);
    }

    #[test]
    fn empty_relocations_returns_empty_summary() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary, RelocationSummary::default());
        assert!(module.relocations.is_empty());
        assert_eq!(
            module.relocation_summary,
            Some(RelocationSummary::default())
        );
    }

    #[test]
    fn rela_count_fast_path_skips_type_lookup() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xAA);
        let elf = make_elf_with_rela_count(
            vec![
                make_reloc(0x800000100, 8, 0x100),
                make_reloc(0x800000108, 8, 0x200),
                make_reloc(0x800000110, 6, 0),
            ],
            2,
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 2);
        assert_eq!(summary.relative_fast_path, 2);
        assert_eq!(summary.glob_dat, 1);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &0x100u64.to_le_bytes());
        let bytes = module.memory.read(0x800000108, 8).unwrap();
        assert_eq!(bytes, &0x200u64.to_le_bytes());
        let bytes = module.memory.read(0x800000110, 8).unwrap();
        assert_eq!(bytes, &[0xAA; 8]);
        assert!(module.relocations[0].applied);
        assert!(module.relocations[1].applied);
        assert!(!module.relocations[2].applied);
    }

    #[test]
    fn rela_count_clamped_when_exceeds_relocations() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xBB);
        let elf = make_elf_with_rela_count(
            vec![
                make_reloc(0x800000100, 8, 0x100),
                make_reloc(0x800000108, 8, 0x200),
            ],
            999,
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 2);
        assert_eq!(summary.relative_fast_path, 2);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        assert_eq!(bytes, &0x100u64.to_le_bytes());
        let bytes = module.memory.read(0x800000108, 8).unwrap();
        assert_eq!(bytes, &0x200u64.to_le_bytes());
    }

    #[test]
    fn rela_count_entire_table_fast_path() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0xCC);
        let elf = make_elf_with_rela_count(
            vec![
                make_reloc(0x800000100, 8, 0xA),
                make_reloc(0x800000108, 8, 0xB),
                make_reloc(0x800000110, 8, 0xC),
            ],
            3,
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 3);
        assert_eq!(summary.relative_fast_path, 3);
        assert!(module.relocations.iter().all(|r| r.applied));
    }

    #[test]
    fn relative_to_unmapped_address_returns_error() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![make_reloc(0x900000000, 8, 0xDEAD)]);
        let err = apply_relocations(&mut module, &elf).unwrap_err();
        assert!(err.to_string().contains("RELATIVE write failed"));
        assert!(err.to_string().contains("0x900000000"));
    }

    #[test]
    fn copy_relocations_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![make_reloc(0x800000100, 5, 0)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.copy, 1);
    }

    #[test]
    fn tls_relocations_counted() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf(vec![
            make_reloc(0x800000100, 16, 0),
            make_reloc(0x800000108, 17, 0),
            make_reloc(0x800000110, 18, 0),
        ]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.tls, 3);
    }

    #[test]
    fn relocation_kind_name_matches_expected() {
        assert_eq!(RelocationKind::Relative.name(), "RELATIVE");
        assert_eq!(RelocationKind::GlobDat.name(), "GLOB_DAT");
        assert_eq!(RelocationKind::JumpSlot.name(), "JUMP_SLOT");
        assert_eq!(RelocationKind::Abs64.name(), "ABS64");
        assert_eq!(RelocationKind::Copy.name(), "COPY");
        assert_eq!(RelocationKind::Tls.name(), "TLS");
        assert_eq!(RelocationKind::Ifunc.name(), "IFUNC");
        assert_eq!(RelocationKind::Unknown(42).name(), "Unknown");
    }

    #[test]
    fn apply_relative_with_nonzero_load_bias() {
        let load_bias = 0x800000000u64;
        let pref_region_vaddr = 0x800000000u64;
        let actual_region_vaddr = pref_region_vaddr.wrapping_add(load_bias);

        let mut module = LoadedModule {
            name: "test".into(),
            name_source: crate::mapper::ModuleNameSource::Filename,
            module_type: crate::mapper::ModuleType::Eboot,
            memory: crate::memory::ProcessMemory::new(vec![crate::memory::MemoryRegion {
                vaddr: actual_region_vaddr,
                size: 0x1000,
                file_offset: pref_region_vaddr,
                file_size: 0x1000,
                permissions: crate::memory::SegmentFlags::from_p_flags(6),
                data: vec![0xAA; 0x1000],
            }]),
            preferred_base: pref_region_vaddr,
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
        };

        let r_offset = pref_region_vaddr + 0x100;
        let elf = make_elf(vec![make_reloc(r_offset, 8, 0xCAFE)]);
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.relative, 1);

        let target = load_bias.wrapping_add(r_offset);
        let expected_value = load_bias.wrapping_add(0xCAFE);
        let bytes = module.memory.read(target, 8).unwrap();
        assert_eq!(bytes, &expected_value.to_le_bytes());
    }

    #[test]
    fn import_resolver_stubs_glob_dat() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 6, 0, 0)],
            vec![make_sym("sceKernelSleep#libkernel")],
        );
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let summary = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap();
        assert_eq!(summary.glob_dat, 1);
        assert_eq!(summary.stubbed_imports, 1);
        assert_eq!(summary.resolved_imports, 0);
        assert_eq!(summary.missing_imports, 0);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let addr = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(addr, 0xFFFF_0000_0000_0000);
        assert!(module.relocations[0].applied);
    }

    #[test]
    fn import_resolver_stubs_jump_slot() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 7, 0, 0)],
            vec![make_sym("sceKernelSleep#libkernel")],
        );
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let summary = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap();
        assert_eq!(summary.jump_slot, 1);
        assert_eq!(summary.stubbed_imports, 1);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let addr = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(addr, 0xFFFF_0000_0000_0000);
        assert!(module.relocations[0].applied);
    }

    #[test]
    fn import_resolver_without_resolver_still_counts() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![make_reloc_with_sym(0x800000100, 6, 0, 0)],
            vec![make_sym("sceKernelSleep#libkernel")],
        );
        let summary = apply_relocations(&mut module, &elf).unwrap();
        assert_eq!(summary.glob_dat, 1);
        assert_eq!(summary.missing_imports, 1);
        assert_eq!(summary.stubbed_imports, 0);
        assert!(!module.relocations[0].applied);
    }

    #[test]
    fn import_resolver_caches_stable_addresses() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![
                make_reloc_with_sym(0x800000100, 6, 0, 0),
                make_reloc_with_sym(0x800000108, 6, 0, 0),
            ],
            vec![make_sym("sceKernelSleep#libkernel")],
        );
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let summary = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap();
        assert_eq!(summary.glob_dat, 2);
        assert_eq!(summary.stubbed_imports, 2);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let addr1 = u64::from_le_bytes(bytes.try_into().unwrap());
        let bytes = module.memory.read(0x800000108, 8).unwrap();
        let addr2 = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn import_resolver_invalid_symbol_errors() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(vec![make_reloc_with_sym(0x800000100, 6, 99, 0)], vec![]);
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let err = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap_err();
        assert!(err.to_string().contains("symbol index 99 not found"));
    }

    #[test]
    fn import_resolver_allocates_unique_per_import() {
        let mut module = make_module(0, 0x800000000, 0x1000, 0x00);
        let elf = make_elf_sym(
            vec![
                make_reloc_with_sym(0x800000100, 6, 0, 0),
                make_reloc_with_sym(0x800000108, 6, 1, 0),
            ],
            vec![
                make_sym("sceKernelSleep#libkernel"),
                make_sym("scePthreadCreate#libkernel"),
            ],
        );
        let mut stubber = StubAllocator::new(0xFFFF_0000_0000_0000);
        let summary = apply_relocations_with(&mut module, &elf, Some(&mut stubber)).unwrap();
        assert_eq!(summary.glob_dat, 2);
        assert_eq!(summary.stubbed_imports, 2);
        let bytes = module.memory.read(0x800000100, 8).unwrap();
        let addr1 = u64::from_le_bytes(bytes.try_into().unwrap());
        let bytes = module.memory.read(0x800000108, 8).unwrap();
        let addr2 = u64::from_le_bytes(bytes.try_into().unwrap());
        assert_ne!(addr1, addr2);
    }
}
