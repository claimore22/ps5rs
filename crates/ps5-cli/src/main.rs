use clap::{Parser, Subcommand, ValueHint};
use std::io::Write;
use std::path::PathBuf;

const NIDS_CSV: &str = include_str!("../../../data/nids.csv");

#[derive(Parser)]
#[command(name = "ps5rs", version, about = "PS5 binary inspector")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Inspect {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Imports {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Segments { file: PathBuf },
    Dynamic { file: PathBuf },
    Symbols { file: PathBuf },
    Nid { name: String },
    Scan {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long)]
        include_modules: bool,
    },
    Analyze {
        #[arg(long)]
        include_modules: bool,
        #[command(subcommand)]
        command: AnalyzeCommand,
    },
    Extract {
        file: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    BatchExtract {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        output: PathBuf,
        #[arg(long)]
        include_modules: bool,
    },
    Validate {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum AnalyzeCommand {
    Stats {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Heatmap {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Frequency {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Unresolved {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Graph {
        path: PathBuf,
        #[arg(long)]
        include_nids: bool,
        #[arg(long, value_enum, default_value = "dot")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Imports {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Unknown {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Collect {
        path: PathBuf,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    LibraryVersions {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Engines {
        path: PathBuf,
        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Terminal,
    Csv,
    Json,
    Dot,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { file, json, output } => cmd_inspect(&file, json, &output),
        Commands::Imports { file, json, output } => cmd_imports(&file, json, &output),
        Commands::Segments { file } => cmd_segments(&file),
        Commands::Dynamic { file } => cmd_dynamic(&file),
        Commands::Symbols { file } => cmd_symbols(&file),
        Commands::Nid { name } => cmd_nid(&name),
        Commands::Scan { path, output, include_modules } => {
            cmd_scan(&path, &output, include_modules)
        }
        Commands::Analyze { include_modules, command } => cmd_analyze(command, include_modules),
        Commands::Extract { file, output } => cmd_extract(&file, &output),
        Commands::BatchExtract { path, output, include_modules } => {
            cmd_batch_extract(&path, &output, include_modules)
        }
        Commands::Validate { path, output } => cmd_validate(&path, &output),
    }
}

fn load_file(path: &PathBuf) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        std::process::exit(1);
    })
}

fn load_catalog() -> ps5_nid::Catalog {
    let mut cat = ps5_nid::Catalog::new();
    let loaded = cat.load_nids_csv(NIDS_CSV);
    eprintln!("Loaded {} NID mappings from built-in catalog", loaded);
    cat
}

fn write_to_output_or_stdout(
    output: &Option<PathBuf>,
    write_fn: &dyn Fn(&mut dyn Write) -> std::io::Result<()>,
) {
    if let Some(out_path) = output {
        let mut file = std::fs::File::create(out_path).unwrap_or_else(|e| {
            eprintln!("error: cannot create {}: {e}", out_path.display());
            std::process::exit(1);
        });
        write_fn(&mut file).unwrap();
        eprintln!("Written to {}", out_path.display());
    } else {
        let stdout = std::io::stdout();
        write_fn(&mut stdout.lock()).unwrap();
    }
}

fn is_dataset_dir(path: &std::path::Path) -> bool {
    path.join("manifest.json").exists() && path.join("images").is_dir()
}

fn osabi_name(osabi: u8) -> &'static str {
    use ps5_format::elf_constants::*;
    match osabi {
        ELFOSABI_NONE => "UNIX System V",
        ELFOSABI_HPUX => "HP-UX",
        ELFOSABI_NETBSD => "NetBSD",
        ELFOSABI_LINUX => "Linux",
        ELFOSABI_FREEBSD => "UNIX - FreeBSD",
        ELFOSABI_OPENBSD => "OpenBSD",
        _ => "Unknown",
    }
}

fn e_version_name(v: u32) -> &'static str {
    match v {
        1 => "Current",
        _ => "Unknown",
    }
}

fn dataset_to_database_real(ds: &ps5_analysis::AnalysisDataset) -> ps5_analysis::AnalysisDatabase {
    let games: Vec<ps5_analysis::GameAnalysis> = ds
        .images
        .iter()
        .map(|(name, doc)| {
            let img = &doc.image;

            let platform = match img.platform {
                ps5_image::Platform::Ps4 => ps5_analysis::Platform::Ps4,
                ps5_image::Platform::Ps5 => ps5_analysis::Platform::Ps5,
                ps5_image::Platform::RawElf => ps5_analysis::Platform::RawElf,
                ps5_image::Platform::Unknown => ps5_analysis::Platform::Unknown,
            };

            let imports: Vec<ps5_analysis::ImportInfo> = img
                .imports
                .iter()
                .map(|imp| ps5_analysis::ImportInfo {
                    nid_hash: imp.nid_hash.clone(),
                    resolved_name: imp
                        .resolved_name
                        .clone()
                        .unwrap_or_else(|| "?".into()),
                    library_id: imp.library_id,
                    library_name: imp.library_name.clone(),
                })
                .collect();

            let import_libs: Vec<ps5_analysis::LibInfo> = img
                .import_libs
                .iter()
                .map(|(id, name)| ps5_analysis::LibInfo {
                    id: *id,
                    name: name.clone(),
                })
                .collect();

            ps5_analysis::GameAnalysis {
                name: name.clone(),
                path: String::new(),
                sha256: img.sha256.clone(),
                file_size: img.file_size,
                platform,
                entry_point: img.entry_point,
                is_self: img.is_self,
                imports,
                import_libs,
                needed_files: img.needed_files.clone(),
                num_relocations: img.relocations.len(),
                num_symbols: img.imports.len() + img.exports.len(),
                has_tls: img.tls.is_some(),
            }
        })
        .collect();

    ps5_analysis::AnalysisDatabase {
        schema_version: 1,
        tool: "ps5rs".to_string(),
        games,
    }
}

fn load_dataset_or_collect(
    path: &std::path::Path,
    catalog: &ps5_nid::Catalog,
    include_modules: bool,
) -> ps5_analysis::AnalysisDatabase {
    if is_dataset_dir(path) {
        eprintln!("Loading dataset from {}...", path.display());
        let ds = ps5_analysis::AnalysisDataset::open(path).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
        eprintln!("Loaded {} images from dataset", ds.images.len());
        dataset_to_database_real(&ds)
    } else {
        eprintln!("Collecting analysis from {}...", path.display());
        let options = ps5_analysis::CollectorOptions {
            include_prx: include_modules,
        };
        ps5_analysis::collect(path, catalog, &options)
    }
}

fn cmd_scan(path: &std::path::Path, output: &std::path::Path, include_modules: bool) {
    let catalog = load_catalog();
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
    eprintln!(
        "Dataset schema version: {}",
        result.manifest.schema_version
    );
}

fn cmd_analyze(command: AnalyzeCommand, include_modules: bool) {
    let catalog = load_catalog();

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

        AnalyzeCommand::Stats { path, format, output } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let stats = ps5_analysis::reports::compute_stats(&db);

            match format {
                OutputFormat::Terminal => print_stats_terminal(&stats),
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

        AnalyzeCommand::Heatmap { path, format, output } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let heatmap = ps5_analysis::reports::build_heatmap(&db);

            match format {
                OutputFormat::Terminal => print_heatmap_terminal(&heatmap),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_heatmap(&heatmap, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_heatmap(&heatmap, w)
                }),
                _ => eprintln!("unsupported format for heatmap"),
            }
        }

        AnalyzeCommand::Frequency { path, format, output } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let freq = ps5_analysis::reports::build_frequency(&db);

            match format {
                OutputFormat::Terminal => print_frequency_terminal(&freq),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_nid_frequency(&freq, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_nid_frequency(&freq, w)
                }),
                _ => eprintln!("unsupported format for frequency"),
            }
        }

        AnalyzeCommand::Unresolved { path, format, output } => {
            let db = load_dataset_or_collect(&path, &catalog, include_modules);
            let entries = ps5_analysis::reports::find_unresolved(&db);

            match format {
                OutputFormat::Terminal => print_unresolved_terminal(&entries),
                OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::json::export_unresolved(&entries, w)
                }),
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_unresolved(&entries, w)
                }),
                _ => eprintln!("unsupported format for unresolved"),
            }
        }

        AnalyzeCommand::Graph { path, include_nids, format, output } => {
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

        AnalyzeCommand::Imports { path, format, output } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let inv = ps5_analysis::reports::build_import_inventory(&ds);
                match format {
                    OutputFormat::Terminal => print_import_inventory_terminal(&inv),
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
                    OutputFormat::Terminal => print_imports_terminal(&db),
                    _ => eprintln!("unsupported format for imports (use --format csv or --format terminal)"),
                }
            }
        }

        AnalyzeCommand::Unknown { path, format, output } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_unknown_nids(&ds);
                match format {
                    OutputFormat::Terminal => print_unknown_nids_terminal(&report),
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
                eprintln!("error: analyze unknown requires a dataset directory (run 'ps5rs scan' first)");
                std::process::exit(1);
            }
        }

        AnalyzeCommand::LibraryVersions { path, format, output } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_library_versions(&ds);
                match format {
                    OutputFormat::Terminal => print_library_versions_terminal(&report),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::json::export_library_versions(&report, w)
                    }),
                    OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::csv::export_library_versions(&report, w)
                    }),
                    _ => eprintln!("unsupported format for library versions"),
                }
            } else {
                eprintln!("error: analyze library-versions requires a dataset directory (run 'ps5rs scan' first)");
                std::process::exit(1);
            }
        }

        AnalyzeCommand::Engines { path, format, output } => {
            if is_dataset_dir(&path) {
                let ds = ps5_analysis::AnalysisDataset::open(&path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
                let report = ps5_analysis::reports::build_engine_hints(&ds);
                match format {
                    OutputFormat::Terminal => print_engines_terminal(&report),
                    OutputFormat::Json => write_to_output_or_stdout(&output, &|w| {
                        ps5_analysis::export::json::export_engine_hints(&report, w)
                    }),
                    _ => eprintln!("unsupported format for engine hints (use --format json or terminal)"),
                }
            } else {
                eprintln!("error: analyze engines requires a dataset directory (run 'ps5rs scan' first)");
                std::process::exit(1);
            }
        }
    }
}

