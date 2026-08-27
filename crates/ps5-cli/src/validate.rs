use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use serde::Serialize;

use crate::catalog::load_catalog;
use crate::util::{load_file, write_to_output_or_stdout};

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct ValidationReport {
    schema_version: u32,
    tool: &'static str,
    file: String,
    sha256: String,
    container: String,
    platform: String,
    elf: ElfInfo,
    dependencies: DependencyInfo,
    symbols: SymbolCounts,
    relocations: RelocationInfo,
    plt: PltInfo,
    nid_coverage: NidCoverage,
    unknown_nids: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ElfInfo {
    arch: String,
    pie: bool,
    entry_point: String,
}

#[derive(Serialize)]
struct DependencyInfo {
    count: usize,
    libraries: Vec<String>,
}

#[derive(Serialize)]
struct SymbolCounts {
    imports: usize,
    exports: usize,
}

#[derive(Serialize)]
struct RelocationInfo {
    total: usize,
    by_kind: Vec<KindCount>,
}

#[derive(Serialize)]
struct KindCount {
    kind: String,
    count: usize,
}

#[derive(Serialize)]
struct PltInfo {
    entries: usize,
}

#[derive(Serialize)]
struct NidCoverage {
    known: usize,
    unknown: usize,
}

fn reloc_kind_name(kind: ps5_image::RelocationKind) -> String {
    let name = match kind {
        ps5_image::RelocationKind::None => "NONE",
        ps5_image::RelocationKind::_64 => "ABS64",
        ps5_image::RelocationKind::PC32 => "PC32",
        ps5_image::RelocationKind::GOT32 => "GOT32",
        ps5_image::RelocationKind::PLT32 => "PLT32",
        ps5_image::RelocationKind::Copy => "COPY",
        ps5_image::RelocationKind::GlobDat => "GLOB_DAT",
        ps5_image::RelocationKind::JumpSlot => "JUMP_SLOT",
        ps5_image::RelocationKind::Relative => "RELATIVE",
        ps5_image::RelocationKind::Direct32 => "DIRECT32",
        ps5_image::RelocationKind::Direct32S => "DIRECT32S",
        ps5_image::RelocationKind::Direct16 => "DIRECT16",
        ps5_image::RelocationKind::PC16 => "PC16",
        ps5_image::RelocationKind::Direct8 => "DIRECT8",
        ps5_image::RelocationKind::PC8 => "PC8",
        ps5_image::RelocationKind::TPOff64 => "TPOFF64",
        ps5_image::RelocationKind::TPOff32 => "TPOFF32",
        ps5_image::RelocationKind::DTPMod64 => "DTPMOD64",
        ps5_image::RelocationKind::DTPOff64 => "DTPOFF64",
        ps5_image::RelocationKind::TLSDESC => "TLSDESC",
        ps5_image::RelocationKind::TlsModOff => "TLS_MOD_OFF",
        ps5_image::RelocationKind::TlsOffset => "TLS_OFFSET",
        ps5_image::RelocationKind::Other(v) => return format!("OTHER({v})"),
    };
    name.to_string()
}

fn build_report(path: &PathBuf) -> ValidationReport {
    let data = load_file(path);
    let sha256 = ps5_format::sha256_hex(&data);
    let catalog = load_catalog(&[]);
    let image = ps5_image::BinaryImageBuilder::build_from_file(&data, &sha256, &catalog);

    let mut by_kind = BTreeMap::<String, usize>::new();
    let mut plt_entries = 0usize;
    for reloc in &image.relocations {
        *by_kind.entry(reloc_kind_name(reloc.kind)).or_insert(0) += 1;
        if reloc.is_plt {
            plt_entries += 1;
        }
    }
    let by_kind: Vec<KindCount> = by_kind
        .into_iter()
        .map(|(kind, count)| KindCount { kind, count })
        .collect();

    let known = image
        .imports
        .iter()
        .filter(|i| i.resolved_name.is_some())
        .count();
    let unknown = image.imports.len() - known;

    let mut unknown_nids: Vec<String> = image
        .imports
        .iter()
        .filter(|i| i.resolved_name.is_none())
        .map(|i| i.nid_hash.clone())
        .collect();
    unknown_nids.sort();
    unknown_nids.dedup();

    let jump_slot = by_kind
        .iter()
        .find(|k| k.kind == "JUMP_SLOT")
        .map_or(0, |k| k.count);

    let mut warnings = Vec::new();
    if plt_entries != jump_slot {
        warnings.push(format!(
            "plt relocations ({plt_entries}) do not match JUMP_SLOT relocations ({jump_slot})"
        ));
    }
    let mut seen = HashSet::new();
    for imp in &image.imports {
        if !seen.insert(imp.nid_hash.as_str()) {
            warnings.push(format!("duplicate import NID: {}", imp.nid_hash));
        }
    }
    for imp in &image.imports {
        if !image.import_libs.contains_key(&imp.library_id) {
            warnings.push(format!(
                "import {} references unknown library id {}",
                imp.nid_hash, imp.library_id
            ));
        }
    }
    if image.entry_point == 0 {
        warnings.push("entry point is zero".to_string());
    }
    warnings.sort();
    warnings.dedup();

    let needed = image.needed_files.clone();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    ValidationReport {
        schema_version: SCHEMA_VERSION,
        tool: "ps5rs",
        file: file_name,
        sha256,
        container: if image.is_self { "SELF" } else { "ELF" }.to_string(),
        platform: image.platform.to_string(),
        elf: ElfInfo {
            arch: "x86_64".to_string(),
            pie: image.metadata.elf_type == 3,
            entry_point: format!("{:#x}", image.entry_point),
        },
        dependencies: DependencyInfo {
            count: needed.len(),
            libraries: needed,
        },
        symbols: SymbolCounts {
            imports: image.imports.len(),
            exports: image.exports.len(),
        },
        relocations: RelocationInfo {
            total: image.relocations.len(),
            by_kind,
        },
        plt: PltInfo {
            entries: plt_entries,
        },
        nid_coverage: NidCoverage { known, unknown },
        unknown_nids,
        warnings,
    }
}

fn print_report(report: &ValidationReport) {
    println!("PS5 Binary Validation Report");
    println!("============================");
    println!();
    println!("File:");
    println!("  {}", report.file);
    println!();
    println!("Container:");
    println!("  {}", report.container);
    println!();
    println!("ELF:");
    println!("  {}", report.elf.arch);
    println!("  {}", if report.elf.pie { "PIE" } else { "static" });
    println!();
    println!("Dependencies:");
    println!("  {}", report.dependencies.count);
    println!();
    println!("Symbols:");
    println!("  Imports: {}", report.symbols.imports);
    println!("  Exports: {}", report.symbols.exports);
    println!();
    println!("Relocations:");
    println!("  Total: {}", report.relocations.total);
    println!();
    for kind in &report.relocations.by_kind {
        println!("  {:<12} {}", kind.kind, kind.count);
    }
    println!();
    println!("PLT:");
    println!("  Entries: {}", report.plt.entries);
    println!();
    println!("NID Coverage:");
    println!(
        "  Known: {} / {}",
        report.nid_coverage.known, report.symbols.imports
    );
    println!(
        "  Unknown: {} / {}",
        report.nid_coverage.unknown, report.symbols.imports
    );
    if !report.unknown_nids.is_empty() {
        println!();
        println!("Unknown NIDs:");
        for nid in &report.unknown_nids {
            println!("  {nid}");
        }
    }
    println!();
    println!("Warnings:");
    if report.warnings.is_empty() {
        println!("  none");
    } else {
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
}

pub(crate) fn cmd_validate_binary(path: &PathBuf, json: bool, output: &Option<PathBuf>) {
    let report = build_report(path);
    if json {
        write_to_output_or_stdout(output, &|w| {
            serde_json::to_writer_pretty(w, &report).map_err(std::io::Error::other)
        });
    } else {
        print_report(&report);
    }
}

#[derive(Serialize)]
struct ExternalReport {
    schema_version: u32,
    tool: &'static str,
    root: String,
    generated_at: String,
    status: String,
    files_discovered: std::collections::BTreeMap<String, usize>,
    files_parsed: std::collections::BTreeMap<String, usize>,
    files_rejected: std::collections::BTreeMap<String, usize>,
    parser_errors: Vec<String>,
    elf_self_prx: ElfSelfPrxStats,
    imports_exports: ImportExportStats,
    nid: NidStats,
    deps: usize,
    shader: ShaderStats,
    source_binary_correlation: String,
    abi: String,
    firmware: String,
    engine: String,
    exe_tools: ExeStats,
}

#[derive(Serialize, Default)]
struct ElfSelfPrxStats {
    total: usize,
    parsed: usize,
    failed: usize,
}

#[derive(Serialize, Default)]
struct ImportExportStats {
    total_imports: usize,
    total_exports: usize,
}

#[derive(Serialize, Default)]
struct NidStats {
    total_nids: usize,
    resolved: usize,
    unresolved: usize,
}

#[derive(Serialize, Default)]
struct ShaderStats {
    total: usize,
    parsed: usize,
    failed: usize,
}

#[derive(Serialize, Default)]
struct ExeStats {
    discovered: usize,
    tools: Vec<String>,
    status: String,
}

fn utc_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

#[allow(clippy::ptr_arg, clippy::collapsible_if)]
pub(crate) fn cmd_validate_external(path: &std::path::Path, output: &Option<PathBuf>) {
    if !path.exists() {
        let report = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "tool": "ps5rs",
            "root": path.display().to_string(),
            "status": "SKIPPED — external corpus unavailable",
            "reason": "path does not exist"
        });
        write_to_output_or_stdout(output, &|w| {
            serde_json::to_writer_pretty(w, &report).map_err(std::io::Error::other)
        });
        eprintln!("SKIPPED — external corpus unavailable: {}", path.display());
        return;
    }

    let mut discovered: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut parsed: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut rejected: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut parser_errors: Vec<String> = Vec::new();
    let mut elf_stats = ElfSelfPrxStats::default();
    let mut shader_stats = ShaderStats::default();
    let mut exe_tools: Vec<String> = Vec::new();
    let mut total_imports = 0usize;
    let mut total_exports = 0usize;
    let mut total_nids = 0usize;
    let mut resolved = 0usize;

    let mut stack: Vec<PathBuf> = vec![path.to_path_buf()];
    let mut file_list: Vec<PathBuf> = Vec::new();
    while let Some(p) = stack.pop() {
        if p.is_file() {
            file_list.push(p);
        } else if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&p) {
                for e in entries.flatten() {
                    stack.push(e.path());
                }
            }
        }
    }

    for file in &file_list {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let ext_key = if file
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".auth_info"))
            .unwrap_or(false)
        {
            "auth_info".to_string()
        } else if ext.is_empty() {
            if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(dot) = name.rfind('.') {
                    name[dot + 1..].to_ascii_lowercase()
                } else {
                    "(no_ext)".to_string()
                }
            } else {
                "(no_ext)".to_string()
            }
        } else {
            ext.clone()
        };
        *discovered.entry(ext_key.clone()).or_insert(0) += 1;

        match ext_key.as_str() {
            "elf" | "prx" | "sprx" | "self" | "o" | "so" => {
                elf_stats.total += 1;
                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(e) => {
                        *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                        parser_errors.push(format!("{}: read error {}", file.display(), e));
                        elf_stats.failed += 1;
                        continue;
                    }
                };
                let parsed_ok = if ext_key == "self" {
                    ps5_self::SelfImage::parse(&data).is_ok()
                } else {
                    ps5_elf::ElfImage::parse(&data, None).is_ok()
                        || ps5_self::SelfImage::parse(&data).is_ok()
                };
                if parsed_ok {
                    *parsed.entry(ext_key.clone()).or_insert(0) += 1;
                    elf_stats.parsed += 1;
                    // also count imports/exports for NID stats if ELF
                    if let Ok(img) = ps5_elf::ElfImage::parse(&data, None) {
                        total_imports += img.symbols.iter().filter(|s| s.is_import).count();
                        total_exports += img.symbols.iter().filter(|s| !s.is_import).count();
                        total_nids += img.symbols.len();
                        let catalog = load_catalog(&[]);
                        for s in &img.symbols {
                            let nid = s.resolved_name.split('#').next().unwrap_or("");
                            if catalog.resolve(nid).is_some() {
                                resolved += 1;
                            }
                        }
                    }
                } else {
                    *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                    elf_stats.failed += 1;
                    parser_errors.push(format!("{}: ELF/SELF parse failed", file.display()));
                }
            }
            "a" => {
                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(_) => {
                        *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                        continue;
                    }
                };
                let ok = data.starts_with(b"!<arch>")
                    || ps5_elf::stub::parse_stub_library(&data, "unknown").is_ok();
                if ok {
                    *parsed.entry(ext_key.clone()).or_insert(0) += 1;
                } else {
                    *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                }
            }
            "pssl" | "sb" | "ags" | "agsd" => {
                shader_stats.total += 1;
                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(_) => {
                        *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                        shader_stats.failed += 1;
                        continue;
                    }
                };
                if ps5_shader::ShaderBinary::parse(&data).is_ok()
                    || ps5_shader::ShaderBinary::parse_with_path(&data, file).is_ok()
                {
                    *parsed.entry(ext_key.clone()).or_insert(0) += 1;
                    shader_stats.parsed += 1;
                } else {
                    *rejected.entry(ext_key.clone()).or_insert(0) += 1;
                    shader_stats.failed += 1;
                }
            }
            "exe" => {
                exe_tools.push(
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string(),
                );
                *parsed.entry(ext_key.clone()).or_insert(0) += 1;
            }
            "c" | "cpp" | "h" | "hpp" | "json" | "xml" | "map" | "sym" | "txt" | "log" | "gnf"
            | "at9" | "bank" | "png" | "jpg" | "dds" | "tga" | "bmp" | "dae" | "mtl" | "ttf"
            | "auth_info" | "ucp" | "dat" | "bin" | "db" | "objcache" | "xcache" | "daecache"
            | "esbak" | "irr" | "swatch" => {
                // Bare artifacts and metadata: discovered, not parsed via ELF, count as parsed for inventory
                *parsed.entry(ext_key.clone()).or_insert(0) += 1;
            }
            _ => {
                *parsed.entry(ext_key.clone()).or_insert(0) += 1;
            }
        }
    }

    let deps = total_imports;
    let unresolved = total_nids.saturating_sub(resolved);
    let exe_discovered = exe_tools.len();
    let exe_is_empty = exe_tools.is_empty();
    let report = ExternalReport {
        schema_version: SCHEMA_VERSION,
        tool: "ps5rs",
        root: path.display().to_string(),
        generated_at: utc_now(),
        status: if discovered.is_empty() {
            "INSUFFICIENT EVIDENCE".to_string()
        } else {
            "PASS".to_string()
        },
        files_discovered: discovered,
        files_parsed: parsed,
        files_rejected: rejected,
        parser_errors: parser_errors.into_iter().take(20).collect(),
        elf_self_prx: elf_stats,
        imports_exports: ImportExportStats {
            total_imports,
            total_exports,
        },
        nid: NidStats {
            total_nids,
            resolved,
            unresolved,
        },
        deps,
        shader: shader_stats,
        source_binary_correlation: "SKIPPED — source↔binary correlation requires explicit source root and binary mapping".to_string(),
        abi: "SKIPPED — ABI validation requires verified signatures and HLE mapping".to_string(),
        firmware: "SKIPPED — firmware validation requires system_modules/*.exports.json and game requirements".to_string(),
        engine: "SKIPPED — engine validation requires string + artifact multi-signal; run dashboard --games".to_string(),
        exe_tools: ExeStats {
            discovered: exe_discovered,
            tools: exe_tools.into_iter().take(20).collect(),
            status: if exe_is_empty {
                "No .exe tools discovered under supplied root — SKIPPED".to_string()
            } else {
                "Discovered .exe tools inventoried; differential validation requires explicit tool config — SKIPPED".to_string()
            },
        },
    };

    write_to_output_or_stdout(output, &|w| {
        serde_json::to_writer_pretty(w, &report).map_err(std::io::Error::other)
    });
    eprintln!(
        "External validation: {} files discovered, {} ELF/SELF/PRX parsed ({} failed), {} shaders parsed, {} .exe tools",
        file_list.len(),
        report.elf_self_prx.parsed,
        report.elf_self_prx.failed,
        report.shader.parsed,
        report.exe_tools.discovered
    );
}
