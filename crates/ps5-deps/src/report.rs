use serde::Serialize;

use crate::graph::ModuleGraph;

#[derive(Debug, Clone, Serialize)]
pub struct DepReport {
    pub modules: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub missing: Vec<String>,
    pub load_order: Result<Vec<String>, Vec<String>>,
}

impl DepReport {
    pub fn from_graph(graph: &ModuleGraph) -> Self {
        let modules: Vec<String> = graph.all_modules().map(|s| s.to_string()).collect();
        let edges: Vec<(String, String)> = graph
            .all_modules()
            .flat_map(|m| {
                graph
                    .dependencies(m)
                    .into_iter()
                    .map(move |d| (m.to_string(), d.to_string()))
            })
            .collect();
        let missing: Vec<String> = graph.unavailable_modules().map(|s| s.to_string()).collect();
        let load_order = graph.topological_sort();
        Self {
            modules,
            edges,
            missing,
            load_order,
        }
    }
}
