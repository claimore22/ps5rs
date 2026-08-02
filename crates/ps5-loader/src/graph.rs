use std::collections::{HashMap, HashSet};

/// A single edge in the module dependency graph.
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
}

/// Directed dependency graph for modules.
///
/// Nodes are keyed by **canonical name** — `DT_SONAME` if available,
/// otherwise the `DT_NEEDED` filename.
#[derive(Debug, Clone)]
pub struct ModuleGraph {
    /// Adjacency list: module → modules it depends on.
    edges: Vec<DependencyEdge>,
    /// All known module names (canonical).
    nodes: HashSet<String>,
    /// Aliases for lookup: filename/alias → canonical name.
    aliases: HashMap<String, String>,
    /// Modules that were referenced by `DT_NEEDED` but not provided.
    unavailable: HashSet<String>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            nodes: HashSet::new(),
            aliases: HashMap::new(),
            unavailable: HashSet::new(),
        }
    }

    /// Add a node (module) with its known aliases.
    pub fn add_node(&mut self, canonical: &str, aliases: &[String]) {
        self.nodes.insert(canonical.to_string());
        self.unavailable.remove(canonical);
        for alias in aliases {
            self.aliases
                .entry(alias.clone())
                .or_insert_with(|| canonical.to_string());
        }
    }

    /// Mark a module as unavailable (referenced but not provided).
    pub fn mark_unavailable(&mut self, name: &str) {
        self.nodes.insert(name.to_string());
        self.unavailable.insert(name.to_string());
    }

    /// Check if a module was marked unavailable.
    pub fn is_unavailable(&self, name: &str) -> bool {
        self.unavailable.contains(name)
    }

    /// Iterate over all unavailable module names.
    pub fn unavailable_modules(&self) -> impl Iterator<Item = &str> {
        self.unavailable.iter().map(|s| s.as_str())
    }

    /// Add a dependency edge: `from` requires `to`.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.nodes.insert(from.to_string());
        self.nodes.insert(to.to_string());
        self.edges.push(DependencyEdge {
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    /// Resolve an alias or filename to its canonical name.
    pub fn resolve_alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(|s| s.as_str())
    }

    /// Return all known module names.
    pub fn all_modules(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().map(|s| s.as_str())
    }

    /// Return the direct dependencies of a module (what it `DT_NEEDED`).
    pub fn dependencies(&self, module: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.from == module)
            .map(|e| e.to.as_str())
            .collect()
    }

    /// Return modules that depend on the given module (reverse edges).
    pub fn dependents(&self, module: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.to == module)
            .map(|e| e.from.as_str())
            .collect()
    }

    /// Produce a topological sort of the dependency graph (DFS-based).
    ///
    /// Returns `Ok(load_order)` — modules listed such that a module's
    /// dependencies appear before it.  Returns `Err(cycle_nodes)` if a
    /// cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<String>> {
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();
        let mut order = Vec::new();
        let mut cycle_nodes = Vec::new();

        fn dfs(
            node: &str,
            graph: &ModuleGraph,
            visited: &mut HashSet<String>,
            in_progress: &mut HashSet<String>,
            order: &mut Vec<String>,
            cycle_nodes: &mut Vec<String>,
        ) -> bool {
            if in_progress.contains(node) {
                cycle_nodes.push(node.to_string());
                return false;
            }
            if visited.contains(node) {
                return true;
            }
            in_progress.insert(node.to_string());
            for dep in graph.dependencies(node) {
                if !dfs(dep, graph, visited, in_progress, order, cycle_nodes) {
                    return false;
                }
            }
            in_progress.remove(node);
            visited.insert(node.to_string());
            order.push(node.to_string());
            true
        }

        let mut nodes: Vec<String> = self.nodes.iter().cloned().collect();
        nodes.sort();

        for node in &nodes {
            if !visited.contains(node)
                && !dfs(
                    node,
                    self,
                    &mut visited,
                    &mut in_progress,
                    &mut order,
                    &mut cycle_nodes,
                )
            {
                return Err(cycle_nodes);
            }
        }

        Ok(order)
    }

    /// Breadth-first load order — same as topological but with leaves first.
    ///
    /// This produces the same valid order as [`Self::topological_sort`] but
    /// may be easier to debug.
    pub fn load_order(&self) -> Result<Vec<String>, Vec<String>> {
        let sorted = self.topological_sort()?;
        Ok(sorted)
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = ModuleGraph::new();
        assert_eq!(g.node_count(), 0);
        assert!(g.topological_sort().unwrap().is_empty());
    }

    #[test]
    fn single_node() {
        let mut g = ModuleGraph::new();
        g.add_node("libkernel.prx", &[]);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.topological_sort().unwrap(), vec!["libkernel.prx"]);
    }

    #[test]
    fn linear_dependency() {
        let mut g = ModuleGraph::new();
        g.add_node("eboot.bin", &["eboot.bin".to_string()]);
        g.add_node("libc.prx", &["libc.prx".to_string()]);
        g.add_node("libkernel.prx", &["libkernel.prx".to_string()]);
        g.add_edge("eboot.bin", "libc.prx");
        g.add_edge("libc.prx", "libkernel.prx");
        let order = g.topological_sort().unwrap();
        assert!(
            order.iter().position(|n| n == "libkernel.prx").unwrap()
                < order.iter().position(|n| n == "libc.prx").unwrap()
        );
        assert!(
            order.iter().position(|n| n == "libc.prx").unwrap()
                < order.iter().position(|n| n == "eboot.bin").unwrap()
        );
    }

    #[test]
    fn diamond_dependency() {
        let mut g = ModuleGraph::new();
        g.add_node("eboot", &[]);
        g.add_node("libA", &[]);
        g.add_node("libB", &[]);
        g.add_node("libKernel", &[]);
        g.add_edge("eboot", "libA");
        g.add_edge("eboot", "libB");
        g.add_edge("libA", "libKernel");
        g.add_edge("libB", "libKernel");
        let order = g.topological_sort().unwrap();
        assert_eq!(order.len(), 4);
        assert!(
            order.iter().position(|n| n == "libKernel").unwrap()
                < order.iter().position(|n| n == "libA").unwrap()
        );
        assert!(
            order.iter().position(|n| n == "libKernel").unwrap()
                < order.iter().position(|n| n == "libB").unwrap()
        );
    }

    #[test]
    fn cycle_detected() {
        let mut g = ModuleGraph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_edge("C", "A");
        assert!(g.topological_sort().is_err());
    }

    #[test]
    fn alias_resolution() {
        let mut g = ModuleGraph::new();
        g.add_node(
            "libkernel.prx",
            &["libkernel.prx".to_string(), "libkernel".to_string()],
        );
        assert_eq!(g.resolve_alias("libkernel.prx"), Some("libkernel.prx"));
        assert_eq!(g.resolve_alias("libkernel"), Some("libkernel.prx"));
        assert_eq!(g.resolve_alias("unknown"), None);
    }

    #[test]
    fn dependencies_returns_correct_edges() {
        let mut g = ModuleGraph::new();
        g.add_edge("eboot", "libc");
        g.add_edge("eboot", "libkernel");
        let deps = g.dependencies("eboot");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"libc"));
        assert!(deps.contains(&"libkernel"));
    }

    #[test]
    fn dependents_returns_reverse_edges() {
        let mut g = ModuleGraph::new();
        g.add_edge("eboot", "libkernel");
        g.add_edge("libc", "libkernel");
        let deps = g.dependents("libkernel");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"eboot"));
        assert!(deps.contains(&"libc"));
    }
}
