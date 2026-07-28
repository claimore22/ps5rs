#[allow(clippy::print_literal)]
pub(crate) fn print_stats_terminal(stats: &ps5_analysis::AnalysisStats) {
    println!("Analysis Statistics");
    println!("{}", "=".repeat(40));
    println!("  Games analyzed:     {}", stats.total_games);
    println!("  Total imports:      {}", stats.total_imports);
    println!("  Unique NIDs:        {}", stats.unique_nids);
    println!("  Unique libraries:   {}", stats.unique_libs);
    println!("  Resolution rate:    {:.1}%", stats.resolution_rate);
    if let Some(ref name) = stats.most_common_nid {
        println!(
            "  Most common NID:    {} ({}) — {} imports",
            name,
            stats.most_common_nid_name.as_deref().unwrap_or("?"),
            stats.most_common_nid_count
        );
    }
    if let Some(ref lib) = stats.most_used_lib {
        println!(
            "  Most used library:  {} — {} imports",
            lib, stats.most_used_lib_count
        );
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn print_heatmap_terminal(heatmap: &ps5_analysis::LibraryHeatmap) {
    if heatmap.libraries.is_empty() {
        println!("No libraries found.");
        return;
    }

    let max_lib_width = heatmap
        .libraries
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(20)
        .min(30);
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
    println!(
        "{}",
        "-".repeat(max_lib_width + 2 + (game_width + 1) * heatmap.games.len())
    );

    for (i, lib) in heatmap.libraries.iter().enumerate() {
        let display_lib = if lib.len() > max_lib_width {
            &lib[..max_lib_width]
        } else {
            lib
        };
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
pub(crate) fn print_frequency_terminal(freq: &ps5_analysis::NidFrequency) {
    println!(
        "NID Frequency (top 50 of {} unique, {} total imports)",
        freq.unique_nids, freq.total_imports
    );
    println!("{}", "=".repeat(80));
    println!("{:<44} {:<20} {:>6}  {}", "NID", "Name", "Count", "Games");
    println!("{}", "-".repeat(80));

    for entry in freq.entries.iter().take(50) {
        let games_str = if entry.games.len() > 3 {
            format!(
                "{}, +{} more",
                entry.games[..3].join(", "),
                entry.games.len() - 3
            )
        } else {
            entry.games.join(", ")
        };
        println!(
            "{:<44} {:<20} {:>6}  {}",
            entry.nid_hash, entry.name, entry.count, games_str
        );
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn print_unresolved_terminal(entries: &[ps5_analysis::UnresolvedEntry]) {
    println!("Unresolved NIDs ({} total)", entries.len());
    println!("{}", "=".repeat(60));
    println!("{:<20} {:<20} {}", "Game", "Library", "NID");
    println!("{}", "-".repeat(60));

    for entry in entries.iter().take(100) {
        println!(
            "{:<20} {:<20} {}",
            entry.game, entry.library, entry.nid_hash
        );
    }
    if entries.len() > 100 {
        println!("... and {} more", entries.len() - 100);
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn print_imports_terminal(db: &ps5_analysis::AnalysisDatabase) {
    println!(
        "Raw imports ({} games, {} total imports)",
        db.games.len(),
        db.games.iter().map(|g| g.imports.len()).sum::<usize>()
    );
    println!("{}", "=".repeat(80));
    println!("{:<20} {:<20} {:<44} {}", "Game", "Library", "NID", "Name");
    println!("{}", "-".repeat(80));

    for game in &db.games {
        let gname = game.display_name.as_deref().unwrap_or(&game.name);
        for imp in game.imports.iter().take(5) {
            println!(
                "{:<20} {:<20} {:<44} {}",
                gname, imp.library_name, imp.nid_hash, imp.resolved_name
            );
        }
        if game.imports.len() > 5 {
            println!("{:<20} ... and {} more imports", "", game.imports.len() - 5);
        }
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn print_import_inventory_terminal(inv: &ps5_analysis::reports::LibraryInventory) {
    println!("Import Inventory ({} games)", inv.total_games);
    println!("{}", "=".repeat(60));
    println!("{:<30} {:>8} {:>10}", "Library", "Games", "Imports");
    println!("{}", "-".repeat(60));

    for entry in &inv.entries {
        println!(
            "{:<30} {:>8} {:>10}",
            entry.library, entry.games, entry.imports
        );
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn print_unknown_nids_terminal(report: &ps5_analysis::reports::UnknownNidReport) {
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
pub(crate) fn print_library_versions_terminal(report: &ps5_analysis::LibraryVersionReport) {
    if report.entries.is_empty() {
        println!("No library versions found.");
        return;
    }

    println!(
        "Library Versions ({} unique, across {} games)",
        report.entries.len(),
        report
            .entries
            .iter()
            .map(|e| e.game_count)
            .max()
            .unwrap_or(0)
    );
    println!("{}", "=".repeat(80));
    println!(
        "{:<36} {:>12} {:>12}  {}",
        "Library", "Version", "Games", "Game List"
    );
    println!("{}", "-".repeat(80));

    for entry in &report.entries {
        let games_str = if entry.games.len() > 3 {
            format!(
                "{}, +{} more",
                entry.games[..3].join(", "),
                entry.games.len() - 3
            )
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
pub(crate) fn print_engines_terminal(report: &ps5_analysis::EngineHintReport) {
    println!("Engine Hints ({} games)", report.games.len());
    println!("{}", "=".repeat(70));
    println!("{:<30} {:<16} {}", "Game", "Engine", "SCE Libraries");
    println!("{}", "-".repeat(70));

    for game in &report.games {
        let engine = game.engines.join(", ");
        let sce = if game.sce_libraries.len() > 4 {
            format!(
                "{}, +{} more",
                game.sce_libraries[..4].join(", "),
                game.sce_libraries.len() - 4
            )
        } else {
            game.sce_libraries.join(", ")
        };
        println!(
            "{:<30} {:<16} {}",
            game.display_name.as_deref().unwrap_or(&game.name),
            engine,
            sce
        );
    }
}
