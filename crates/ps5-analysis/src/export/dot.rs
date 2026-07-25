use crate::model::*;
use std::io::Write;

pub fn export_graph(graph: &DependencyGraph, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "digraph PS5Analysis {{")?;
    writeln!(writer, "    rankdir=LR;")?;
    writeln!(writer, "    node [fontsize=10];")?;
    writeln!(writer)?;

    writeln!(writer, "    // Game nodes")?;
    for game in &graph.game_nodes {
        writeln!(writer, "    \"{}\" [shape=box,style=filled,fillcolor=lightblue];", game)?;
    }
    writeln!(writer)?;

    writeln!(writer, "    // Library nodes")?;
    for lib in &graph.lib_nodes {
        writeln!(writer, "    \"{}\" [shape=ellipse,style=filled,fillcolor=lightyellow];", lib)?;
    }
    writeln!(writer)?;

    writeln!(writer, "    // Edges")?;
    for edge in &graph.edges {
        writeln!(writer, "    \"{}\" -> \"{}\" [label=\"{}\"];",
            edge.from, edge.to, edge.weight)?;
    }

    writeln!(writer, "}}")?;
    Ok(())
}
