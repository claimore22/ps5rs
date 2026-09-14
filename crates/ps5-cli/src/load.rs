use std::path::{Path, PathBuf};

use ps5_farm::report::{self, EdgeInfo, GraphInfo, LoadReport, ModuleInfo, Totals};
use ps5_loader::LibraryImportCounts;
use ps5_loader::OfflineExportTable;

use crate::util::load_file;

fn format_size(bytes: usize) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    format!("{:.2} MB", mb)
}

fn container_name(data: &[u8]) -> &'static str {
    if data.len() < 4 {
        return "Too small";
    }
    if &data[0..4] == b"\x7fELF" {
        return "Raw ELF";
    }
    match u32::from_be_bytes([data[0], data[1], data[2], data[3]]) {
        0x5414F5EE => "PS5 SELF",
        0x4F153D1D => "PS4 SELF",
        _ => "Unknown",
    }
}

fn machine_name(machine: u16) -> &'static str {
    match machine {
        62 => "x86-64",
        3 => "i386",
        40 => "ARM",
        0xB7 => "AArch64",
        _ => "Unknown",
    }
}

fn elf_type_name(e_type: u16) -> &'static str {
    match e_type {
        0x0002 => "ET_EXEC",
        0x0003 => "ET_DYN",
        0xFE10 => "ET_SCE_DYNEXEC",
        0xFE18 => "ET_SCE_DYNAMIC",
        _ => "Unknown",
    }
}

pub(crate) fn get_elf_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() >= 4 && &data[0..4] == b"\x7fELF" {
        return data.to_vec();
    }
    let result = ps5_self::extract::extract_elf(data).unwrap_or_else(|e| {
        eprintln!("error: SELF extraction failed: {e}");
        std::process::exit(1);
    });
    result.elf
}

/// Walk `--prx-dir` and return a list of (filename, full_path) pairs.
fn scan_prx_dir(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(dir).unwrap_or_else(|e| {
        eprintln!("error: cannot read PRX directory {}: {e}", dir.display());
        std::process::exit(1);
    });
    for entry in read_dir {
        let entry = entry.unwrap_or_else(|e| {
            eprintln!("error: reading directory entry: {e}");
            std::process::exit(1);
        });
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().map(|s| s.to_string_lossy().to_string())
        {
            entries.push((name, path));
        }
    }
    entries
}

/// Try to find a file matching a `DT_NEEDED` name.
///
/// Search order:
/// 1. Exact filename match
/// 2. Append `.self` (encrypted PRX on disk)
/// 3. Case-insensitive fallback
fn find_prx<'a>(name: &'a str, files: &'a [(String, PathBuf)]) -> Option<&'a PathBuf> {
    if let Some((_, path)) = files.iter().find(|(f, _)| f == name) {
        return Some(path);
    }
    let with_self = format!("{name}.self");
    if let Some((_, path)) = files.iter().find(|(f, _)| f == &with_self) {
        return Some(path);
    }
    let lower = name.to_lowercase();
    files
        .iter()
        .find(|(f, _)| f.to_lowercase() == lower)
        .map(|(_, path)| path)
}

/// Display the module dependency graph as ASCII.
fn print_graph(ctx: &ps5_loader::ModuleContext) {
    println!("Module Dependency Graph");
    println!("----------------------");

    let mut edge_count = 0;
    for node in ctx.graph.all_modules() {
        let deps = ctx.graph.dependencies(node);
        if deps.is_empty() {
            if ctx.graph.is_unavailable(node) {
                println!("  {node} [MISSING]");
            } else {
                println!("  {node}");
            }
        } else {
            for dep in deps {
                let tag = if ctx.graph.is_unavailable(dep) {
                    " [MISSING]"
                } else {
                    " [loaded]"
                };
                println!("  {node} → {dep}{tag}");
                edge_count += 1;
            }
        }
    }
    if edge_count == 0 && ctx.graph.node_count() == 0 {
        println!("  (no dependencies)");
    }
    println!();
}

