use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    }
}

fn load_file(path: &PathBuf) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        std::process::exit(1);
    })
}

fn lib_id_from_nid(nid: &str) -> Option<u16> {
    if let Some(hash_end) = nid.find('#') {
        let lib_str = &nid[hash_end + 1..];
        // Sony base64: ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-
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

            let catalog = ps5_nid::Catalog::new();
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
