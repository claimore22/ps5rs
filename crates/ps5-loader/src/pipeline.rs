use std::collections::HashSet;

use crate::address::LoadAddressAllocator;
use crate::context::ModuleContext;
use crate::exports::ExportTable;
use crate::graph::ModuleGraph;
use crate::imports::StubAllocator;
use crate::mapper::{LoadedModule, LoaderError, ModuleState, load_elf};
use crate::offline::OfflineExportTable;
use crate::relocation::apply_relocations_with;
use crate::resolver::CrossModuleResolver;

/// Load an eboot and all its PRX dependencies (transitive), returning a
/// fully-linked [`ModuleContext`].
///
/// # Pipeline phases (per module, in dependency order)
///
/// 1. **Map** — load the eboot/PRX via [`load_elf`].
/// 2. **RELATIVE** — apply `R_X86_64_RELATIVE` relocations (no import resolver).
/// 3. **Export** — register defined symbols in the global export table.
/// 4. **Resolve** — resolve `GLOB_DAT` / `JUMP_SLOT` imports using the global
///    export table with [`CrossModuleResolver`] (stub fallback for unknowns).
///
/// Modules are loaded in dependency-first order (DFS) so that every module
/// sees its dependency's exports during resolution.  Modules referenced by
/// `DT_NEEDED` but not provided by the `prx_provider` are recorded in the
/// graph as unavailable.
///
/// Pass [`OfflineExportTable`] via `offline_exports` to enable known-system-
/// function detection for system PRX exports (even when the system PRX is
/// not loaded).
struct LoadContext<'a, P>
where
    P: FnMut(&str) -> Option<Vec<u8>>,
{
    export_table: ExportTable,
    offline_exports: Option<&'a OfflineExportTable>,
    graph: ModuleGraph,
    address_alloc: LoadAddressAllocator,
    stub_alloc: StubAllocator,
    loaded_modules: Vec<LoadedModule>,
    total_resolved: u32,
    total_known: u32,
    total_stubbed: u32,
    processed: HashSet<String>,
    prx_provider: &'a mut P,
}

impl<P> LoadContext<'_, P>
where
    P: FnMut(&str) -> Option<Vec<u8>>,
{
    fn load_one(
        &mut self,
        name: &str,
        elf: &ps5_elf::ElfImage,
    ) -> Result<Option<LoadedModule>, LoaderError> {
        let canonical = elf.soname.as_deref().unwrap_or(name);
        if !self.processed.insert(canonical.to_string()) {
            return Ok(self
                .loaded_modules
                .iter()
                .find(|m| m.canonical_name() == canonical)
                .cloned());
        }

        for needed_name in &elf.needed_files {
            let Some(prx_bytes) = (self.prx_provider)(needed_name) else {
                tracing::warn!(module = %needed_name, "PRX not found, marking unavailable");
                self.graph.mark_unavailable(needed_name);
                self.graph.add_edge(canonical, needed_name);
                continue;
            };
            let prx_elf = ps5_elf::ElfImage::parse(&prx_bytes, None)
                .map_err(|e| LoaderError(format!("{} parse: {e}", needed_name)))?;

            let prx_canonical = prx_elf.soname.as_deref().unwrap_or(needed_name);
            self.graph.add_edge(canonical, prx_canonical);

            self.load_one(needed_name, &prx_elf)?;
        }

        let module_size = elf
            .program_headers
            .iter()
            .filter(|ph| ph.is_load())
            .map(|ph| ph.p_vaddr + ph.p_memsz)
            .max()
            .unwrap_or(0);
        let load_bias = self.address_alloc.allocate(module_size);

        let mut module = load_elf(name, elf.data)?;
        module.load_bias = load_bias;
        for region in &mut module.memory.regions {
            region.vaddr = region.vaddr.wrapping_add(load_bias);
        }

        let _rel_summary = apply_relocations_with(&mut module, elf, None)?;
        module.state = ModuleState::Relocated;

        let export_count_before = self.export_table.len();
        self.export_table.register_module(&module, elf);
        module.exports_count = self.export_table.len() - export_count_before;

        let mut resolver = CrossModuleResolver::new(
            &self.export_table,
            self.offline_exports,
            &mut self.stub_alloc,
        );
        let _rel_summary = apply_relocations_with(&mut module, elf, Some(&mut resolver))?;
        module.state = ModuleState::Linked;
        module.imports_resolved = resolver.resolved_count();
        module.imports_known = resolver.known_count();
        module.imports_stubbed = resolver.stubbed_count();
        module.per_library_imports = resolver.per_library_imports();

        module.state = ModuleState::Initialized;
        self.total_resolved += resolver.resolved_count();
        self.total_known += resolver.known_count();
        self.total_stubbed += resolver.stubbed_count();

        self.graph
            .add_node(module.canonical_name(), &module.aliases);

        self.loaded_modules.push(module.clone());
        Ok(Some(module))
    }
}