fn print_stats_terminal(stats: &ps5_analysis::AnalysisStats) {
    println!("Analysis Statistics");
    println!("{}", "=".repeat(40));
    println!("  Games analyzed:     {}", stats.total_games);
    println!("  Total imports:      {}", stats.total_imports);
    println!("  Unique NIDs:        {}", stats.unique_nids);
    println!("  Unique libraries:   {}", stats.unique_libs);
    println!("  Resolution rate:    {:.1}%", stats.resolution_rate);
    if let Some(ref name) = stats.most_common_nid {
        println!("  Most common NID:    {} ({}) — {} imports",
            name,
            stats.most_common_nid_name.as_deref().unwrap_or("?"),
            stats.most_common_nid_count);
    }
    if let Some(ref lib) = stats.most_used_lib {
        println!("  Most used library:  {} — {} imports", lib, stats.most_used_lib_count);
    }
}

fn print_heatmap_terminal(heatmap: &ps5_analysis::LibraryHeatmap) {
    if heatmap.libraries.is_empty() {
        println!("No libraries found.");
        return;
    }

    let max_lib_width = heatmap.libraries.iter().map(|l| l.len()).max().unwrap_or(20).min(30);
    let game_width = 10;

    print!("{:<width$}", "Library", width = max_lib_width + 2);
    for game in &heatmap.games {
        let short = if game.len() > game_width {
            &game[..game_width]
        } else {
            game
        };
        print!("{:>width$} ", short, width = game_width);
    }
    println!();
    println!("{}", "-".repeat(max_lib_width + 2 + (game_width + 1) * heatmap.games.len()));

    for (i, lib) in heatmap.libraries.iter().enumerate() {
        let display_lib = if lib.len() > max_lib_width { &lib[..max_lib_width] } else { lib };
        print!("{:<width$} ", display_lib, width = max_lib_width + 2);
        for count in &heatmap.matrix[i] {
            if *count > 0 {
                print!("{:>width$} ", count, width = game_width);
            } else {
                print!("{:>width$} ", ".", width = game_width);
            }
        }
        println!();
    }
}

