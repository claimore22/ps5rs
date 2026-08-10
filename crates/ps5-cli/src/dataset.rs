use std::path::PathBuf;

use crate::catalog::load_catalog;
use crate::util::{is_dataset_dir, write_to_output_or_stdout};

pub(crate) fn cmd_validate(path: &std::path::Path, output: &Option<PathBuf>) {
    if !is_dataset_dir(path) {
        eprintln!(
            "error: {} is not a dataset directory (run 'ps5rs scan' first)",
            path.display()
        );
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
    eprintln!(
        "  Valid ELF:    {}/{}",
        report.elf_valid, report.total_games
    );
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

pub(crate) fn cmd_dashboard(
    path: &std::path::Path,
    output: &PathBuf,
    games: Option<&std::path::Path>,
) {
    let ds = ps5_analysis::AnalysisDataset::open(path).unwrap_or_else(|e| {
        eprintln!("error: failed to load dataset from {}: {e}", path.display());
        std::process::exit(1);
    });

    eprintln!("Computing dashboard data from {} games...", ds.images.len());
    let mut data = ps5_dashboard::data::compute(&ds);

    let loader_dir = path.join("load");
    if loader_dir.is_dir() {
        eprintln!("Loading loader data from {}...", loader_dir.display());
        data.inject_loader_data(&loader_dir);
    }

    if let Some(games_root) = games {
        eprintln!("Scanning {} for middleware...", games_root.display());
        let catalog = load_catalog(&[]);
        let report = ps5_analysis::build_middleware_report(games_root, &catalog);
        data.inject_middleware(&report);
        eprintln!(
            "  Middleware: {} games, {} modules ({} third-party, {} Sony, {} unknown)",
            report.games.len(),
            report.total_prx,
            report.third_party_modules,
            report.sony_modules,
            report.unknown_modules
        );
    }

    let html = ps5_dashboard::html::generate_html(&data);

    let out_file = if output.to_string_lossy().ends_with(".html") {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("error: cannot create {}: {e}", parent.display());
                std::process::exit(1);
            });
        }
        output.clone()
    } else {
        std::fs::create_dir_all(output).unwrap_or_else(|e| {
            eprintln!("error: cannot create {}: {e}", output.display());
            std::process::exit(1);
        });
        output.join("index.html")
    };

    std::fs::write(&out_file, &html).unwrap_or_else(|e| {
        eprintln!("error: cannot write {}: {e}", out_file.display());
        std::process::exit(1);
    });

    eprintln!("Dashboard written to {}", out_file.display());
    eprintln!("  Games: {}", data.overview.total_games);
    eprintln!("  Libraries: {}", data.overview.unique_libs);
    eprintln!(
        "  NIDs: {} ({:.1}% resolved)",
        data.overview.unique_nids, data.overview.resolution_rate
    );
}

pub(crate) fn cmd_export_unknown(path: &std::path::Path, group_by: &str, output: &Option<PathBuf>) {
    if !is_dataset_dir(path) {
        eprintln!("error: export-unknown requires a dataset directory (run 'ps5rs scan' first)");
        std::process::exit(1);
    }

    let ds = ps5_analysis::AnalysisDataset::open(path).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let mut report = ps5_analysis::reports::build_unknown_nids(&ds);

    if group_by == "library" {
        report.entries.sort_by(|a, b| {
            a.libraries
                .first()
                .cmp(&b.libraries.first())
                .then(b.count.cmp(&a.count))
        });
    }

    eprintln!(
        "Exporting {} unknown NIDs ({} total unresolved, {:.1}% of {} imports)",
        report.entries.len(),
        report.total_unknown,
        if report.total_imports > 0 {
            report.total_unknown as f64 / report.total_imports as f64 * 100.0
        } else {
            0.0
        },
        report.total_imports,
    );

    write_to_output_or_stdout(output, &|w| {
        ps5_analysis::export::csv::export_unknown_nids(&report, w)
    });
}
