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
    Inspect { file: PathBuf },
    Imports { file: PathBuf },
    Segments { file: PathBuf },
    Dynamic { file: PathBuf },
    Symbols { file: PathBuf },
    Nid { name: String },
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommand,
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
        #[arg(long, value_enum, default_value = "csv")]
        format: OutputFormat,
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<PathBuf>,
    },
    Collect {
        path: PathBuf,
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
        Commands::Inspect { file } => cmd_inspect(&file),
        Commands::Imports { file } => cmd_imports(&file),
        Commands::Segments { file } => cmd_segments(&file),
        Commands::Dynamic { file } => cmd_dynamic(&file),
        Commands::Symbols { file } => cmd_symbols(&file),
        Commands::Nid { name } => cmd_nid(&name),
        Commands::Analyze { command } => cmd_analyze(command),
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

fn lib_id_from_nid(nid: &str) -> Option<u16> {
    if let Some(hash_end) = nid.find('#') {
        let lib_str = &nid[hash_end + 1..];
        const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
        let mut val: u16 = 0;
        for ch in lib_str.bytes() {
            if let Some(pos) = B64.iter().position(|&b| b == ch) {
                val = val * 64 + pos as u16;
            } else {
                return None;
            }
        }
        Some(val)
    } else {
        None
    }
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

fn cmd_analyze(command: AnalyzeCommand) {
    let catalog = load_catalog();
    let options = ps5_analysis::CollectorOptions::default();

    match command {
        AnalyzeCommand::Collect { path, output } => {
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
            eprintln!("Collected {} games", db.games.len());

            write_to_output_or_stdout(&output, &|w| {
                ps5_analysis::export::json::export_analysis(&db, w)
            });
        }

        AnalyzeCommand::Stats { path, format, output } => {
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
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
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
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
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
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
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
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
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);
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
            eprintln!("Collecting analysis from {}...", path.display());
            let db = ps5_analysis::collect(&path, &catalog, &options);

            match format {
                OutputFormat::Csv => write_to_output_or_stdout(&output, &|w| {
                    ps5_analysis::export::csv::export_imports(&db, w)
                }),
                OutputFormat::Terminal => print_imports_terminal(&db),
                _ => eprintln!("unsupported format for imports (use --format csv or --format terminal)"),
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
fn cmd_inspect(path: &PathBuf) {
    let data = load_file(path);
    println!("ps5rs v{} — PS5 binary inspector", env!("CARGO_PKG_VERSION"));
    println!("File: {}", path.display());
    println!("Size: {} bytes", data.len());
    println!();

    match ps5_self::SelfImage::parse(&data) {
        Ok(img) => {
            println!("Platform: {:?}", img.platform);
            if img.is_self() {
                println!("SELF segments: {}", img.segments.len());
                for (i, seg) in img.segments.iter().enumerate() {
                    let flags = if seg.is_data() { "DATA" } else if seg.is_encrypted() { "ENCRYPTED" } else { "CODE" };
                    println!("  [{i}] offset={:#x} file_size={:#x} mem_size={:#x} flags={}",
                        seg.file_offset, seg.file_size, seg.mem_size, flags);
                }
            }
            println!();

            let elf = &img.elf;
            println!("ELF type: {:#x}", elf.header.e_type);
            println!("Machine: {:#x}", elf.header.e_machine);
            println!("Entry point: {:#x}", elf.header.e_entry);
            println!("Program headers: {}", elf.program_headers.len());
            println!("Dynamic entries: {}", elf.dynamic_entries.len());
            println!("Symbols: {}", elf.symbols.len());
            println!("Relocations: {}", elf.relocations.len());
            if let Some(ref tls) = elf.tls {
                println!("TLS: vaddr={:#x} filesz={:#x} memsz={:#x}", tls.vaddr, tls.filesz, tls.memsz);
            }

            let imports: Vec<_> = elf.symbols.iter().filter(|s| s.is_import).collect();
            println!("Imports: {}", imports.len());

            let catalog = load_catalog();
            let mut lib_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for sym in &imports {
                let parts: Vec<&str> = sym.resolved_name.split('#').collect();
                let nid = parts[0];
                let lib_name = if parts.len() >= 2 {
                    lib_id_from_nid(&sym.resolved_name)
                        .and_then(|id| elf.import_libs.get(&id).cloned())
                        .unwrap_or_else(|| format!("lib_{}", parts[1]))
                } else {
                    "?".to_string()
                };
                let resolved = catalog.resolve(nid).unwrap_or("?");
                *lib_counts.entry(format!("{lib_name}: {resolved}")).or_insert(0) += 1;
            }

            if !lib_counts.is_empty() {
                println!("\nBy library + resolved name:");
                let mut sorted: Vec<_> = lib_counts.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (lib, count) in sorted {
                    println!("  {lib}: {count}");
                }
            }
        }
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    }
}

#[allow(clippy::print_literal)]
fn cmd_imports(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    let catalog = ps5_nid::Catalog::new();
    let imports: Vec<_> = img.elf.symbols.iter().filter(|s| s.is_import).collect();

    println!("Imports from {} ({})", path.display(), imports.len());
    println!("{:<64} {:<16} {}", "NID", "Resolved", "Library");
    println!("{}", "-".repeat(100));

    for sym in &imports {
        let parts: Vec<&str> = sym.resolved_name.split('#').collect();
        let nid = parts[0];
        let lib_name = if parts.len() >= 2 {
            lib_id_from_nid(&sym.resolved_name)
                .and_then(|id| img.elf.import_libs.get(&id).cloned())
                .unwrap_or_else(|| format!("lib_{}", parts[1]))
        } else {
            "?".to_string()
        };

        let resolved = catalog.resolve(nid).unwrap_or("?");
        println!("{:<64} {:<16} {}", nid, resolved, lib_name);
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
            0x17 => "DT_JMPREL",
            0x19 => "DT_INIT_ARRAY",
            0x1b => "DT_INIT_ARRAYSZ",
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