#[allow(clippy::print_literal)]
fn print_frequency_terminal(freq: &ps5_analysis::NidFrequency) {
    println!("NID Frequency (top 50 of {} unique, {} total imports)",
        freq.unique_nids, freq.total_imports);
    println!("{}", "=".repeat(80));
    println!("{:<44} {:<20} {:>6}  {}", "NID", "Name", "Count", "Games");
    println!("{}", "-".repeat(80));

    for entry in freq.entries.iter().take(50) {
        let games_str = if entry.games.len() > 3 {
            format!("{}, +{} more", entry.games[..3].join(", "), entry.games.len() - 3)
        } else {
            entry.games.join(", ")
        };
        println!("{:<44} {:<20} {:>6}  {}",
            entry.nid_hash, entry.name, entry.count, games_str);
    }
}

#[allow(clippy::print_literal)]
fn print_unresolved_terminal(entries: &[ps5_analysis::UnresolvedEntry]) {
    println!("Unresolved NIDs ({} total)", entries.len());
    println!("{}", "=".repeat(60));
    println!("{:<20} {:<20} {}", "Game", "Library", "NID");
    println!("{}", "-".repeat(60));

    for entry in entries.iter().take(100) {
        println!("{:<20} {:<20} {}", entry.game, entry.library, entry.nid_hash);
    }
    if entries.len() > 100 {
        println!("... and {} more", entries.len() - 100);
    }
}

