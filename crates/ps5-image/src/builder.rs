use crate::*;

pub struct BinaryImageBuilder;

impl BinaryImageBuilder {
    pub fn build_from_file(data: &[u8], sha256: &str, catalog: &ps5_nid::Catalog) -> BinaryImage {
        match ps5_self::SelfImage::parse(data) {
            Ok(img) => Self::build_from_self(&img, sha256, catalog),
            Err(_) => BinaryImage {
                sha256: sha256.to_string(),
                platform: Platform::Unknown,
                is_self: false,
                file_size: data.len() as u64,
                entry_point: 0,
                metadata: BinaryMetadata::default(),
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
                dynamic_entries: Vec::new(),
                version_defs: Vec::new(),
                lib_versions: Vec::new(),
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
        let _prx_validation = ps5_prx::PrxModule::from_elf("module", &img.elf, catalog).ok();

        let segments = img
            .elf
            .program_headers
            .iter()
            .enumerate()
            .map(|(i, ph)| {
                let is_data_seg = img
                    .segments
                    .iter()
                    .any(|s| s.is_data() && s.phdr_index() as usize == i);
                let self_seg = img
                    .segments
                    .iter()
                    .find(|s| s.is_data() && s.phdr_index() as usize == i);

                LoadedSegment {
                    vaddr: ph.p_vaddr,
                    file_offset: ph.p_offset,
                    filesz: ph.p_filesz,
                    memsz: ph.p_memsz,
                    is_executable: ph.is_executable(),
                    is_writable: ph.is_writable(),
                    seg_type: SegmentType::from_u32(ph.p_type),
                    p_paddr: ph.p_paddr,
                    p_align: ph.p_align,
                    is_encrypted: self_seg.is_some_and(|s| s.is_encrypted()),
                    is_compressed: self_seg.is_some_and(|s| s.is_compressed()),
                    phdr_index: if is_data_seg {
                        self_seg.map(|s| s.phdr_index() as u16)
                    } else {
                        None
                    },
                }
            })
            .collect();

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

        // Build section headers
        let sections = Self::build_section_headers(&img.elf);

        // Build metadata
        let metadata = BinaryMetadata {
            build_id: ps5_elf::section::find_build_id(img.elf.data, &img.elf.section_headers),
            elf_type: img.elf.header.e_type,
            elf_flags: img.elf.header.e_flags,
            osabi: img.elf.header.ei_osabi,
            ei_abi_version: img.elf.header.ei_abi_version,
            e_version: img.elf.header.e_version,
            self_key_type: if img.is_self() {
                Some(img.self_header.key_type)
            } else {
                None
            },
            self_attr: if img.is_self() {
                Some(img.self_header.attr)
            } else {
                None
            },
            self_mode: if img.is_self() {
                Some(img.self_header.mode)
            } else {
                None
            },
            self_endian: if img.is_self() {
                Some(img.self_header.endian)
            } else {
                None
            },
            self_version: if img.is_self() {
                Some(img.self_header.version)
            } else {
                None
            },
            self_flags: if img.is_self() {
                Some(img.self_header.flags)
            } else {
                None
            },
            sections,
        };

        // Build dynamic entries
        let dynamic_entries = img
            .elf
            .dynamic_entries
            .iter()
            .map(|e| DynamicEntry {
                tag: e.d_tag,
                value: e.d_val,
                resolved_tag: DynamicEntry::tag_name(e.d_tag).map(|s| s.to_string()),
            })
            .collect();

        // Build version defs (empty for now — PS5 ELF may not use standard .gnu.version_d)
        let version_defs = Vec::new();

        // Build lib versions from PT_SCE_LIBVERSION segment
        let lib_versions = img
            .elf
            .lib_versions
            .iter()
            .map(|lv| crate::LibVersionEntry {
                name: lv.name.clone(),
                version_raw: lv.version_raw,
                version_string: lv.guessed_version_string(),
            })
            .collect();

        BinaryImage {
            sha256: sha256.to_string(),
            platform,
            is_self,
            file_size,
            entry_point,
            metadata,
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
            dynamic_entries,
            version_defs,
            lib_versions,
        }
    }

    fn build_imports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ImportEntry> {
        elf.symbols
            .iter()
            .filter(|s| s.is_import)
            .enumerate()
            .map(|(idx, sym)| {
                let parts: Vec<&str> = sym.resolved_name.split('#').collect();
                let nid = parts[0];
                let lib_id = ps5_nid::lib_id_from_nid(&sym.resolved_name).unwrap_or(0);
                let lib_name = elf
                    .import_libs
                    .get(&lib_id)
                    .cloned()
                    .unwrap_or_else(|| format!("lib_{}", parts.get(1).unwrap_or(&"?")));
                let resolved = catalog
                    .resolve(nid)
                    .and_then(|e| e.primary_name().map(str::to_string));

                ImportEntry {
                    nid_hash: nid.to_string(),
                    resolved_name: resolved,
                    library_id: lib_id,
                    library_name: lib_name,
                    value: sym.st_value,
                    size: sym.st_size,
                    shndx: sym.st_shndx,
                    binding: SymbolBinding::from_u8(sym.st_info >> 4),
                    sym_type: SymbolType::from_u8(sym.st_info & 0xf),
                    visibility: SymbolVisibility::from_u8(sym.st_other & 0x3),
                    ordinal: idx as u32,
                }
            })
            .collect()
    }

    fn build_exports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ExportEntry> {
        elf.symbols
            .iter()
            .filter(|s| !s.is_import && s.st_value != 0)
            .map(|sym| {
                let resolved = if sym.resolved_name.contains('#') {
                    let nid = sym.resolved_name.split('#').next().unwrap_or("");
                    catalog
                        .resolve(nid)
                        .and_then(|e| e.primary_name().map(str::to_string))
                } else {
                    Some(sym.resolved_name.clone())
                };
                ExportEntry {
                    nid_hash: if sym.resolved_name.contains('#') {
                        sym.resolved_name
                            .split('#')
                            .next()
                            .unwrap_or("")
                            .to_string()
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
        elf.relocations
            .iter()
            .map(|r| RelocationEntry {
                offset: r.r_offset,
                addend: r.r_addend,
                kind: RelocationKind::from_u32(r.r_type()),
                symbol_index: r.r_sym(),
                is_plt: r.is_plt,
            })
            .collect()
    }

    fn build_section_headers(elf: &ps5_elf::ElfImage) -> Vec<SectionHeader> {
        if elf.section_headers.is_empty() {
            return Vec::new();
        }

        // Find shstrtab section to resolve names
        let shstrtab_section = elf
            .section_headers
            .iter()
            .find(|s| {
                s.sh_type == ps5_format::elf_constants::SHT_STRTAB
                    && elf.header.shstrndx != u16::MAX
                    && elf
                        .section_headers
                        .get(elf.header.shstrndx as usize)
                        .map(|ss| ss.sh_offset == s.sh_offset && ss.sh_size == s.sh_size)
                        .unwrap_or(false)
            })
            .or_else(|| {
                elf.section_headers
                    .get(elf.header.shstrndx as usize)
                    .filter(|s| s.sh_type == ps5_format::elf_constants::SHT_STRTAB)
            });

        let shstrtab_offset = shstrtab_section.map(|s| s.sh_offset).unwrap_or(0);

        elf.section_headers
            .iter()
            .map(|sh| {
                let name =
                    ps5_elf::section::resolve_section_name(elf.data, shstrtab_offset, sh.sh_name);
                SectionHeader {
                    name,
                    sh_type: sh.sh_type,
                    sh_addr: sh.sh_addr,
                    sh_offset: sh.sh_offset,
                    sh_size: sh.sh_size,
                    sh_flags: sh.sh_flags,
                    sh_flags_str: SectionHeader::flags_string(sh.sh_flags),
                    sh_info: sh.sh_info,
                    sh_link: sh.sh_link,
                    sh_addralign: sh.sh_addralign,
                    sh_entsize: sh.sh_entsize,
                }
            })
            .collect()
    }

    pub fn build_from_prx_module(
        prx: &ps5_prx::PrxModule,
        base: &BinaryImage,
    ) -> BinaryImage {
        let mut img = base.clone();
        img.needed_files = prx.metadata.needed_files.clone();
        let libs: std::collections::HashMap<u16, String> = prx
            .metadata
            .import_libs
            .iter()
            .enumerate()
            .map(|(i, lib)| (i as u16, lib.clone()))
            .collect();
        img.import_libs = libs;
        img
    }
}
