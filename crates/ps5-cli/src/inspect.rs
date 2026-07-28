use std::path::PathBuf;

use crate::catalog::load_catalog;
use crate::util::{e_version_name, load_file, osabi_name, write_to_output_or_stdout};

#[allow(clippy::print_literal)]
pub(crate) fn cmd_inspect(path: &PathBuf, json: bool, output: &Option<PathBuf>) {
    let data = load_file(path);
    let sha256 = ps5_format::sha256_hex(&data);
    let catalog = load_catalog(&[]);
    let image = ps5_image::BinaryImageBuilder::build_from_file(&data, &sha256, &catalog);

    if json {
        write_to_output_or_stdout(output, &|w| {
            ps5_image::json::export_json(&image, w).map_err(std::io::Error::other)
        });
        return;
    }

    println!(
        "ps5rs v{} — PS5 binary inspector",
        env!("CARGO_PKG_VERSION")
    );
    println!("File: {}", path.display());
    println!("Size: {} bytes", image.file_size);
    println!();

    println!("Platform: {}", image.platform);
    println!("SELF: {}", image.is_self);
    println!("SHA-256: {}", &image.sha256[..16]);
    println!("Entry point: {:#x}", image.entry_point);
    println!("ELF type: {:#x}", image.metadata.elf_type);
    println!("ELF flags: {:#x}", image.metadata.elf_flags);
    println!(
        "OS/ABI: {} ({:#x})",
        osabi_name(image.metadata.osabi),
        image.metadata.osabi
    );
    println!("ABI Version: {}", image.metadata.ei_abi_version);
    println!(
        "ELF Version: {} ({})",
        image.metadata.e_version,
        e_version_name(image.metadata.e_version)
    );
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
        println!(
            "TLS: vaddr={:#x} filesz={:#x} memsz={:#x}",
            tls.vaddr, tls.filesz, tls.memsz
        );
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
        let mut lib_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for imp in &image.imports {
            let label = format!(
                "{}: {}",
                imp.library_name,
                imp.resolved_name.as_deref().unwrap_or("?")
            );
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
pub(crate) fn cmd_imports(path: &PathBuf, json: bool, output: &Option<PathBuf>) {
    let data = load_file(path);
    let sha256 = ps5_format::sha256_hex(&data);
    let catalog = ps5_nid::Catalog::new();
    let image = ps5_image::BinaryImageBuilder::build_from_file(&data, &sha256, &catalog);

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
pub(crate) fn cmd_segments(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    println!("Program headers from {}", path.display());
    println!(
        "{:<4} {:<16} {:<12} {:<18} {:<18} {:<18} {:<18} {}",
        "#", "Type", "Flags", "Offset", "VAddr", "FileSz", "MemSz", "Mapped File Offset"
    );
    println!("{}", "-".repeat(140));

    for (i, ph) in img.elf.program_headers.iter().enumerate() {
        let flags_str = format!(
            "{}{}{}",
            if ph.is_readable() { "R" } else { "-" },
            if ph.is_writable() { "W" } else { "-" },
            if ph.is_executable() { "X" } else { "-" }
        );

        println!(
            "{:<4} {:<16} {:<12} {:#018x} {:#018x} {:#018x} {:#018x}",
            i,
            ph.type_name(),
            flags_str,
            ph.p_offset,
            ph.p_vaddr,
            ph.p_filesz,
            ph.p_memsz
        );
    }

    if !img.segments.is_empty() {
        println!("\nSELF data segments:");
        for (i, seg) in img.segments.iter().enumerate() {
            println!(
                "  [{i}] phdr_index={} offset={:#x} file_size={:#x} mem_size={:#x}",
                seg.phdr_index(),
                seg.file_offset,
                seg.file_size,
                seg.mem_size
            );
        }
    }
}

#[allow(clippy::print_literal)]
pub(crate) fn cmd_dynamic(path: &PathBuf) {
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
pub(crate) fn cmd_symbols(path: &PathBuf) {
    let data = load_file(path);
    let img = ps5_self::SelfImage::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(1);
    });

    let elf = &img.elf;
    println!("Symbols from {} ({})", path.display(), elf.symbols.len());
    println!(
        "symtab_offset={:#x} strtab_offset={:#x} strtab_size={:#x} symtab_size={:#x}",
        elf.symtab_offset, elf.strtab_offset, elf.strtab_size, elf.symtab_size
    );
    println!();
    println!(
        "{:<6} {:<10} {:<10} {:<6} {:<18} {:<18} {:<8} {}",
        "#", "shndx", "info", "bind", "value", "size", "name_off", "name"
    );
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
        println!(
            "{:<6} {:<#10x} {:<#10x} {:<6} {:#018x} {:#018x} {:<#10x} \"{}\"",
            i,
            sym.st_shndx,
            sym.st_info,
            bind_str,
            sym.st_value,
            sym.st_size,
            sym.st_name,
            sym.resolved_name
        );
    }

    if elf.symbols.len() > limit {
        println!("... and {} more", elf.symbols.len() - limit);
    }
}

pub(crate) fn cmd_nid(name: &str) {
    let nid = ps5_nid::hash(name);
    println!("{name} -> {nid}");
}
