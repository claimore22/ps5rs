use serde::{Deserialize, Serialize};

/// A serializable load report for one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub module_type: String,
    pub load_bias: u64,
    pub entry_point: Option<u64>,
    pub exports_count: usize,
    pub imports_resolved: u32,
    pub imports_known: u32,
    pub imports_stubbed: u32,
    pub relative: u32,
    pub glob_dat: u32,
    pub jump_slot: u32,
    pub abs64: u32,
    pub copy: u32,
    pub tls_relocations: u32,
    pub ifunc: u32,
    pub unknown: u32,
    pub state: String,
    pub has_tls: bool,
    pub init_va: u64,
    pub init_array_va: u64,
    pub init_array_sz: u64,
    pub fini_va: u64,
    pub fini_array_va: u64,
    pub fini_array_sz: u64,
    pub preinit_array_va: u64,
    pub preinit_array_sz: u64,
    pub per_library: Vec<ps5_loader::LibraryImportCounts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInfo {
    pub nodes: Vec<String>,
    pub unavailable: Vec<String>,
    pub edges: Vec<EdgeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInfo {
    pub from: String,
    pub to: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Totals {
    pub modules: usize,
    pub resolved: u32,
    pub known: u32,
    pub stubbed: u32,
    pub exports: usize,
    pub unavailable: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadReport {
    pub modules: Vec<ModuleInfo>,
    pub graph: GraphInfo,
    pub totals: Totals,
}

/// Build the serializable report from a loaded module context.
pub fn build_report(ctx: &ps5_loader::ModuleContext) -> LoadReport {
    let modules: Vec<ModuleInfo> = ctx
        .modules
        .iter()
        .map(|m| {
            let type_label = match m.module_type {
                ps5_loader::ModuleType::Eboot => "Eboot",
                ps5_loader::ModuleType::Prx => "Prx",
            };
            let state_label = match m.state {
                ps5_loader::ModuleState::Mapped => "Mapped",
                ps5_loader::ModuleState::Relocated => "Relocated",
                ps5_loader::ModuleState::Linked => "Linked",
                ps5_loader::ModuleState::Initialized => "Initialized",
            };
            let rs = m.relocation_summary.as_ref();
            ModuleInfo {
                name: m.name.clone(),
                module_type: type_label.to_string(),
                load_bias: m.load_bias,
                entry_point: m.entry_point,
                exports_count: m.exports_count,
                imports_resolved: m.imports_resolved,
                imports_known: m.imports_known,
                imports_stubbed: m.imports_stubbed,
                relative: rs.map(|s| s.relative).unwrap_or(0),
                glob_dat: rs.map(|s| s.glob_dat).unwrap_or(0),
                jump_slot: rs.map(|s| s.jump_slot).unwrap_or(0),
                abs64: rs.map(|s| s.abs64).unwrap_or(0),
                copy: rs.map(|s| s.copy).unwrap_or(0),
                tls_relocations: rs.map(|s| s.tls).unwrap_or(0),
                ifunc: rs.map(|s| s.ifunc).unwrap_or(0),
                unknown: rs.map(|s| s.unknown).unwrap_or(0),
                state: state_label.to_string(),
                has_tls: m.tls.is_some(),
                init_va: m.init_va,
                init_array_va: m.init_array_va,
                init_array_sz: m.init_array_sz,
                fini_va: m.fini_va,
                fini_array_va: m.fini_array_va,
                fini_array_sz: m.fini_array_sz,
                preinit_array_va: m.preinit_array_va,
                preinit_array_sz: m.preinit_array_sz,
                per_library: m.per_library_imports.clone(),
            }
        })
        .collect();

    let nodes: Vec<String> = ctx.graph.all_modules().map(|s| s.to_string()).collect();
    let unavailable: Vec<String> = ctx
        .graph
        .unavailable_modules()
        .map(|s| s.to_string())
        .collect();
    let mut edges = Vec::new();
    for node in &nodes {
        for dep in ctx.graph.dependencies(node) {
            let status = if ctx.graph.is_unavailable(dep) {
                "missing"
            } else {
                "loaded"
            };
            edges.push(EdgeInfo {
                from: node.clone(),
                to: dep.to_string(),
                status: status.to_string(),
            });
        }
    }

    let totals = Totals {
        modules: ctx.modules.len(),
        resolved: ctx.resolved_imports,
        known: ctx.known_imports,
        stubbed: ctx.stubbed_imports,
        exports: ctx.exports.len(),
        unavailable: unavailable.len(),
    };

    LoadReport {
        modules,
        graph: GraphInfo {
            nodes,
            unavailable,
            edges,
        },
        totals,
    }
}
