#![allow(clippy::collapsible_if)]
use std::path::Path;

use ps5_deps::ModuleGraph;

use crate::cli::OutputFormat;
use crate::util::write_to_output_or_stdout;

pub(crate) fn cmd_deps(path: &Path, format: OutputFormat, output: &Option<std::path::PathBuf>) {
    // Build a simple ModuleGraph from the filesystem.
    // For now, we reuse ps5-analysis collection if it's a dataset, otherwise scan for eboot.bin.
    let graph = if path.is_file() {
        // Single file: show its direct DT_NEEDED
        let mut g = ModuleGraph::new();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        g.add_node(name, &[]);
        // Try to parse as ELF and extract needed files (best-effort)
        if let Ok(data) = std::fs::read(path) {
            if let Ok(img) = ps5_self::SelfImage::parse(&data) {
                for needed in &img.elf.needed_files {
                    g.add_edge(name, needed);
                }
            }
        }
        g
    } else {
        // Directory: scan for games (reuse ps5-analysis find logic)
        let mut g = ModuleGraph::new();
        // Walk for eboot.bin and prx files to infer deps
        let mut game_dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    // Simple: if contains eboot.bin, add node
                    if p.join("eboot.bin").exists() {
                        let name = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("game")
                            .to_string();
                        g.add_node(&name, &[]);
                        // Add dummy edges for demonstration (scan prx)
                        if let Ok(prx_entries) = std::fs::read_dir(p.join("sce_module")) {
                            for prx in prx_entries.flatten() {
                                if let Some(fname) = prx.path().file_name().and_then(|n| n.to_str())
                                {
                                    if fname.ends_with(".prx") {
                                        g.add_edge(&name, fname);
                                    }
                                }
                            }
                        }
                        game_dirs.push(p);
                    }
                }
            }
        }
        if game_dirs.is_empty() {
            // Fallback: treat path itself as a game dir
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("game")
                .to_string();
            g.add_node(&name, &[]);
        }
        g
    };

    match format {
        OutputFormat::Dot => {
            write_to_output_or_stdout(output, &|w| {
                writeln!(w, "digraph deps {{")?;
                for edge in graph
                    .all_modules()
                    .flat_map(|m| graph.dependencies(m).into_iter().map(move |d| (m, d)))
                {
                    // This is a simplified view; actual edges are in graph.edges
                    let _ = edge;
                }
                // Use the graph's internal edges via debug
                for node in graph.all_modules() {
                    for dep in graph.dependencies(node) {
                        writeln!(w, "  \"{}\" -> \"{}\";", node, dep)?;
                    }
                }
                // Also include unavailable
                for missing in graph.unavailable_modules() {
                    writeln!(w, "  \"{}\" [style=dashed, color=red];", missing)?;
                }
                writeln!(w, "}}")?;
                Ok(())
            });
        }
        OutputFormat::Json => {
            write_to_output_or_stdout(output, &|w| {
                let report = ps5_deps::report::DepReport::from_graph(&graph);
                let json = serde_json::to_string_pretty(&report).unwrap();
                writeln!(w, "{}", json)
            });
        }
        _ => {
            // Terminal: pretty print
            let report = ps5_deps::report::DepReport::from_graph(&graph);
            write_to_output_or_stdout(output, &|w| {
                writeln!(w, "Modules ({}):", report.modules.len())?;
                for m in &report.modules {
                    writeln!(w, "  - {}", m)?;
                }
                writeln!(w, "\nEdges ({}):", report.edges.len())?;
                for (from, to) in &report.edges {
                    writeln!(w, "  {} -> {}", from, to)?;
                }
                if !report.missing.is_empty() {
                    writeln!(w, "\nMissing ({}):", report.missing.len())?;
                    for m in &report.missing {
                        writeln!(w, "  ! {}", m)?;
                    }
                }
                match &report.load_order {
                    Ok(order) => {
                        writeln!(w, "\nLoad order:")?;
                        for (i, m) in order.iter().enumerate() {
                            writeln!(w, "  {}. {}", i + 1, m)?;
                        }
                    }
                    Err(cycle) => {
                        writeln!(w, "\nCycle detected: {:?}", cycle)?;
                    }
                }
                Ok(())
            });
        }
    }
}
