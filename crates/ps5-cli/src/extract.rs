use std::path::PathBuf;

use crate::util::load_file;

pub(crate) fn cmd_extract(path: &PathBuf, output: &Option<PathBuf>) {
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

pub(crate) fn cmd_batch_extract(path: &std::path::Path, output: &std::path::Path, _extra_nids: &[PathBuf], include_modules: bool) {
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
