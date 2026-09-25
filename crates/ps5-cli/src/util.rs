use std::io::Write;
use std::path::PathBuf;

pub(crate) fn load_file(path: &PathBuf) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        std::process::exit(1);
    })
}

pub(crate) fn write_to_output_or_stdout(
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

/// Trim a trailing Windows (`\`) or Unix (`/`) separator from a path string.
/// Returns a `PathBuf` without the final separator, unless the path is a single
/// root component (e.g. `C:\` or `/`). This function is used as a custom Clap
/// value parser for directory arguments such as the `scan` command.
pub(crate) fn sanitize_path_arg(s: &str) -> Result<std::path::PathBuf, std::convert::Infallible> {
    // Remove a single trailing slash or backslash if present and the path has more
    // than one component. This avoids treating a trailing separator as a separate
    // argument when the user writes something like "C:\\Games\\".
    let trimmed = if s.len() > 1 && (s.ends_with('/') || s.ends_with('\\')) {
        &s[..s.len() - 1]
    } else {
        s
    };
    Ok(std::path::PathBuf::from(trimmed))
}

pub(crate) fn is_dataset_dir(path: &std::path::Path) -> bool {
    path.join("manifest.json").exists() && path.join("images").is_dir()
}

pub(crate) fn osabi_name(osabi: u8) -> &'static str {
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

pub(crate) fn e_version_name(v: u32) -> &'static str {
    match v {
        1 => "Current",
        _ => "Unknown",
    }
}

pub(crate) fn dataset_to_database_real(
    ds: &ps5_analysis::AnalysisDataset,
) -> ps5_analysis::AnalysisDatabase {
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
                    resolved_name: imp.resolved_name.clone().unwrap_or_else(|| "?".into()),
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
                display_name: Some(ds.display_name_for(name).to_string()),
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

pub(crate) fn load_dataset_or_collect(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_trailing_backslash() {
        let input = "C:\\Games\\PS5\\";
        let expected = std::path::PathBuf::from("C:\\Games\\PS5");
        assert_eq!(sanitize_path_arg(input).unwrap(), expected);
    }

    #[test]
    fn sanitize_keeps_path_without_trailing() {
        let input = "C:\\Games\\PS5";
        let expected = std::path::PathBuf::from("C:\\Games\\PS5");
        assert_eq!(sanitize_path_arg(input).unwrap(), expected);
    }
}