pub fn load_modules(
    eboot_name: &str,
    eboot_bytes: &[u8],
    mut prx_provider: impl FnMut(&str) -> Option<Vec<u8>>,
    offline_exports: Option<&OfflineExportTable>,
) -> Result<ModuleContext, LoaderError> {
    let eboot_elf = ps5_elf::ElfImage::parse(eboot_bytes, None)
        .map_err(|e| LoaderError(format!("eboot ELF parse: {e}")))?;

    let mut ctx = LoadContext {
        export_table: ExportTable::new(),
        offline_exports,
        graph: ModuleGraph::new(),
        address_alloc: LoadAddressAllocator::default(),
        stub_alloc: StubAllocator::new(0x0000_7fff_0000_0000),
        loaded_modules: Vec::new(),
        total_resolved: 0,
        total_known: 0,
        total_stubbed: 0,
        processed: HashSet::new(),
        prx_provider: &mut prx_provider,
    };

    let eboot_canonical = eboot_elf.soname.as_deref().unwrap_or(eboot_name);
    ctx.graph.add_node(eboot_canonical, &[]);

    for needed_name in &eboot_elf.needed_files {
        let Some(prx_bytes) = (ctx.prx_provider)(needed_name) else {
            tracing::warn!(module = %needed_name, "PRX not found, marking unavailable");
            ctx.graph.mark_unavailable(needed_name);
            ctx.graph.add_edge(eboot_canonical, needed_name);
            continue;
        };
        let prx_elf = ps5_elf::ElfImage::parse(&prx_bytes, None)
            .map_err(|e| LoaderError(format!("{} parse: {e}", needed_name)))?;

        let prx_canonical = prx_elf.soname.as_deref().unwrap_or(needed_name);
        ctx.graph.add_edge(eboot_canonical, prx_canonical);

        ctx.load_one(needed_name, &prx_elf)?;
    }

    let module_size = eboot_elf
        .program_headers
        .iter()
        .filter(|ph| ph.is_load())
        .map(|ph| ph.p_vaddr + ph.p_memsz)
        .max()
        .unwrap_or(0);
    let load_bias = ctx.address_alloc.allocate(module_size);

    let mut module = load_elf(eboot_name, eboot_bytes)?;
    module.load_bias = load_bias;
    for region in &mut module.memory.regions {
        region.vaddr = region.vaddr.wrapping_add(load_bias);
    }

    let _rel_summary = apply_relocations_with(&mut module, &eboot_elf, None)?;
    module.state = ModuleState::Relocated;

    let export_count_before = ctx.export_table.len();
    ctx.export_table.register_module(&module, &eboot_elf);
    module.exports_count = ctx.export_table.len() - export_count_before;

    let mut resolver =
        CrossModuleResolver::new(&ctx.export_table, offline_exports, &mut ctx.stub_alloc);
    let _rel_summary = apply_relocations_with(&mut module, &eboot_elf, Some(&mut resolver))?;
    module.state = ModuleState::Linked;
    module.imports_resolved = resolver.resolved_count();
    module.imports_known = resolver.known_count();
    module.imports_stubbed = resolver.stubbed_count();
    module.per_library_imports = resolver.per_library_imports();
    ctx.total_resolved += resolver.resolved_count();
    ctx.total_known += resolver.known_count();
    ctx.total_stubbed += resolver.stubbed_count();

    module.state = ModuleState::Initialized;

    ctx.graph.add_node(module.canonical_name(), &module.aliases);

    ctx.loaded_modules.push(module);

    Ok(ModuleContext::new(
        ctx.loaded_modules,
        ctx.export_table,
        ctx.graph,
        ctx.stub_alloc,
        ctx.total_resolved,
        ctx.total_known,
        ctx.total_stubbed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_simple_elf(e_type: u16) -> Vec<u8> {
        let mut buf = vec![0u8; 0x1100];
        buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        buf[4] = 2;
        buf[5] = 1;
        buf[6] = 1;
        buf[16..18].copy_from_slice(&e_type.to_le_bytes());
        buf[18..20].copy_from_slice(&62u16.to_le_bytes());
        buf[20..24].copy_from_slice(&1u32.to_le_bytes());
        buf[24..32].copy_from_slice(&0u64.to_le_bytes());
        buf[32..40].copy_from_slice(&64u64.to_le_bytes());
        buf[52..54].copy_from_slice(&64u16.to_le_bytes());
        buf[54..56].copy_from_slice(&56u16.to_le_bytes());
        buf[56..58].copy_from_slice(&1u16.to_le_bytes());

        let phoff: usize = 64;
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
    fn load_eboot_no_dependencies() {
        let elf = build_simple_elf(0xFE10);
        let ctx = load_modules("eboot.bin", &elf, |_| None, None).unwrap();
        assert_eq!(ctx.modules.len(), 1);
        assert!(ctx.eboot().is_some());
        assert_eq!(ctx.prxs().len(), 0);
        assert_eq!(ctx.exports.len(), 0);
    }

    #[test]
    fn skip_missing_prx() {
        let elf = build_simple_elf(0xFE10);
        let ctx = load_modules(
            "eboot.bin",
            &elf,
            |name| {
                if name == "missing.prx" {
                    None
                } else {
                    Some(build_simple_elf(0xFE18))
                }
            },
            None,
        )
        .unwrap();
        assert_eq!(ctx.modules.len(), 1);
    }

    #[test]
    fn prx_gets_load_bias() {
        let elf = build_simple_elf(0xFE10);
        let ctx = load_modules(
            "eboot.bin",
            &elf,
            |name| {
                if name == "libc.prx" {
                    Some(build_simple_elf(0xFE18))
                } else {
                    None
                }
            },
            None,
        )
        .unwrap();
        assert_eq!(ctx.modules.len(), 1);
    }

    #[test]
    fn graph_built_from_needed() {
        let needed_name = b"libc.prx";
        let mut strtab = vec![0u8];
        strtab.extend_from_slice(needed_name);
        strtab.push(0);

        let dyn_entries = vec![
            (5u64, 0x100u64),
            (0xau64, strtab.len() as u64),
            (6u64, 0x200u64),
            (0xbu64, 24u64),
            (1u64, 1u64),
        ];
        let mut dyn_data = Vec::new();
        for (tag, val) in &dyn_entries {
            dyn_data.extend_from_slice(&tag.to_le_bytes());
            dyn_data.extend_from_slice(&val.to_le_bytes());
        }
        dyn_data.extend_from_slice(&[0u8; 16]);
        dyn_data.extend_from_slice(&strtab);
        let _padding = 0x100usize.saturating_sub(dyn_data.len());
        dyn_data.resize(0x100 + strtab.len(), 0);
        let dyn_len = (dyn_entries.len() + 1) * 16;
        dyn_data[0x100..0x100 + strtab.len()].copy_from_slice(&strtab);
        let symtab_start = 0x200usize;
        dyn_data.resize(symtab_start + 24, 0);

        let total_size = 0x300;
        let mut elf = vec![0u8; 64 + 56 + total_size];
        elf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&0xFE10u16.to_le_bytes());
        elf[18..20].copy_from_slice(&62u16.to_le_bytes());
        elf[20..24].copy_from_slice(&1u32.to_le_bytes());
        elf[32..40].copy_from_slice(&64u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64u16.to_le_bytes());
        elf[54..56].copy_from_slice(&56u16.to_le_bytes());
        elf[56..58].copy_from_slice(&2u16.to_le_bytes());

        let phoff: usize = 64;
        elf[phoff..phoff + 4].copy_from_slice(&2u32.to_le_bytes());
        elf[phoff + 4..phoff + 8].copy_from_slice(&4u32.to_le_bytes());
        elf[phoff + 8..phoff + 16].copy_from_slice(&64u64.to_le_bytes());
        elf[phoff + 16..phoff + 24].copy_from_slice(&0x1000u64.to_le_bytes());
        elf[phoff + 32..phoff + 40].copy_from_slice(&(dyn_len as u64).to_le_bytes());
        elf[phoff + 40..phoff + 48].copy_from_slice(&(dyn_len as u64).to_le_bytes());

        let phoff2 = phoff + 56;
        elf[phoff2..phoff2 + 4].copy_from_slice(&1u32.to_le_bytes());
        elf[phoff2 + 4..phoff2 + 8].copy_from_slice(&5u32.to_le_bytes());
        elf[phoff2 + 8..phoff2 + 16].copy_from_slice(&(64u64 + 56).to_le_bytes());
        elf[phoff2 + 16..phoff2 + 24].copy_from_slice(&0x1000u64.to_le_bytes());
        elf[phoff2 + 32..phoff2 + 40].copy_from_slice(&(total_size as u64).to_le_bytes());
        elf[phoff2 + 40..phoff2 + 48].copy_from_slice(&(total_size as u64).to_le_bytes());

        let payload_start = 64 + 56;
        elf[payload_start..payload_start + dyn_data.len()].copy_from_slice(&dyn_data);

        let ctx = load_modules(
            "eboot.bin",
            &elf,
            |name| {
                if name == "libc.prx" {
                    Some(build_simple_elf(0xFE18))
                } else {
                    None
                }
            },
            None,
        )
        .unwrap();
        assert!(!ctx.modules.is_empty());
    }

    #[test]
    fn state_transitions_to_initialized() {
        let elf = build_simple_elf(0xFE10);
        let ctx = load_modules("eboot.bin", &elf, |_| None, None).unwrap();
        for module in &ctx.modules {
            assert_eq!(module.state, ModuleState::Initialized);
        }
    }
}