/// Display per-module details.
fn print_modules(ctx: &ps5_loader::ModuleContext) {
    println!("Modules");
    println!("-------");

    for (i, module) in ctx.modules.iter().enumerate() {
        let type_label = match module.module_type {
            ps5_loader::ModuleType::Eboot => "EBOOT",
            ps5_loader::ModuleType::Prx => "PRX",
        };
        println!(
            "  [{i}] {} ({type_label}) @ {:#018x}",
            module.name, module.load_bias
        );
        println!("      Exports: {}", module.exports_count);
        println!(
            "      Imports: {} resolved, {} known, {} stubbed",
            module.imports_resolved, module.imports_known, module.imports_stubbed,
        );
        if let Some(rs) = module.relocation_summary.as_ref() {
            let mut parts = Vec::new();
            if rs.relative > 0 {
                parts.push(format!("RELATIVE {}", rs.relative));
            }
            if rs.glob_dat > 0 {
                parts.push(format!("GLOB_DAT {}", rs.glob_dat));
            }
            if rs.jump_slot > 0 {
                parts.push(format!("JUMP_SLOT {}", rs.jump_slot));
            }
            if rs.abs64 > 0 {
                parts.push(format!("ABS64 {}", rs.abs64));
            }
            if !parts.is_empty() {
                println!("      Relocations: {}", parts.join(", "));
            }
        }
        if let Some(entry) = module.entry_point {
            println!("      Entry: {:#018x}", entry);
        }
        if let Some(ref soname) = module.soname
            && soname != &module.name
        {
            println!("      SONAME: {soname}");
        }
        if let Some(ref tls) = module.tls {
            println!(
                "      TLS: vaddr={:#018x} filesz={} memsz={} align={}",
                tls.vaddr, tls.filesz, tls.memsz, tls.align
            );
        }
        if module.init_va > 0 {
            println!("      INIT: {:#018x}", module.init_va);
        }
        if module.init_array_va > 0 {
            println!(
                "      INIT_ARRAY: {} entries @ {:#018x}",
                module.init_array_sz, module.init_array_va
            );
        }
        if module.fini_va > 0 {
            println!("      FINI: {:#018x}", module.fini_va);
        }
        if module.fini_array_va > 0 {
            println!(
                "      FINI_ARRAY: {} entries @ {:#018x}",
                module.fini_array_sz, module.fini_array_va
            );
        }
        if module.preinit_array_va > 0 {
            println!(
                "      PREINIT_ARRAY: {} entries @ {:#018x}",
                module.preinit_array_sz, module.preinit_array_va
            );
        }
        if !module.per_library_imports.is_empty() {
            println!("      Imports by library:");
            for lib in &module.per_library_imports {
                let parts: Vec<String> = std::iter::empty()
                    .chain(if lib.resolved > 0 {
                        Some(format!("{} resolved", lib.resolved))
                    } else {
                        None
                    })
                    .chain(if lib.known > 0 {
                        Some(format!("{} known", lib.known))
                    } else {
                        None
                    })
                    .chain(if lib.stubbed > 0 {
                        Some(format!("{} stubbed", lib.stubbed))
                    } else {
                        None
                    })
                    .collect();
                if !parts.is_empty() {
                    println!("        {} ({})", lib.library, parts.join(", "));
                }
            }
        }
        println!();
    }

    if ctx.graph.unavailable_modules().next().is_some() {
        println!("Unavailable modules:");
        for m in ctx.graph.unavailable_modules() {
            println!("  {m}");
        }
        println!();
    }
}

pub(crate) fn cmd_load(path: &PathBuf, prx_dir: Option<PathBuf>, json: bool) {
    let data = load_file(path);

    let dir = prx_dir.or_else(|| {
        let default = path.parent().map(|p| p.join("sce_module"));
        match default {
            Some(ref d) if d.is_dir() => Some(d.clone()),
            _ => None,
        }
    });

    if let Some(ref dir) = dir {
        cmd_load_multi(path, &data, dir, json);
    } else {
        cmd_load_single(path, &data, json);
    }
}