#[allow(clippy::print_literal)]
fn print_imports_terminal(db: &ps5_analysis::AnalysisDatabase) {
    println!("Raw imports ({} games, {} total imports)",
        db.games.len(),
        db.games.iter().map(|g| g.imports.len()).sum::<usize>());
    println!("{}", "=".repeat(80));
    println!("{:<20} {:<20} {:<44} {}", "Game", "Library", "NID", "Name");
    println!("{}", "-".repeat(80));

    for game in &db.games {
        for imp in game.imports.iter().take(5) {
            println!("{:<20} {:<20} {:<44} {}",
                game.name, imp.library_name, imp.nid_hash, imp.resolved_name);
        }
        if game.imports.len() > 5 {
            println!("{:<20} ... and {} more imports", "", game.imports.len() - 5);
        }
    }
}

#[allow(clippy::print_literal)]
fn print_import_inventory_terminal(inv: &ps5_analysis::reports::LibraryInventory) {
    println!("Import Inventory ({} games)", inv.total_games);
    println!("{}", "=".repeat(60));
    println!("{:<30} {:>8} {:>10}", "Library", "Games", "Imports");
    println!("{}", "-".repeat(60));

    for entry in &inv.entries {
        println!("{:<30} {:>8} {:>10}", entry.library, entry.games, entry.imports);
    }
}

#[allow(clippy::print_literal)]
fn print_unknown_nids_terminal(report: &ps5_analysis::reports::UnknownNidReport) {
    println!(
        "Unknown NIDs ({} unique, {} total, {:.1}% of {} imports)",
        report.entries.len(),
        report.total_unknown,
        if report.total_imports > 0 {
            report.total_unknown as f64 / report.total_imports as f64 * 100.0
        } else {
            0.0
        },
        report.total_imports,
    );
    println!("{}", "=".repeat(70));
    println!("{:<44} {:>6}  {}", "NID", "Count", "Libraries");
    println!("{}", "-".repeat(70));

    for entry in report.entries.iter().take(100) {
        println!(
            "{:<44} {:>6}  {}",
            entry.nid_hash,
            entry.count,
            entry.libraries.join(", ")
        );
    }
    if report.entries.len() > 100 {
        println!("... and {} more", report.entries.len() - 100);
    }
}

