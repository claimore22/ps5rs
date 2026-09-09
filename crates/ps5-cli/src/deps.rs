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
        let mut g = ModuleGraph::new();
        let mut game_dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.join("eboot.bin").exists() {
                    let name = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("game")
                        .to_string();
                    g.add_node(&name, &[]);
                    if let Ok(data) = std::fs::read(p.join("eboot.bin")) {
                        if let Ok(img) = ps5_self::SelfImage::parse(&data) {
                            for needed in &img.elf.needed_files {
                                g.add_edge(&name, needed);
                            }
                            for lib in img.elf.import_libs.values() {
                                if !img.elf.needed_files.contains(lib) {
                                    g.add_edge(&name, lib);
                                }
                            }
                        }
                    }
                    let sce_module = p.join("sce_module");
                    if sce_module.is_dir() {
                        if let Ok(prx_entries) = std::fs::read_dir(&sce_module) {
                            for prx in prx_entries.flatten() {
                                let prx_path = prx.path();
                                let Some(fname) = prx_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|s| s.to_string())
                                else {
                                    continue;
                                };
                                if !fname.ends_with(".prx") && !fname.ends_with(".sprx") {
                                    continue;
                                }
                                g.add_node(&fname, &[]);
                                g.add_edge(&name, &fname);
                                if let Ok(data) = std::fs::read(&prx_path) {
                                    if let Ok(img) = ps5_self::SelfImage::parse(&data) {
                                        for needed in &img.elf.needed_files {
                                            g.add_edge(&fname, needed);
                                        }
                                    } else if let Ok(img) = ps5_elf::ElfImage::parse(&data, None) {
                                        for needed in &img.needed_files {
                                            g.add_edge(&fname, needed);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    game_dirs.push(p);
                }
            }
        }
        if game_dirs.is_empty() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("game")
                .to_string();
            g.add_node(&name, &[]);
            if path.join("eboot.bin").exists() {
                if let Ok(data) = std::fs::read(path.join("eboot.bin")) {
                    if let Ok(img) = ps5_self::SelfImage::parse(&data) {
                        for needed in &img.elf.needed_files {
                            g.add_edge(&name, needed);
                        }
                    }
                }
            }
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