/// Multi-module mode: load eboot + PRXs from `--prx-dir`.
fn cmd_load_multi(path: &Path, data: &[u8], prx_dir: &Path, json: bool) {
    let container = container_name(data);
    let elf_bytes = get_elf_bytes(data);

    let elf_image = ps5_elf::ElfImage::parse(&elf_bytes, None).unwrap_or_else(|e| {
        eprintln!("error: ELF parse failed: {e}");
        std::process::exit(1);
    });

    println!("Container: {container}");
    println!(
        "ELF: {} {}",
        machine_name(elf_image.header.e_machine),
        elf_type_name(elf_image.header.e_type),
    );
    println!();

    let prx_files = scan_prx_dir(prx_dir);
    println!(
        "PRX directory: {} ({} files)",
        prx_dir.display(),
        prx_files.len()
    );
    println!();

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or(std::borrow::Cow::Borrowed("?"));
    let eboot_display_name = file_name.to_string();

    let offline = if Path::new("system_modules").is_dir() {
        let table = OfflineExportTable::load_from_dir(Path::new("system_modules"));
        if !table.is_empty() {
            println!("Offline exports: {} known symbols loaded", table.len());
            Some(table)
        } else {
            None
        }
    } else {
        None
    };

    let ctx = ps5_loader::load_modules(
        &eboot_display_name,
        &elf_bytes,
        |name| {
            let found = find_prx(name, &prx_files);
            match found {
                Some(p) => {
                    let contents = std::fs::read(p).unwrap_or_else(|e| {
                        eprintln!("error: cannot read PRX {}: {e}", p.display());
                        std::process::exit(1);
                    });
                    let elf_contents = get_elf_bytes(&contents);
                    Some(elf_contents)
                }
                None => None,
            }
        },
        offline.as_ref(),
    )
    .unwrap_or_else(|e| {
        eprintln!("error: load failed: {e}");
        std::process::exit(1);
    });

    if json {
        let report = report::build_report(&ctx);
        let json_str = serde_json::to_string_pretty(&report).unwrap_or_else(|e| {
            eprintln!("error: JSON serialization failed: {e}");
            std::process::exit(1);
        });
        println!("{json_str}");
        return;
    }

    print_graph(&ctx);
    print_modules(&ctx);

    println!("Summary");
    println!("-------");
    println!("  Total modules loaded: {}", ctx.modules.len());
    println!("  Total exports registered: {}", ctx.exports.len());
    println!("  Total imports resolved: {}", ctx.resolved_imports);
    if ctx.known_imports > 0 {
        println!("  Total imports known (offline): {}", ctx.known_imports);
    }
    println!("  Total imports stubbed: {}", ctx.stubbed_imports);
    if ctx.graph.unavailable_modules().next().is_some() {
        println!(
            "  Missing dependencies: {}",
            ctx.graph.unavailable_modules().count()
        );
    }
}