#[allow(clippy::print_literal)]
fn cmd_inspect(path: &PathBuf, json: bool, output: &Option<PathBuf>) {
    let data = load_file(path);
    let sha256 = ps5_format::sha256_hex(&data);
    let catalog = load_catalog();
    let image = ps5_image::BinaryImageBuilder::build_from_file(data, &sha256, &catalog);

    if json {
        write_to_output_or_stdout(output, &|w| {
            ps5_image::json::export_json(&image, w).map_err(std::io::Error::other)
        });
        return;
    }

    println!("ps5rs v{} — PS5 binary inspector", env!("CARGO_PKG_VERSION"));
    println!("File: {}", path.display());
    println!("Size: {} bytes", image.file_size);
    println!();

    println!("Platform: {}", image.platform);
    println!("SELF: {}", image.is_self);
    println!("SHA-256: {}", &image.sha256[..16]);
    println!("Entry point: {:#x}", image.entry_point);
    println!("ELF type: {:#x}", image.metadata.elf_type);
    println!("ELF flags: {:#x}", image.metadata.elf_flags);
    println!("OS/ABI: {} ({:#x})", osabi_name(image.metadata.osabi), image.metadata.osabi);
    println!("ABI Version: {}", image.metadata.ei_abi_version);
    println!("ELF Version: {} ({})", image.metadata.e_version, e_version_name(image.metadata.e_version));
    if let Some(ref bid) = image.metadata.build_id {
        println!("Build ID: {bid}");
    }
    println!("Segments: {}", image.segments.len());
    println!("Sections: {}", image.metadata.sections.len());
    println!("Imports: {}", image.imports.len());
    println!("Exports: {}", image.exports.len());
    println!("Relocations: {}", image.relocations.len());
    println!("Dynamic entries: {}", image.dynamic_entries.len());
    if !image.lib_versions.is_empty() {
        println!("Library versions: {}", image.lib_versions.len());
    }
    if let Some(ref tls) = image.tls {
        println!("TLS: vaddr={:#x} filesz={:#x} memsz={:#x}", tls.vaddr, tls.filesz, tls.memsz);
    }
    if image.init_va != 0 {
        println!("init: {:#x}", image.init_va);
    }
    if image.fini_va != 0 {
        println!("fini: {:#x}", image.fini_va);
    }

    if !image.import_libs.is_empty() {
        println!("\nImport libraries:");
        for (id, name) in &image.import_libs {
            println!("  [{id}] {name}");
        }
    }

    if !image.needed_files.is_empty() {
        println!("\nNeeded files:");
        for f in &image.needed_files {
            println!("  {f}");
        }
    }

    if !image.lib_versions.is_empty() {
        println!("\nLibrary versions:");
        for lv in &image.lib_versions {
            println!("  {:<36} {}", lv.name, lv.version_string);
        }
    }

    if !image.imports.is_empty() {
        let mut lib_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for imp in &image.imports {
            let label = format!("{}: {}", imp.library_name,
                imp.resolved_name.as_deref().unwrap_or("?"));
            *lib_counts.entry(label).or_insert(0) += 1;
        }
        println!("\nBy library + resolved name:");
        let mut sorted: Vec<_> = lib_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (lib, count) in sorted {
            println!("  {lib}: {count}");
        }
    }
}

#[allow(clippy::print_literal)]
fn cmd_imports(path: &PathBuf, json: bool, output: &Option<PathBuf>) {
    let data = load_file(path);
    let sha256 = ps5_format::sha256_hex(&data);
    let catalog = ps5_nid::Catalog::new();
    let image = ps5_image::BinaryImageBuilder::build_from_file(data, &sha256, &catalog);

    if json {
        write_to_output_or_stdout(output, &|w| {
            ps5_image::json::export_json(&image, w).map_err(std::io::Error::other)
        });
        return;
    }

    println!("Imports from {} ({})", path.display(), image.imports.len());
    println!("{:<64} {:<16} {}", "NID", "Resolved", "Library");
    println!("{}", "-".repeat(100));

    for imp in &image.imports {
        let resolved = imp.resolved_name.as_deref().unwrap_or("?");
        println!("{:<64} {:<16} {}", imp.nid_hash, resolved, imp.library_name);
    }
}

#[allow(clippy::print_literal)]
fn cmd_segments(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    println!("Program headers from {}", path.display());
    println!("{:<4} {:<16} {:<12} {:<18} {:<18} {:<18} {:<18} {}", "#", "Type", "Flags", "Offset", "VAddr", "FileSz", "MemSz", "Mapped File Offset");
    println!("{}", "-".repeat(140));

    for (i, ph) in img.elf.program_headers.iter().enumerate() {
        let flags_str = format!("{}{}{}",
            if ph.is_readable() { "R" } else { "-" },
            if ph.is_writable() { "W" } else { "-" },
            if ph.is_executable() { "X" } else { "-" });

        println!("{:<4} {:<16} {:<12} {:#018x} {:#018x} {:#018x} {:#018x}",
            i, ph.type_name(), flags_str, ph.p_offset, ph.p_vaddr, ph.p_filesz, ph.p_memsz);
    }

    if !img.segments.is_empty() {
        println!("\nSELF data segments:");
        for (i, seg) in img.segments.iter().enumerate() {
            println!("  [{i}] phdr_index={} offset={:#x} file_size={:#x} mem_size={:#x}",
                seg.phdr_index(), seg.file_offset, seg.file_size, seg.mem_size);
        }
    }
}

