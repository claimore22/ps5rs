use std::path::{Path, PathBuf};

use crate::util::load_file;

pub(crate) fn cmd_extract(path: &PathBuf, output: &Option<PathBuf>, anyps5_layout: bool) {
    if anyps5_layout {
        cmd_extract_anyps5(path, output);
        return;
    }
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

    println!(
        "Format:  {}",
        if result.was_self {
            "SELF"
        } else {
            "Raw ELF (passthrough)"
        }
    );
    println!("Output:  {}", out_path.display());
    println!("Size:    {} bytes", result.elf.len());

    if result.compressed_segments > 0 {
        println!();
        println!("Notes:");
        println!(
            "  {} compressed segment(s) — inflated during extraction",
            result.compressed_segments
        );
    }
}

pub(crate) fn cmd_batch_extract(
    path: &std::path::Path,
    output: &std::path::Path,
    _extra_nids: &[PathBuf],
    include_modules: bool,
    anyps5_layout: bool,
) {
    if anyps5_layout {
        cmd_batch_extract_anyps5(path, output);
        return;
    }
    let options = ps5_analysis::BatchExtractOptions { include_modules };

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

fn cmd_extract_anyps5(path: &Path, output: &Option<PathBuf>) {
    let game_root = if path.is_dir() {
        path.to_path_buf()
    } else if path.is_file()
        && path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("eboot.bin"))
        && let Some(parent) = path.parent()
    {
        parent.to_path_buf()
    } else {
        eprintln!(
            "error: --anyps5-layout needs a game directory or an eboot.bin file, got {}",
            path.display()
        );
        std::process::exit(1);
    };
    let out_dir = match output {
        Some(o) => o.clone(),
        None => match game_root.parent().and_then(|p| {
            game_root
                .file_name()
                .map(|n| p.join(format!("{}-anyps5", n.to_string_lossy())))
        }) {
            Some(o) => o,
            None => {
                eprintln!("error: cannot derive an output directory; pass -o explicitly");
                std::process::exit(1);
            }
        },
    };
    print_anyps5_report(&game_root, &out_dir);
}

fn print_anyps5_report(game_root: &Path, out_dir: &Path) {
    eprintln!(
        "Building AnyPS5 layout for {} -> {}...",
        game_root.display(),
        out_dir.display()
    );
    let report = match ps5_analysis::layout_game(game_root, out_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    print_game_layout(&report.manifest, out_dir);
    if !report.manifest.complete {
        std::process::exit(2);
    }
}

fn print_game_layout(manifest: &ps5_analysis::GameLayoutManifest, out_dir: &Path) {
    println!("Game: {}", manifest.game);
    println!(
        "  Converted: {} file(s) to source/",
        manifest.converted.len()
    );
    println!("  Copied: {} file(s) to app0/", manifest.copied_files);
    if !manifest.module_dirs.is_empty() {
        println!(
            "  Extra module dirs (relinker --module-dir): {}",
            manifest.module_dirs.join(", ")
        );
    }
    if let Some(eboot) = &manifest.eboot {
        let mut cmd = format!(
            "relinker {}/source/{eboot} -o {}/app.elf",
            out_dir.display(),
            out_dir.display()
        );
        for dir in &manifest.module_dirs {
            cmd.push_str(&format!(" --module-dir {dir}"));
        }
        println!("  Suggested: {cmd}");
        println!("  Then merge the generated app0/*.guest.prx files into app0/");
    }
    if !manifest.failures.is_empty() {
        println!("  Failures:");
        for failure in &manifest.failures {
            println!(
                "    [{}] {}: {}",
                failure.phase, failure.relative_path, failure.reason
            );
        }
    }
    println!(
        "  Manifest: {}",
        out_dir.join("anyps5-manifest.json").display()
    );
}

fn cmd_batch_extract_anyps5(path: &std::path::Path, output: &std::path::Path) {
    eprintln!("Batch building AnyPS5 layouts from {}...", path.display());
    let report = match ps5_analysis::layout_games(path, output) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    for game in &report.games {
        eprintln!();
        eprintln!("Game: {}", game.name);
        print_game_layout(&game.manifest, &game.out_dir);
    }
    eprintln!();
    eprintln!(
        "Laid out {}/{} games to {}",
        report.succeeded,
        report.total,
        output.display()
    );
    if !report.failures.is_empty() {
        eprintln!();
        eprintln!("Failed:");
        for (game, reason) in &report.failures {
            eprintln!("  {game}: {reason}");
        }
    }
    if report.failed > 0 {
        std::process::exit(2);
    }
}
