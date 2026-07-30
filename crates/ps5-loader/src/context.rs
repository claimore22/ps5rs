use crate::exports::ExportTable;
use crate::graph::ModuleGraph;
use crate::imports::StubAllocator;
use crate::mapper::LoadedModule;

/// The result of loading a set of modules (eboot + PRX dependencies).
///
/// Owns the loaded modules, the merged export table, and the dependency
/// graph.  This is the primary API returned by
/// [`load_modules`](crate::load_modules).
///
/// `ModuleContext` deliberately avoids the term `Process` because runtime
/// execution (threads, syscalls, etc.) is a separate concern that belongs
/// in a higher layer.
#[derive(Debug)]
pub struct ModuleContext {
    /// All loaded modules in load (dependency) order.
    /// Index 0 is the deepest dependency; last is the eboot.
    pub modules: Vec<LoadedModule>,
    /// Merged export table from all registered modules.
    pub exports: ExportTable,
    /// Dependency graph of the loaded modules.
    pub graph: ModuleGraph,
    /// The stub allocator used for unresolved imports.
    pub stub_allocator: StubAllocator,
    /// Total imports resolved against module exports.
    pub resolved_imports: u32,
    /// Total imports known (matched via offline export table, but not loaded).
    pub known_imports: u32,
    /// Total imports assigned stub addresses.
    pub stubbed_imports: u32,
}

impl ModuleContext {
    pub fn new(
        modules: Vec<LoadedModule>,
        exports: ExportTable,
        graph: ModuleGraph,
        stub_allocator: StubAllocator,
        resolved_imports: u32,
        known_imports: u32,
        stubbed_imports: u32,
    ) -> Self {
        Self {
            modules,
            exports,
            graph,
            stub_allocator,
            resolved_imports,
            known_imports,
            stubbed_imports,
        }
    }

    /// Return a reference to the eboot (the last module in load order).
    pub fn eboot(&self) -> Option<&LoadedModule> {
        self.modules
            .iter()
            .rev()
            .find(|m| m.module_type == crate::mapper::ModuleType::Eboot)
    }

    /// Return a mutable reference to the eboot.
    pub fn eboot_mut(&mut self) -> Option<&mut LoadedModule> {
        self.modules
            .iter_mut()
            .rev()
            .find(|m| m.module_type == crate::mapper::ModuleType::Eboot)
    }

    /// Return all PRX modules.
    pub fn prxs(&self) -> Vec<&LoadedModule> {
        self.modules
            .iter()
            .filter(|m| m.module_type == crate::mapper::ModuleType::Prx)
            .collect()
    }

    /// Find a module by canonical name.
    pub fn find_module(&self, name: &str) -> Option<&LoadedModule> {
        self.modules
            .iter()
            .find(|m| m.canonical_name() == name || m.name == name)
    }

    /// Find a mutable module by canonical name.
    pub fn find_module_mut(&mut self, name: &str) -> Option<&mut LoadedModule> {
        self.modules
            .iter_mut()
            .find(|m| m.canonical_name() == name || m.name == name)
    }
}
