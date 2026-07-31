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