#[allow(clippy::print_literal)]
fn cmd_dynamic(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    println!("Dynamic entries from {}", path.display());
    println!("{:<24} {:<20} {}", "Tag", "Tag (decimal)", "Value");
    println!("{}", "-".repeat(70));

    for entry in &img.elf.dynamic_entries {
        let tag_name = match entry.d_tag {
            0 => "DT_NULL",
            1 => "DT_NEEDED",
            2 => "DT_PLTRELSZ",
            3 => "DT_PLTGOT",
            5 => "DT_STRTAB",
            6 => "DT_SYMTAB",
            7 => "DT_RELA",
            8 => "DT_RELASZ",
            0xa => "DT_STRSZ",
            0xb => "DT_SYMENT",
            0xc => "DT_INIT",
            0xd => "DT_FINI",
            0x17 => "DT_JMPREL",
            0x19 => "DT_INIT_ARRAY",
            0x1a => "DT_FINI_ARRAY",
            0x1b => "DT_INIT_ARRAYSZ",
            0x1c => "DT_FINI_ARRAYSZ",
            0x61000029 => "DT_SCE_JMPREL",
            0x6100002D => "DT_SCE_PLTRELSZ",
            0x6100002F => "DT_SCE_RELA",
            0x61000031 => "DT_SCE_RELASZ",
            0x61000035 => "DT_SCE_STRTAB",
            0x61000037 => "DT_SCE_STRSZ",
            0x61000039 => "DT_SCE_SYMTAB",
            0x6100003F => "DT_SCE_SYMTABSZ",
            0x61000045 => "DT_SCE_NEEDED_MOD",
            0x61000049 => "DT_SCE_NEEDED_LIB",
            _ => "?",
        };
        println!("{:<24} {:<20} {:#x}", tag_name, entry.d_tag, entry.d_val);
    }

    if !img.elf.import_libs.is_empty() {
        println!("\nImport libraries:");
        for (id, name) in &img.elf.import_libs {
            println!("  [{id}] {name}");
        }
    }
}

#[allow(clippy::print_literal)]
fn cmd_symbols(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    let elf = &img.elf;
    println!("Symbols from {} ({})", path.display(), elf.symbols.len());
    println!("symtab_offset={:#x} strtab_offset={:#x} strtab_size={:#x} symtab_size={:#x}",
        elf.symtab_offset, elf.strtab_offset, elf.strtab_size, elf.symtab_size);
    println!();
    println!("{:<6} {:<10} {:<10} {:<6} {:<18} {:<18} {:<8} {}",
        "#", "shndx", "info", "bind", "value", "size", "name_off", "name");
    println!("{}", "-".repeat(120));

    let limit = elf.symbols.len().min(50);
    for (i, sym) in elf.symbols.iter().take(limit).enumerate() {
        let bind = sym.st_info >> 4;
        let bind_str = match bind {
            0 => "LOCAL",
            1 => "GLOBAL",
            2 => "WEAK",
            _ => "??",
        };
        println!("{:<6} {:<#10x} {:<#10x} {:<6} {:#018x} {:#018x} {:<#10x} \"{}\"",
            i, sym.st_shndx, sym.st_info, bind_str, sym.st_value, sym.st_size, sym.st_name, sym.resolved_name);
    }

    if elf.symbols.len() > limit {
        println!("... and {} more", elf.symbols.len() - limit);
    }
}

fn cmd_nid(name: &str) {
    let nid = ps5_nid::hash(name);
    println!("{name} -> {nid}");
}

