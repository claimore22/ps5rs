use crate::model::*;
use std::io::Write;

pub fn export_graph(graph: &DependencyGraph, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "digraph PS5Analysis {{")?;
    writeln!(writer, "    rankdir=LR;")?;
    writeln!(writer, "    node [fontsize=10];")?;
    writeln!(writer)?;

    writeln!(writer, "    // Game nodes")?;
    for game in &graph.game_nodes {
        writeln!(
            writer,
            "    \"{}\" [shape=box,style=filled,fillcolor=lightblue];",
            game
        )?;
    }
    writeln!(writer)?;

    writeln!(writer, "    // Library nodes")?;
    for lib in &graph.lib_nodes {
        writeln!(
            writer,
            "    \"{}\" [shape=ellipse,style=filled,fillcolor=lightyellow];",
            lib
        )?;
    }
    writeln!(writer)?;

    writeln!(writer, "    // Edges")?;
    for edge in &graph.edges {
        writeln!(
            writer,
            "    \"{}\" -> \"{}\" [label=\"{}\"];",
            edge.from, edge.to, edge.weight
        )?;
    }

    writeln!(writer, "}}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DependencyGraph, make_db, make_game, make_import};
    use crate::reports::build_graph;

    #[test]
    fn dot_graph_valid_syntax() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "funcA", 1, "libA")],
        )]);
        let graph = build_graph(&db, false);
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with("digraph PS5Analysis {"));
        assert!(s.ends_with("}\n"));
        assert!(s.contains("rankdir=LR;"));
        assert!(s.contains("node [fontsize=10];"));
    }

    #[test]
    fn dot_graph_contains_game_node() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "funcA", 1, "libA")],
        )]);
        let graph = build_graph(&db, false);
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"GameA\" [shape=box,style=filled,fillcolor=lightblue];"));
    }

    #[test]
    fn dot_graph_contains_lib_node() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "funcA", 1, "libA")],
        )]);
        let graph = build_graph(&db, false);
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"libA\" [shape=ellipse,style=filled,fillcolor=lightyellow];"));
    }

    #[test]
    fn dot_graph_contains_edges() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "funcB", 1, "libA"),
            ],
        )]);
        let graph = build_graph(&db, false);
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"GameA\" -> \"libA\" [label=\"2\"]"));
    }

    #[test]
    fn dot_empty_graph() {
        let graph = DependencyGraph {
            game_nodes: vec![],
            lib_nodes: vec![],
            edges: vec![],
        };
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("digraph PS5Analysis {"));
        assert!(s.contains("}"));
        // No edges
        assert!(!s.contains("->"));
    }
}