/// Single-file mode (legacy): load one binary with no PRX resolution.
fn cmd_load_single(path: &Path, data: &[u8], json: bool) {
    let container = container_name(data);
    let elf_bytes = get_elf_bytes(data);

    let elf_image = ps5_elf::ElfImage::parse(&elf_bytes, None).unwrap_or_else(|e| {
        eprintln!("error: ELF parse failed: {e}");
        std::process::exit(1);
    });

    let mut module = ps5_loader::load_elf(
        path.file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or(std::borrow::Cow::Borrowed("unknown"))
            .as_ref(),
        &elf_bytes,
    )
    .unwrap_or_else(|e| {
        eprintln!("error: loader failed: {e}");
        std::process::exit(1);
    });

    let mut stubber = ps5_loader::StubAllocator::new(ps5_loader::STUB_REGION_BASE);

    let offline = if Path::new("system_modules").is_dir() {
        let table = OfflineExportTable::load_from_dir(Path::new("system_modules"));
        if !table.is_empty() { Some(table) } else { None }
    } else {
        None
    };

    let empty_exports = ps5_loader::ExportTable::new();
    let summary = if let Some(ref offline_table) = offline {
        let mut resolver =
            ps5_loader::CrossModuleResolver::new(&empty_exports, Some(offline_table), &mut stubber);
        let s = ps5_loader::apply_relocations_with(&mut module, &elf_image, Some(&mut resolver))
            .unwrap_or_else(|e| {
                eprintln!("error: relocation failed: {e}");
                std::process::exit(1);
            });
        module.per_library_imports = resolver.per_library_imports();
        s
    } else {
        ps5_loader::apply_relocations_with(&mut module, &elf_image, Some(&mut stubber))
            .unwrap_or_else(|e| {
                eprintln!("error: relocation failed: {e}");
                std::process::exit(1);
            })
    };

    if json {
        let info = ModuleInfo {
            name: module.name.clone(),
            module_type: match module.module_type {
                ps5_loader::ModuleType::Eboot => "Eboot".to_string(),
                ps5_loader::ModuleType::Prx => "Prx".to_string(),
            },
            load_bias: module.load_bias,
            entry_point: module.entry_point,
            exports_count: module.exports_count,
            imports_resolved: summary.resolved_imports,
            imports_known: summary.known_imports,
            imports_stubbed: summary.stubbed_imports,
            relative: summary.relative,
            glob_dat: summary.glob_dat,
            jump_slot: summary.jump_slot,
            abs64: summary.abs64,
            copy: summary.copy,
            tls_relocations: summary.tls,
            ifunc: summary.ifunc,
            unknown: summary.unknown,
            state: "Linked".to_string(),
            has_tls: module.tls.is_some(),
            init_va: module.init_va,
            init_array_va: module.init_array_va,
            init_array_sz: module.init_array_sz,
            fini_va: module.fini_va,
            fini_array_va: module.fini_array_va,
            fini_array_sz: module.fini_array_sz,
            preinit_array_va: module.preinit_array_va,
            preinit_array_sz: module.preinit_array_sz,
            per_library: module.per_library_imports.clone(),
        };
        let report = LoadReport {
            modules: vec![info],
            graph: GraphInfo {
                nodes: vec![module.name.clone()],
                unavailable: vec![],
                edges: vec![],
            },
            totals: Totals {
                modules: 1,
                resolved: summary.resolved_imports,
                known: summary.known_imports,
                stubbed: summary.stubbed_imports,
                exports: 0,
                unavailable: 0,
            },
        };
        let json_str = serde_json::to_string_pretty(&report).unwrap_or_else(|e| {
            eprintln!("error: JSON serialization failed: {e}");
            std::process::exit(1);
        });
        println!("{json_str}");
        return;
    }

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or(std::borrow::Cow::Borrowed("?"));

    println!("Module: {file_name}");
    let type_label = if module.module_type == ps5_loader::ModuleType::Eboot {
        "EBOOT"
    } else {
        "PRX"
    };
    println!("Type: {type_label}");
    println!();

    println!("Container: {container}");
    println!(
        "ELF: {} {}",
        machine_name(elf_image.header.e_machine),
        elf_type_name(elf_image.header.e_type),
    );
    println!();

    println!("Memory");
    println!("-------");
    for seg in &module.memory.regions {
        println!(
            "{:<5} {:<8} @ {:#018x}",
            seg.permissions,
            format_size(seg.size),
            seg.vaddr,
        );
    }
    println!();

    if let Some(entry) = module.entry_point {
        println!("Entry point");
        println!("-----------");
        println!("{:#018x}", entry);
        println!();
    }

    let has_any = summary.relative > 0
        || summary.glob_dat > 0
        || summary.jump_slot > 0
        || summary.abs64 > 0
        || summary.copy > 0
        || summary.tls > 0
        || summary.ifunc > 0
        || summary.unknown > 0;

    println!("Relocations");
    println!("-----------");
    if has_any {
        let mut rows: Vec<(&str, u32, String)> = Vec::new();
        if summary.relative > 0 {
            let label = if summary.relative_fast_path > 0 {
                format!("applied ({} fast path)", summary.relative_fast_path)
            } else {
                "applied".to_string()
            };
            rows.push(("RELATIVE", summary.relative, label));
        }
        let resolved_label = if summary.stubbed_imports > 0 {
            "resolved to stub"
        } else if summary.known_imports > 0 {
            "known system (stub)"
        } else {
            "pending"
        };
        if summary.glob_dat > 0 {
            rows.push(("GLOB_DAT", summary.glob_dat, resolved_label.to_string()));
        }
        if summary.jump_slot > 0 {
            rows.push(("JUMP_SLOT", summary.jump_slot, resolved_label.to_string()));
        }
        if summary.abs64 > 0 {
            rows.push(("ABS64", summary.abs64, "applied".to_string()));
        }
        if summary.copy > 0 {
            rows.push(("COPY", summary.copy, "pending".to_string()));
        }
        if summary.tls > 0 {
            rows.push(("TLS", summary.tls, "pending".to_string()));
        }
        if summary.ifunc > 0 {
            rows.push(("IFUNC", summary.ifunc, "pending".to_string()));
        }
        if summary.unknown > 0 {
            rows.push(("Unknown", summary.unknown, "pending".to_string()));
        }

        let max_name = rows.iter().map(|r| r.0.len()).max().unwrap_or(8);
        let max_count = rows
            .iter()
            .map(|r| r.1.to_string().len())
            .max()
            .unwrap_or(4);

        for (name, count, label) in &rows {
            println!(
                "  {:<width$} {:>count_width$} {}",
                name,
                count,
                label,
                width = max_name,
                count_width = max_count,
            );
        }
    } else {
        println!("  No relocations");
    }
    if summary.known_imports > 0 {
        println!();
        println!(
            "Known imports: {} (matched system functions, not loaded)",
            summary.known_imports
        );
    }
    if summary.stubbed_imports > 0 {
        println!("Unknown imports: {} (stub region)", summary.stubbed_imports);
        println!("Stub region: {:#018x}", ps5_loader::STUB_REGION_BASE);
    }
    println!();

    println!("Status");
    println!("------");
    println!("✓ Memory mapped");
    if summary.relative > 0 {
        if summary.relative_fast_path > 0 {
            println!(
                "✓ RELATIVE relocations applied ({} via DT_RELACOUNT)",
                summary.relative_fast_path
            );
        } else {
            println!("✓ RELATIVE relocations applied");
        }
    } else {
        println!("✓ No RELATIVE relocations");
    }
    if summary.resolved_imports > 0 {
        println!(
            "✓ {} imports resolved (loaded modules)",
            summary.resolved_imports
        );
    }
    if summary.known_imports > 0 {
        println!(
            "✓ {} imports known (offline system functions)",
            summary.known_imports
        );
    }
    if summary.stubbed_imports > 0 {
        println!("⚠ {} imports unknown (stubbed)", summary.stubbed_imports);
    }
    if module.tls.is_some() {
        println!("✓ TLS metadata recorded");
    }
    if module.init_va > 0
        || module.init_array_va > 0
        || module.fini_va > 0
        || module.fini_array_va > 0
        || module.preinit_array_va > 0
    {
        println!("✓ Init/fini metadata recorded (not executed)");
    }
    if summary.resolved_imports + summary.known_imports + summary.stubbed_imports == 0 {
        println!("⚠ No imports to resolve");
    }
}