fn cmd_extract(path: &PathBuf, output: &Option<PathBuf>) {
    let data = load_file(path);

    let result = ps5_self::extract::extract_elf(&data).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let out_path = output.clone().unwrap_or_else(|| {
        let mut p = path.clone();
        let stem = p.file_stem().unwrap_or_default().to_owned();
        p.set_file_name(stem);
        p.set_extension("elf");
        p
    });

    std::fs::write(&out_path, &result.elf).unwrap_or_else(|e| {
        eprintln!("error: cannot write {}: {e}", out_path.display());
        std::process::exit(1);
    });

    println!("Format:  {}", if result.was_self { "SELF" } else { "Raw ELF (passthrough)" });
    println!("Output:  {}", out_path.display());
    println!("Size:    {} bytes", result.elf.len());

    if result.encrypted_segments > 0 || result.compressed_segments > 0 {
        println!();
        println!("Warnings:");
        if result.encrypted_segments > 0 {
            println!("  {} encrypted segment(s) — data may be invalid", result.encrypted_segments);
        }
        if result.compressed_segments > 0 {
            println!("  {} compressed segment(s) — data may be invalid", result.compressed_segments);
        }
    }
}

fn cmd_batch_extract(path: &std::path::Path, output: &std::path::Path, include_modules: bool) {
    let options = ps5_analysis::BatchExtractOptions {
        include_modules,
    };

    eprintln!("Batch extracting from {}...", path.display());
    let result = ps5_analysis::batch_extract(path, output, &options).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    eprintln!();
    eprintln!(
        "Extracted {}/{} games to {}",
        result.manifest.succeeded,
        result.manifest.total,
        result.output_dir.display()
    );

    if !result.failures.is_empty() {
        eprintln!();
        eprintln!("Failed:");
        for (game, reason) in &result.failures {
            eprintln!("  {game}: {reason}");
        }
    }
}

fn cmd_validate(path: &std::path::Path, output: &Option<PathBuf>) {
    if !is_dataset_dir(path) {
        eprintln!("error: {} is not a dataset directory (run 'ps5rs scan' first)", path.display());
        std::process::exit(1);
    }

    let ds = ps5_analysis::AnalysisDataset::open(path).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let extracted_dir = path.join("analysis").join("extracted");
    let extracted_path = if extracted_dir.exists() {
        Some(extracted_dir.as_path())
    } else {
        None
    };

    let mut report = ps5_analysis::reports::validate_dataset(&ds, extracted_path);
    report.dataset_path = path.display().to_string();

    eprintln!("Validated {} games", report.total_games);
    eprintln!("  Valid ELF:    {}/{}", report.elf_valid, report.total_games);
    eprintln!("  Lib versions: {}", report.libversion_found);
    eprintln!("  NID resolution: {:.1}%", report.nid_resolution_avg);

    if !report.parse_errors.is_empty() {
        eprintln!("\nParse errors:");
        for err in &report.parse_errors {
            eprintln!("  {}: {}", err.name, err.error);
        }
    }

    write_to_output_or_stdout(output, &|w| {
        ps5_analysis::export::json::export_validation(&report, w)
    });
}

#[allow(clippy::print_literal)]
fn print_library_versions_terminal(report: &ps5_analysis::LibraryVersionReport) {
    if report.entries.is_empty() {
        println!("No library versions found.");
        return;
    }

    println!(
        "Library Versions ({} unique, across {} games)",
        report.entries.len(),
        report.entries.iter().map(|e| e.game_count).max().unwrap_or(0)
    );
    println!("{}", "=".repeat(80));
    println!("{:<36} {:>12} {:>12}  {}", "Library", "Version", "Games", "Game List");
    println!("{}", "-".repeat(80));

    for entry in &report.entries {
        let games_str = if entry.games.len() > 3 {
            format!("{}, +{} more", entry.games[..3].join(", "), entry.games.len() - 3)
        } else {
            entry.games.join(", ")
        };
        println!(
            "{:<36} {:>12} {:>12}  {}",
            entry.library, entry.version_string, entry.game_count, games_str
        );
    }
}

#[allow(clippy::print_literal)]
fn print_engines_terminal(report: &ps5_analysis::EngineHintReport) {
    println!("Engine Hints ({} games)", report.games.len());
    println!("{}", "=".repeat(70));
    println!("{:<30} {:<16} {}", "Game", "Engine", "SCE Libraries");
    println!("{}", "-".repeat(70));

    for game in &report.games {
        let engine = game.engines.join(", ");
        let sce = if game.sce_libraries.len() > 4 {
            format!("{}, +{} more", game.sce_libraries[..4].join(", "), game.sce_libraries.len() - 4)
        } else {
            game.sce_libraries.join(", ")
        };
        println!("{:<30} {:<16} {}", game.name, engine, sce);
    }
}
