use crate::*;

pub struct BinaryImageBuilder;

impl BinaryImageBuilder {
    pub fn build_from_file(data: Vec<u8>, sha256: &str, catalog: &ps5_nid::Catalog) -> BinaryImage {
        match ps5_self::SelfImage::parse(&data) {
            Ok(img) => Self::build_from_self(&img, sha256, catalog),
            Err(_) => BinaryImage {
                sha256: sha256.to_string(),
                platform: Platform::Unknown,
                is_self: false,
                file_size: data.len() as u64,
                entry_point: 0,
                segments: Vec::new(),
                imports: Vec::new(),
                exports: Vec::new(),
                relocations: Vec::new(),
                tls: None,
                init_va: 0,
                init_array_va: 0,
                init_array_sz: 0,
                fini_va: 0,
                fini_array_va: 0,
                fini_array_sz: 0,
                preinit_array_va: 0,
                preinit_array_sz: 0,
                import_libs: HashMap::new(),
                needed_files: Vec::new(),
            },
        }
    }

    pub fn build_from_self(
        img: &ps5_self::SelfImage,
        sha256: &str,
        catalog: &ps5_nid::Catalog,
    ) -> BinaryImage {
        let platform = Platform::from_self(img.platform);
        let is_self = img.is_self();
        let file_size = img.data.len() as u64;
        let entry_point = img.elf.header.e_entry;

        let segments = img.elf.program_headers.iter().map(|ph| {
            LoadedSegment {
                vaddr: ph.p_vaddr,
                file_offset: ph.p_offset,
                filesz: ph.p_filesz,
                memsz: ph.p_memsz,
                is_executable: ph.is_executable(),
                is_writable: ph.is_writable(),
            }
        }).collect();

        let imports = Self::build_imports(&img.elf, catalog);
        let exports = Self::build_exports(&img.elf, catalog);
        let relocations = Self::build_relocations(&img.elf);

        let tls = img.elf.tls.as_ref().map(|t| TlsInfo {
            vaddr: t.vaddr,
            filesz: t.filesz,
            memsz: t.memsz,
            align: t.align,
        });

        let needed_files = img.elf.needed_files.clone();
        let import_libs = img.elf.import_libs.clone();

        BinaryImage {
            sha256: sha256.to_string(),
            platform,
            is_self,
            file_size,
            entry_point,
            segments,
            imports,
            exports,
            relocations,
            tls,
            init_va: img.elf.init_va,
            init_array_va: img.elf.init_array_va,
            init_array_sz: img.elf.init_array_sz,
            fini_va: img.elf.fini_va,
            fini_array_va: img.elf.fini_array_va,
            fini_array_sz: img.elf.fini_array_sz,
            preinit_array_va: img.elf.preinit_array_va,
            preinit_array_sz: img.elf.preinit_array_sz,
            import_libs,
            needed_files,
        }
    }

    fn build_imports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ImportEntry> {
        elf.symbols.iter()
            .filter(|s| s.is_import)
            .map(|sym| {
                let parts: Vec<&str> = sym.resolved_name.split('#').collect();
                let nid = parts[0];
                let lib_id = ps5_nid::lib_id_from_nid(&sym.resolved_name).unwrap_or(0);
                let lib_name = elf.import_libs.get(&lib_id).cloned()
                    .unwrap_or_else(|| format!("lib_{}", parts.get(1).unwrap_or(&"?")));
                let resolved = catalog.resolve(nid).map(|s| s.to_string());

                ImportEntry {
                    nid_hash: nid.to_string(),
                    resolved_name: resolved,
                    library_id: lib_id,
                    library_name: lib_name,
                }
            })
            .collect()
    }

    fn build_exports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ExportEntry> {
        elf.symbols.iter()
            .filter(|s| !s.is_import && s.st_value != 0)
            .map(|sym| {
                let resolved = if sym.resolved_name.contains('#') {
                    let nid = sym.resolved_name.split('#').next().unwrap_or("");
                    catalog.resolve(nid).map(|s| s.to_string())
                } else {
                    Some(sym.resolved_name.clone())
                };
                ExportEntry {
                    nid_hash: if sym.resolved_name.contains('#') {
                        sym.resolved_name.split('#').next().unwrap_or("").to_string()
                    } else {
                        String::new()
                    },
                    resolved_name: resolved,
                    vaddr: sym.st_value,
                    size: sym.st_size,
                }
            })
            .collect()
    }

    fn build_relocations(elf: &ps5_elf::ElfImage) -> Vec<RelocationEntry> {
        elf.relocations.iter().map(|r| RelocationEntry {
            offset: r.r_offset,
            info: r.r_info,
            addend: r.r_addend,
            r_type: r.r_type(),
            r_sym: r.r_sym(),
            is_plt: r.is_plt,
        }).collect()
    }
}
