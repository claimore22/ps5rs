use ps5_loader::{ModuleContext, NidResolver, RelocationKind, SymbolNidResolver};

use crate::error::EmuError;
use crate::process::ModuleBytes;

/// One guest import bound to a slot in a loaded module's memory.
#[derive(Debug, Clone)]
pub struct ImportBinding {
    /// Canonical name of the module importing this symbol.
    pub module: String,
    /// Numeric NID (SHA1+SALT hash) of the symbol.
    pub nid: u64,
    /// 11-character base64 NID string.
    pub nid_str: String,
    /// Readable name, when the catalog resolves it.
    pub name: Option<String>,
    /// Library tag from the masked symbol (`#libc`, `#A#B`, ...), or the plain
    /// library name when the module's `import_libs` table resolves it.
    pub library: String,
    /// Runtime address of the GOT / data slot (`load_bias + r_offset`).
    pub got_slot: u64,
    /// Value the loader patched into the slot (real export or stub address).
    pub current: u64,
    /// HLE handler address once the ABI bridge exists.
    pub handler: Option<u64>,
}

/// Aggregate import bindings across every loaded module.
#[derive(Debug, Clone, Default)]
pub struct ImportTable {
    pub bindings: Vec<ImportBinding>,
    /// Bindings whose NID resolved to a readable name.
    pub known: usize,
    /// Bindings whose NID is not in the catalog.
    pub unknown: usize,
}

/// Enumerate all import bindings from a loaded [`ModuleContext`] and the
/// module byte images it was built from.
///
/// Relocations (GLOB_DAT / JUMP_SLOT / import ABS64) are the ground truth for
/// both the imported symbol and the slot it lives in; the slot's current
/// value reflects the loader's resolution (real export address or a stub).
pub fn build_import_table(
    ctx: &ModuleContext,
    sources: &[ModuleBytes],
    catalog: Option<&ps5_nid::Catalog>,
) -> Result<ImportTable, EmuError> {
    let mut bindings = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for module in &ctx.modules {
        let Some(src) = sources
            .iter()
            .find(|s| s.name == module.canonical_name() || s.name == module.name)
        else {
            continue;
        };
        let elf = ps5_elf::ElfImage::parse(&src.bytes, None)
            .map_err(|e| EmuError::Parse(format!("{}: {e}", src.name)))?;

        for reloc in &elf.relocations {
            let kind = RelocationKind::from_type(reloc.r_type());
            let is_import = matches!(kind, RelocationKind::GlobDat | RelocationKind::JumpSlot)
                || (kind == RelocationKind::Abs64 && reloc.r_sym() != 0);
            if !is_import {
                continue;
            }
            let Some(sym) = elf.symbols.get(reloc.r_sym() as usize) else {
                continue;
            };
            if !sym.is_import {
                continue;
            }
            let Some(nid) = SymbolNidResolver.resolve(&sym.resolved_name) else {
                continue;
            };

            let (nid_part, lib_part) = sym
                .resolved_name
                .split_once('#')
                .map(|(n, l)| (Some(n.to_string()), Some(l.to_string())))
                .unwrap_or((Some(sym.resolved_name.clone()), None));

            let lib_id = ps5_nid::lib_id_from_nid(&sym.resolved_name).unwrap_or(0);
            let library = elf
                .import_libs
                .get(&lib_id)
                .cloned()
                .unwrap_or_else(|| lib_part.clone().unwrap_or_default());

            let got_slot = module.load_bias.wrapping_add(reloc.r_offset);
            let current = module
                .memory
                .read(got_slot, 8)
                .ok()
                .map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
                .unwrap_or(0);

            if !seen.insert((module.canonical_name().to_string(), got_slot)) {
                continue;
            }

            let name = nid_part.as_deref().and_then(|nid_str| {
                catalog.and_then(|c| {
                    c.resolve(nid_str)
                        .and_then(|entry| entry.primary_name().map(str::to_string))
                })
            });

            bindings.push(ImportBinding {
                module: module.canonical_name().to_string(),
                nid,
                nid_str: nid_part.unwrap_or_default(),
                name,
                library,
                got_slot,
                current,
                handler: None,
            });
        }
    }

    bindings.sort_by_key(|b| b.nid);
    let known = bindings.iter().filter(|b| b.name.is_some()).count();
    let unknown = bindings.len() - known;
    Ok(ImportTable {
        bindings,
        known,
        unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_produces_empty_table() {
        let graph = ps5_loader::ModuleGraph::new();
        let stubs = ps5_loader::StubAllocator::new(0);
        let ctx =
            ps5_loader::ModuleContext::new(Vec::new(), Default::default(), graph, stubs, 0, 0, 0);
        let table = build_import_table(&ctx, &[], None).unwrap();
        assert!(table.bindings.is_empty());
        assert_eq!(table.known, 0);
        assert_eq!(table.unknown, 0);
    }
}
