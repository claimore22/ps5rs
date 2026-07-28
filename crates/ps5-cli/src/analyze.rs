use crate::catalog::load_catalog;
use crate::cli::{AnalyzeCommand, OutputFormat};
use crate::terminal;
use crate::util::{is_dataset_dir, load_dataset_or_collect, write_to_output_or_stdout};
use std::path::PathBuf;

pub(crate) fn cmd_scan(
    path: &std::path::Path,
    output: &std::path::Path,
    extra_nids: &[PathBuf],
    include_modules: bool,
) {
    let catalog = load_catalog(extra_nids);
    let options = ps5_analysis::ScanOptions {
        include_prx: include_modules,
    };

    eprintln!("Scanning {} for game binaries...", path.display());
    let result = ps5_analysis::scan(path, output, &catalog, &options).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    eprintln!(
        "Wrote {} images to {}",
        result.manifest.image_count,
        output.display()
    );
    eprintln!("Dataset schema version: {}", result.manifest.schema_version);
}

pub(crate) fn cmd_analyze(command: AnalyzeCommand, extra_nids: &[PathBuf], include_modules: bool) {
    let catalog = load_catalog(extra_nids);

    match command {
        AnalyzeCommand::Collect { path, output } => {
            eprintln!("Collecting analysis from {}...", path.display());
            let options = ps5_analysis::CollectorOptions {
                include_prx: include_modules,
            };
            let db = ps5_analysis::collect(&path, &catalog, &options);
            eprintln!("Collected {} games", db.games.len());

            write_to_output_or_stdout(&output, &|w| {
                ps5_analysis::export::json::export_analysis(&db, w)
            });
        }

        AnalyzeCommand::Stats {
            path,
            format,
            output,
        } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let stats = ps5_analysis::reports::compute_stats(&db);

            match format {
                OutputFormat::Terminal => terminal::print_stats_terminal(&stats),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_stats(&stats, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    writeln!(w, "metric,value")?;
                    writeln!(w, "total_games,{}", stats.total_games)?;
                    writeln!(w, "total_imports,{}", stats.total_imports)?;
                    writeln!(w, "unique_nids,{}", stats.unique_nids)?;
                    writeln!(w, "unique_libs,{}", stats.unique_libs)?;
                    writeln!(w, "resolution_rate,{:.1}", stats.resolution_rate)?;
                    Ok(())
                }),
                _ => eprintln!("unsupported format for stats"),
            }
        }

        AnalyzeCommand::Heatmap {
            path,
            format,
            output,
        } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let heatmap = ps5_analysis::reports::build_heatmap(&db);

            match format {
                OutputFormat::Terminal => terminal::print_heatmap_terminal(&heatmap),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_heatmap(&heatmap, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_heatmap(&heatmap, w)
                }),
                _ => eprintln!("unsupported format for heatmap"),
            }
        }

        AnalyzeCommand::Frequency {
            path,
            format,
            output,
        } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let freq = ps5_analysis::reports::build_frequency(&db);

            match format {
                OutputFormat::Terminal => terminal::print_frequency_terminal(&freq),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_nid_frequency(&freq, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_nid_frequency(&freq, w)
                }),
                _ => eprintln!("unsupported format for frequency"),
            }
        }

        AnalyzeCommand::Unresolved {
            path,
            format,
            output,
        } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let entries = ps5_analysis::reports::find_unresolved(&db);

            match format {
                OutputFormat::Terminal => terminal::print_unresolved_terminal(&entries),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_unresolved(&entries, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_unresolved(&entries, w)
                }),
                _ => eprintln!("unsupported format for unresolved"),
            }
        }

        AnalyzeCommand::Graph {
            path,
            include_nids,
            format,
            output,
        } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let graph = ps5_analysis::reports::build_graph(&db, include_nids);

            match format {
                OutputFormat::Dot => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::dot::export_graph(&graph, w)
                }),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_graph(&graph, w)
                }),
                _ => eprintln!("unsupported format for graph (use --format dot or --format json)"),
            }
        }

        AnalyzeCommand::Imports {
            path,
            format,
            output,
        } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let inv = ps5_analysis::reports::build_import_inventory(&ds);
                match format {
                    OutputFormat::Terminal => terminal::print_import_inventory_terminal(&inv),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        let json = serde_json::to_string_pretty(&inv).unwrap();
                        writeln!(w, "{json}")
                    }),
                    OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                        writeln!(w, "library,games,imports")?;
                        for e in &inv.entries {
                            writeln!(w, "{},{},{}", e.library, e.games, e.imports)?;
                        }
                        Ok(())
                    }),
                    _ => eprintln!("unsupported format for imports inventory"),
                }
            } else {
                let db = load_dataset_or_collect(&path, &catalog, include_modules);
                match format {
                    OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::csv::export_imports(&db, w)
                    }),
                    OutputFormat::Terminal => terminal::print_imports_terminal(&db),
                    _ => eprintln!(
                        "unsupported format for imports (use --format csv or --format terminal)"
                    ),
                }
            }
        }

        AnalyzeCommand::Unknown {
            path,
            format,
            output,
        } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_unknown_nids(&ds);
                match format {
                    OutputFormat::Terminal => terminal::print_unknown_nids_terminal(&report),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        let json = serde_json::to_string_pretty(&report).unwrap();
                        writeln!(w, "{json}")
                    }),
                    OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                        writeln!(w, "nid,count,libraries")?;
                        for e in &report.entries {
                            writeln!(w, "{},{},{}", e.nid_hash, e.count, e.libraries.join(";"))?;
                        }
                        Ok(())
                    }),
                    _ => eprintln!("unsupported format for unknown NIDs"),
                }
            } else {
                eprintln!(
                    "error: analyze unknown requires a dataset directory (run 'ps5rs scan' first)"
                );
                std::process::exit(1);
            }
        }

        AnalyzeCommand::LibraryVersions {
            path,
            format,
            output,
        } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_library_versions(&ds);
                match format {
                    OutputFormat::Terminal => terminal::print_library_versions_terminal(&report),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::json::export_library_versions(&report, w)
                    }),
                    OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::csv::export_library_versions(&report, w)
                    }),
                    _ => eprintln!("unsupported format for library versions"),
                }
            } else {
                eprintln!(
                    "error: analyze library-versions requires a dataset directory (run 'ps5rs scan' first)"
                );
                std::process::exit(1);
            }
        }

        AnalyzeCommand::Engines {
            path,
            format,
            output,
        } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_engine_hints(&ds);
                match format {
                    OutputFormat::Terminal => terminal::print_engines_terminal(&report),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::json::export_engine_hints(&report, w)
                    }),
                    _ => eprintln!(
                        "unsupported format for engine hints (use --format json or terminal)"
                    ),
                }
            } else {
                eprintln!(
                    "error: analyze engines requires a dataset directory (run 'ps5rs scan' first)"
                );
                std::process::exit(1);
            }
        }
    }
}
