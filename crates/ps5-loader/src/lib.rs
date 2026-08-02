//! PS5 ELF/PRX loader — virtual memory model, relocation engine, import resolver,
//! and multi-module dependency loading.
//!
//! # Architecture
//!
//! The loader processes a parsed [`ps5_elf::ElfImage`] through four phases:
//!
//! 1. **Map** — [`load_elf`] creates a [`ProcessMemory`] from `PT_LOAD` segments,
//!    applies zero-fill for `.bss`, and returns a [`LoadedModule`].
//! 2. **Relocate** — [`apply_relocations`] / [`apply_relocations_with`] patches
//!    `R_X86_64_RELATIVE` entries (with a DT_RELACOUNT fast path) and optionally
//!    resolves `GLOB_DAT` / `JUMP_SLOT` imports via a pluggable [`ImportResolver`].
//! 3. **Export table** — [`ExportTable::register_module`] collects defined symbols
//!    from each module, keyed by numeric NID.
//! 4. **Cross-module link** — [`load_modules`] loads an eboot + its PRX dependencies,
//!    applies RELATIVE relocations, registers all exports, then resolves imports
//!    against the merged export table.
//!
//! The crate has no dependency on `ps5_nid` — NID computation is the caller's
//! responsibility.

mod address;
mod context;
mod exports;
mod graph;
mod imports;
mod mapper;
mod memory;
mod nid;
mod offline;
mod pipeline;
mod relocation;
mod resolver;

pub use address::LoadAddressAllocator;
pub use context::ModuleContext;
pub use exports::{ExportEntry, ExportTable};
pub use graph::{DependencyEdge, ModuleGraph};
pub use imports::{
    ImportError, ImportRequest, ImportResolver, ResolveResult, STUB_REGION_BASE, StubAllocator,
};
pub use mapper::{
    ImportBinding, LibraryImportCounts, LoadedModule, ModuleNameSource, ModuleState, ModuleType,
    load_elf,
};
pub use memory::{MemoryError, MemoryErrorKind, MemoryRegion, ProcessMemory, SegmentFlags};
pub use nid::{NidResolver, SymbolNidResolver, compute_nid, nid_to_u64};
pub use offline::{OfflineExportEntry, OfflineExportTable};
pub use pipeline::load_modules;
pub use relocation::{
    RelocationError, RelocationKind, RelocationRecord, RelocationSummary, apply_relocations,
    apply_relocations_with,
};
pub use resolver::CrossModuleResolver;

// Re-export LoaderError from mapper (it's the primary error type)
pub use mapper::LoaderError;
