mod header;
mod program;
mod dynamic;
mod relocation;
pub mod section;
mod symbol;

pub use header::ElfHeader;
pub use program::ProgramHeader;
pub use dynamic::DynEntry;
pub use relocation::RelaEntry;
pub use section::ElfSectionHeader;
pub use symbol::SymEntry;

#[derive(Debug, Clone)]
pub struct TlsInfo {
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone)]
pub struct ElfImage<'a> {
    pub data: &'a [u8],
    pub elf_base: usize,
    pub header: ElfHeader,
    pub program_headers: Vec<ProgramHeader>,
    pub section_headers: Vec<ElfSectionHeader>,
    pub dynamic_entries: Vec<DynEntry>,
    pub relocations: Vec<RelaEntry>,
    pub symbols: Vec<SymEntry>,
    pub tls: Option<TlsInfo>,
    pub init_va: u64,
    pub init_array_va: u64,
    pub init_array_sz: u64,
    pub fini_va: u64,
    pub fini_array_va: u64,
    pub fini_array_sz: u64,
    pub preinit_array_va: u64,
    pub preinit_array_sz: u64,
    pub strtab_offset: u64,
    pub strtab_size: u64,
    pub symtab_offset: u64,
    pub symtab_size: u64,
    pub import_libs: std::collections::HashMap<u16, String>,
    pub needed_files: Vec<String>,
}

impl<'a> ElfImage<'a> {
    fn vaddr_to_offset(
        program_headers: &[ProgramHeader],
        phdr_file_offsets: Option<&[u64]>,
        vaddr: u64,
    ) -> u64 {
        for (i, ph) in program_headers.iter().enumerate() {
            if ph.p_type != ps5_format::elf_constants::PT_LOAD {
                continue;
            }
            if vaddr >= ph.p_vaddr && vaddr < ph.p_vaddr + ph.p_filesz {
                let foff = phdr_file_offsets
                    .and_then(|offsets| offsets.get(i).copied())
                    .unwrap_or(ph.p_offset);
                return foff + (vaddr - ph.p_vaddr);
            }
        }
        vaddr
    }

    /// Parse an ELF image.
    ///
    /// `phdr_file_offsets` is an optional mapping from program header index to actual
    /// file offset in the container. In a SELF file, `p_offset` is logical; the real
    /// data lives in the SELF segment table. Pass the remapped offsets here.
    pub fn parse(data: &'a [u8], phdr_file_offsets: Option<&[u64]>) -> ps5_format::Result<Self> {
        let header = ElfHeader::parse(data, 0)?;
        let mut program_headers = Vec::new();
        for i in 0..header.phnum {
            let offset = header.e_phoff as usize + i as usize * header.phentsize as usize;
            program_headers.push(ProgramHeader::parse(data, offset)?);
        }

        let section_headers = section::parse_section_headers(
            data,
            header.e_shoff,
            header.shnum,
            header.shentsize,
            header.shstrndx,
        )?;

        let mut tls = None;
        let mut init_va = 0u64;
        let mut init_array_va = 0u64;
        let mut init_array_sz = 0u64;
        let mut fini_va = 0u64;
        let mut fini_array_va = 0u64;
        let mut fini_array_sz = 0u64;
        let mut preinit_array_va = 0u64;
        let mut preinit_array_sz = 0u64;
        let mut dynamic_phdr = None;

        for ph in &program_headers {
            match ph.p_type {
                ps5_format::elf_constants::PT_TLS => {
                    tls = Some(TlsInfo {
                        vaddr: ph.p_vaddr,
                        filesz: ph.p_filesz,
                        memsz: ph.p_memsz,
                        align: ph.p_align,
                    });
                }
                ps5_format::elf_constants::PT_DYNAMIC => {
                    dynamic_phdr = Some(ph.clone());
                }
                _ => {}
            }
        }

        let dyn_entries = if let Some(ref dyn_ph) = dynamic_phdr {
            // For SELF files, p_offset is logical. Resolve DYNAMIC's file offset
            // via vaddr mapping (it typically lives inside a LOAD segment).
            let dyn_file_offset = Self::vaddr_to_offset(&program_headers, phdr_file_offsets, dyn_ph.p_vaddr);
            dynamic::parse_dynamic(data, dyn_ph, dyn_file_offset)?
        } else {
            Vec::new()
        };

        let mut strtab_vaddr = 0u64;
        let mut strtab_size = 0u64;
        let mut symtab_vaddr = 0u64;
        let mut symtab_size = 0u64;
        let mut rela_vaddr = 0u64;
        let mut rela_size = 0u64;
        let mut jmprel_vaddr = 0u64;
        let mut jmprel_size = 0u64;
        let mut syment = 24u64;

        for entry in &dyn_entries {
            match entry.d_tag as u64 {
                ps5_format::elf_constants::DT_STRTAB => strtab_vaddr = entry.d_val,
                ps5_format::elf_constants::DT_STRSZ => strtab_size = entry.d_val,
                ps5_format::elf_constants::DT_SYMTAB => symtab_vaddr = entry.d_val,
                ps5_format::elf_constants::DT_SYMENT => syment = entry.d_val,
                ps5_format::elf_constants::DT_RELA => rela_vaddr = entry.d_val,
                ps5_format::elf_constants::DT_RELASZ => rela_size = entry.d_val,
                ps5_format::elf_constants::DT_JMPREL => jmprel_vaddr = entry.d_val,
                ps5_format::elf_constants::DT_PLTRELSZ => jmprel_size = entry.d_val,
                ps5_format::elf_constants::DT_INIT => init_va = entry.d_val,
                ps5_format::elf_constants::DT_FINI => fini_va = entry.d_val,
                ps5_format::elf_constants::DT_INIT_ARRAY => init_array_va = entry.d_val,
                ps5_format::elf_constants::DT_INIT_ARRAYSZ => init_array_sz = entry.d_val,
                ps5_format::elf_constants::DT_FINI_ARRAY => fini_array_va = entry.d_val,
                ps5_format::elf_constants::DT_FINI_ARRAYSZ => fini_array_sz = entry.d_val,
                ps5_format::elf_constants::DT_PREINIT_ARRAY => preinit_array_va = entry.d_val,
                ps5_format::elf_constants::DT_PREINIT_ARRAYSZ => preinit_array_sz = entry.d_val,
                ps5_format::self_constants::DT_SCE_STRTAB if strtab_vaddr == 0 => strtab_vaddr = entry.d_val,
                ps5_format::self_constants::DT_SCE_STRSZ if strtab_size == 0 => strtab_size = entry.d_val,
                ps5_format::self_constants::DT_SCE_SYMTAB if symtab_vaddr == 0 => symtab_vaddr = entry.d_val,
                ps5_format::self_constants::DT_SCE_SYMTABSZ => symtab_size = entry.d_val,
                ps5_format::self_constants::DT_SCE_RELA if rela_vaddr == 0 => rela_vaddr = entry.d_val,
                ps5_format::self_constants::DT_SCE_RELASZ if rela_size == 0 => rela_size = entry.d_val,
                ps5_format::self_constants::DT_SCE_JMPREL if jmprel_vaddr == 0 => jmprel_vaddr = entry.d_val,
                ps5_format::self_constants::DT_SCE_PLTRELSZ if jmprel_size == 0 => jmprel_size = entry.d_val,
                _ => {}
            }
        }

        let strtab_offset = Self::vaddr_to_offset(&program_headers, phdr_file_offsets, strtab_vaddr);
        let symtab_offset = Self::vaddr_to_offset(&program_headers, phdr_file_offsets, symtab_vaddr);
        let rela_offset = Self::vaddr_to_offset(&program_headers, phdr_file_offsets, rela_vaddr);
        let jmprel_offset = Self::vaddr_to_offset(&program_headers, phdr_file_offsets, jmprel_vaddr);

        symtab_size = if symtab_size == 0 && symtab_vaddr > 0 && strtab_vaddr > symtab_vaddr {
            strtab_vaddr - symtab_vaddr
        } else {
            symtab_size
        };

        let symbols = if symtab_vaddr > 0 && syment > 0 && symtab_size >= syment {
            let count = (symtab_size / syment) as usize;
            symbol::parse_symbols(data, symtab_offset, syment, count, strtab_offset)?
        } else {
            Vec::new()
        };

        let relocations = relocation::parse_all_relocs(data, rela_offset, rela_size, jmprel_offset, jmprel_size)?;

        let import_libs = if (strtab_offset as usize) < data.len() {
            dynamic::parse_import_libs(&dyn_entries, &data[strtab_offset as usize..])
        } else {
            std::collections::HashMap::new()
        };
        let needed_files = if (strtab_offset as usize) < data.len() {
            dynamic::parse_needed_files(&dyn_entries, &data[strtab_offset as usize..])
        } else {
            Vec::new()
        };

        Ok(Self {
            data,
            elf_base: 0,
            header,
            program_headers,
            section_headers,
            dynamic_entries: dyn_entries,
            relocations,
            symbols,
            tls,
            init_va,
            init_array_va,
            init_array_sz,
            fini_va,
            fini_array_va,
            fini_array_sz,
            preinit_array_va,
            preinit_array_sz,
            strtab_offset,
            strtab_size,
            symtab_offset,
            symtab_size,
            import_libs,
            needed_files,
        })
    }

    pub fn resolve_string(&self, offset: u64) -> &str {
        let abs = self.strtab_offset + offset;
        if abs >= self.data.len() as u64 {
            return "";
        }
        let slice = &self.data[abs as usize..];
        let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
        std::str::from_utf8(&slice[..end]).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_format::elf_constants::*;
    use ps5_format::self_constants::DT_SCE_SYMTABSZ;
    use proptest::prelude::*;

    struct ElfBuilder {
        entry: u64,
        phdrs: Vec<PhdrDef>,
        load_data: Vec<u8>,
        load_vaddr: u64,
        load_offset: u64,
    }

    struct PhdrDef {
        p_type: u32,
        p_flags: u32,
        p_offset: u64,
        p_vaddr: u64,
        p_filesz: u64,
        p_memsz: u64,
        p_align: u64,
    }

    impl ElfBuilder {
        fn new() -> Self {
            Self {
                entry: 0x1000,
                phdrs: Vec::new(),
                load_data: Vec::new(),
                load_vaddr: 0x1000,
                load_offset: 0x1000,
            }
        }

        fn with_load(mut self, vaddr: u64, data: Vec<u8>) -> Self {
            self.phdrs.push(PhdrDef {
                p_type: PT_LOAD,
                p_flags: PF_R | PF_X,
                p_offset: self.load_offset,
                p_vaddr: vaddr,
                p_filesz: data.len() as u64,
                p_memsz: data.len() as u64,
                p_align: 0x1000,
            });
            self.load_vaddr = vaddr;
            self.load_data = data;
            self
        }

        fn with_dynamic(mut self, dyn_vaddr: u64, dyn_filesz: u64) -> Self {
            self.phdrs.push(PhdrDef {
                p_type: PT_DYNAMIC,
                p_flags: PF_R,
                p_offset: dyn_vaddr,
                p_vaddr: dyn_vaddr,
                p_filesz: dyn_filesz,
                p_memsz: dyn_filesz,
                p_align: 8,
            });
            self
        }

        fn build(self) -> Vec<u8> {
            let phdr_count = self.phdrs.len();
            let e_phoff: u64 = 64;
            let _phdr_area_end = e_phoff + (phdr_count as u64) * 56;

            let mut file = vec![0u8; self.load_offset as usize + self.load_data.len()];

            let class = ELFCLASS64;
            let endian = ELFDATA2LSB;
            let e_type = ET_SCE_DYNAMIC;
            let e_machine = EM_X86_64;

            let write_u16 = |data: &mut [u8], off: usize, v: u16| {
                data[off..off + 2].copy_from_slice(&v.to_le_bytes());
            };
            let write_u32 = |data: &mut [u8], off: usize, v: u32| {
                data[off..off + 4].copy_from_slice(&v.to_le_bytes());
            };
            let write_u64 = |data: &mut [u8], off: usize, v: u64| {
                data[off..off + 8].copy_from_slice(&v.to_le_bytes());
            };

            file[0..4].copy_from_slice(&ELF_MAGIC);
            file[EI_CLASS] = class;
            file[EI_DATA] = endian;
            file[EI_VERSION] = 1;
            write_u16(&mut file, 16, e_type);
            write_u16(&mut file, 18, e_machine);
            write_u32(&mut file, 20, 1);
            write_u64(&mut file, 24, self.entry);
            write_u64(&mut file, 32, e_phoff);
            write_u16(&mut file, 52, 64);
            write_u16(&mut file, 54, 56);
            write_u16(&mut file, 56, phdr_count as u16);

            let mut phdr_offset = e_phoff as usize;
            for ph in &self.phdrs {
                write_u32(&mut file, phdr_offset, ph.p_type);
                write_u32(&mut file, phdr_offset + 4, ph.p_flags);
                write_u64(&mut file, phdr_offset + 8, ph.p_offset);
                write_u64(&mut file, phdr_offset + 16, ph.p_vaddr);
                write_u64(&mut file, phdr_offset + 24, 0);
                write_u64(&mut file, phdr_offset + 32, ph.p_filesz);
                write_u64(&mut file, phdr_offset + 40, ph.p_memsz);
                write_u64(&mut file, phdr_offset + 48, ph.p_align);
                phdr_offset += 56;
            }

            let data_start = self.load_offset as usize;
            file[data_start..data_start + self.load_data.len()].copy_from_slice(&self.load_data);

            file
        }
    }

    fn write_u64_bytes(buf: &mut Vec<u8>, v: u64) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn _write_u32_bytes(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    fn build_dynamic_entries(entries: &[(u64, u64)]) -> Vec<u8> {
        let mut buf = Vec::new();
        for &(tag, val) in entries {
            write_u64_bytes(&mut buf, tag);
            write_u64_bytes(&mut buf, val);
        }
        write_u64_bytes(&mut buf, 0);
        write_u64_bytes(&mut buf, 0);
        buf
    }

    fn build_strtab(names: &[&[u8]]) -> Vec<u8> {
        let mut buf = vec![0u8];
        for name in names {
            buf.extend_from_slice(name);
            buf.push(0);
        }
        buf
    }

    fn build_symtab_entry(st_name: u32, st_info: u8, st_shndx: u16, st_value: u64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(24);
        buf.extend_from_slice(&st_name.to_le_bytes());
        buf.push(st_info);
        buf.push(0);
        buf.extend_from_slice(&st_shndx.to_le_bytes());
        buf.extend_from_slice(&st_value.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf
    }

    fn build_rela_entry(r_offset: u64, r_info: u64, r_addend: i64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(24);
        buf.extend_from_slice(&r_offset.to_le_bytes());
        buf.extend_from_slice(&r_info.to_le_bytes());
        buf.extend_from_slice(&(r_addend as u64).to_le_bytes());
        buf
    }

    #[test]
    fn parse_minimal_elf_header() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0xCC; 256])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.header.class, ELFCLASS64);
        assert_eq!(img.header.endian, ELFDATA2LSB);
        assert_eq!(img.header.e_machine, EM_X86_64);
        assert_eq!(img.header.e_type, ET_SCE_DYNAMIC);
        assert_eq!(img.header.e_entry, 0x1000);
    }

    #[test]
    fn parse_program_headers() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0xCC; 256])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.program_headers.len(), 1);
        assert_eq!(img.program_headers[0].p_type, PT_LOAD);
        assert_eq!(img.program_headers[0].p_vaddr, 0x1000);
        assert_eq!(img.program_headers[0].p_filesz, 256);
    }

    #[test]
    fn parse_load_helpers() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0xCC; 64])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert!(img.program_headers[0].is_load());
        assert!(!img.program_headers[0].is_dynamic());
        assert!(!img.program_headers[0].is_tls());
        assert!(img.program_headers[0].is_executable());
    }

    #[test]
    fn parse_dynamic_section() {
        let strtab = build_strtab(&[b"hello"]);
        let placeholder_dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0),
            (DT_STRSZ, 0),
            (DT_SYMTAB, 0),
            (DT_SYMENT, 24),
        ]);
        let dyn_size = placeholder_dynamic.len();

        let strtab_vaddr = 0x1000u64 + dyn_size as u64;
        let symtab_vaddr = strtab_vaddr + strtab.len() as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
            (DT_SYMTAB, symtab_vaddr),
            (DT_SYMENT, 24),
        ]);
        assert_eq!(dynamic.len(), dyn_size);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);
        let symtab_start = data.len();
        data.resize(symtab_start + 24, 0);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.dynamic_entries.len(), 5);
        assert_eq!(img.dynamic_entries[0].d_tag, DT_STRTAB);
        assert_eq!(img.dynamic_entries[0].d_val, strtab_vaddr);
        assert_eq!(img.strtab_size, strtab.len() as u64);
        assert_eq!(img.symtab_offset, symtab_vaddr);
    }

    #[test]
    fn parse_needed_files() {
        let strtab = build_strtab(&[b"libSceFoo", b"libSceBar"]);
        let placeholder_dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0), (DT_STRSZ, 0), (DT_NEEDED, 0), (DT_NEEDED, 0),
        ]);
        let dyn_size = placeholder_dynamic.len();
        let strtab_vaddr = 0x1000u64 + dyn_size as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
            (DT_NEEDED, 1),
            (DT_NEEDED, 11),
        ]);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.needed_files.len(), 2);
        assert_eq!(img.needed_files[0], "libSceFoo");
        assert_eq!(img.needed_files[1], "libSceBar");
    }

    #[test]
    fn parse_tls_info() {
        let mut data = vec![0u8; 0x100];
        data[0] = 42;

        let mut builder = ElfBuilder::new().with_load(0x1000, data);
        builder.phdrs.push(PhdrDef {
            p_type: PT_TLS,
            p_flags: PF_R,
            p_offset: 0x1000,
            p_vaddr: 0x2000,
            p_filesz: 1,
            p_memsz: 64,
            p_align: 8,
        });
        let elf = builder.build();

        let img = ElfImage::parse(&elf, None).unwrap();
        let tls = img.tls.as_ref().unwrap();
        assert_eq!(tls.vaddr, 0x2000);
        assert_eq!(tls.filesz, 1);
        assert_eq!(tls.memsz, 64);
        assert_eq!(tls.align, 8);
    }

    #[test]
    fn parse_no_dynamic() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0; 64])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert!(img.dynamic_entries.is_empty());
        assert_eq!(img.symbols.len(), 0);
        assert_eq!(img.relocations.len(), 0);
    }

    #[test]
    fn parse_import_symbols() {
        let strtab = build_strtab(&[b"hello#libSceFoo"]);
        let symtab = build_symtab_entry(1, 0, 0, 0);

        let placeholder_dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0), (DT_STRSZ, 0), (DT_SYMTAB, 0),
            (DT_SYMENT, 0), (DT_SCE_SYMTABSZ, 0),
        ]);
        let dyn_size = placeholder_dynamic.len();
        let strtab_vaddr = 0x1000u64 + dyn_size as u64;
        let symtab_vaddr = strtab_vaddr + strtab.len() as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
            (DT_SYMTAB, symtab_vaddr),
            (DT_SYMENT, 24),
            (DT_SCE_SYMTABSZ, 24),
        ]);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);
        data.extend_from_slice(&symtab);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.symbols.len(), 1);
        assert!(img.symbols[0].is_import);
        assert_eq!(img.symbols[0].resolved_name, "hello#libSceFoo");
    }

    #[test]
    fn parse_relocations() {
        let rela = build_rela_entry(0x400000, R_X86_64_GLOB_DAT as u64, 0);

        let placeholder_dynamic = build_dynamic_entries(&[(DT_RELA, 0), (DT_RELASZ, 0)]);
        let dyn_size = placeholder_dynamic.len();
        let rela_vaddr = 0x1000u64 + dyn_size as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_RELA, rela_vaddr),
            (DT_RELASZ, 24),
        ]);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&rela);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.relocations.len(), 1);
        assert_eq!(img.relocations[0].r_offset, 0x400000);
        assert_eq!(img.relocations[0].r_type(), R_X86_64_GLOB_DAT);
        assert!(!img.relocations[0].is_plt);
    }

    #[test]
    fn parse_jmprel_plt_relocations() {
        let rela = build_rela_entry(0x400008, R_X86_64_JUMP_SLOT as u64, 0);

        let placeholder_dynamic = build_dynamic_entries(&[(DT_JMPREL, 0), (DT_PLTRELSZ, 0)]);
        let dyn_size = placeholder_dynamic.len();
        let rela_vaddr = 0x1000u64 + dyn_size as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_JMPREL, rela_vaddr),
            (DT_PLTRELSZ, 24),
        ]);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&rela);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.relocations.len(), 1);
        assert_eq!(img.relocations[0].r_type(), R_X86_64_JUMP_SLOT);
        assert!(img.relocations[0].is_plt);
    }

    #[test]
    fn parse_init_va() {
        let mut data = vec![0u8; 0x200];
        let dynamic = build_dynamic_entries(&[
            (DT_INIT, 0x1050),
            (DT_INIT_ARRAY, 0x1060),
            (DT_INIT_ARRAYSZ, 2),
        ]);
        data[0..dynamic.len()].copy_from_slice(&dynamic);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.init_va, 0x1050);
        assert_eq!(img.init_array_va, 0x1060);
        assert_eq!(img.init_array_sz, 2);
    }

    #[test]
    fn parse_fini_and_preinit() {
        let mut data = vec![0u8; 0x200];
        let dynamic = build_dynamic_entries(&[
            (DT_FINI, 0x2050),
            (DT_FINI_ARRAY, 0x2060),
            (DT_FINI_ARRAYSZ, 3),
            (DT_PREINIT_ARRAY, 0x3000),
            (DT_PREINIT_ARRAYSZ, 1),
        ]);
        data[0..dynamic.len()].copy_from_slice(&dynamic);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.fini_va, 0x2050);
        assert_eq!(img.fini_array_va, 0x2060);
        assert_eq!(img.fini_array_sz, 3);
        assert_eq!(img.preinit_array_va, 0x3000);
        assert_eq!(img.preinit_array_sz, 1);
    }

    #[test]
    fn parse_wrong_machine() {
        let mut elf = ElfBuilder::new()
            .with_load(0x1000, vec![0xCC; 64])
            .build();
        // e_machine is at offset 18 (2 bytes LE), overwrite EM_X86_64 (0x3e) with 0x28 (ARM)
        elf[18] = 0x28;
        elf[19] = 0x00;
        let result = ElfImage::parse(&elf, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            ps5_format::ParseError::NotX86_64(m) => assert_eq!(m, 0x28),
            other => panic!("expected NotX86_64, got: {other:?}"),
        }
    }

    #[test]
    fn vaddr_to_offset_roundtrip() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0; 0x400])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();

        let offsets = [0x1000u64, 0x1100, 0x1200, 0x13FF];
        for vaddr in offsets {
            let offset = ElfImage::vaddr_to_offset(&img.program_headers, None, vaddr);
            assert!(offset >= 0x1000 && offset < 0x1400, "vaddr {vaddr:#x} → offset {offset:#x}");
            assert_eq!(offset, vaddr, "for raw ELF p_offset==p_vaddr, so vaddr==offset");
        }
    }

    #[test]
    fn vaddr_to_offset_outside_load() {
        let elf = ElfBuilder::new()
            .with_load(0x1000, vec![0; 0x100])
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();

        let offset = ElfImage::vaddr_to_offset(&img.program_headers, None, 0x5000);
        assert_eq!(offset, 0x5000, "vaddr outside any LOAD returns vaddr itself");
    }

    #[test]
    fn vaddr_to_offset_with_custom_mapping() {
        let mut data = vec![0u8; 0x500];
        data[0x100] = 0x42;

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        let custom_offsets = [0x100u64];
        let offset = ElfImage::vaddr_to_offset(&img.program_headers, Some(&custom_offsets), 0x1100);
        assert_eq!(offset, 0x200, "custom mapping: 0x100 + (0x1100 - 0x1000)");
    }

    #[test]
    fn resolve_string_from_strtab() {
        let strtab = build_strtab(&[b"hello", b"world"]);
        let placeholder_dynamic = build_dynamic_entries(&[(DT_STRTAB, 0), (DT_STRSZ, 0)]);
        let dyn_size = placeholder_dynamic.len();
        let strtab_vaddr = 0x1000u64 + dyn_size as u64;

        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
        ]);

        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);

        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();

        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.resolve_string(1), "hello");
        assert_eq!(img.resolve_string(7), "world");
    }

    #[test]
    fn parse_truncated_elf() {
        let data = vec![0u8; 10];
        let result = ElfImage::parse(&data, None);
        assert!(result.is_err());
    }

    #[test]
    fn parse_bad_magic() {
        let mut data = vec![0u8; 256];
        data[0] = 0xFF;
        let result = ElfImage::parse(&data, None);
        assert!(result.is_err());
    }

    #[test]
    fn parse_wrong_class() {
        let mut data = vec![0u8; 256];
        data[0..4].copy_from_slice(&ELF_MAGIC);
        data[EI_CLASS] = 1;
        let result = ElfImage::parse(&data, None);
        assert!(result.is_err());
    }

    #[test]
    fn parse_big_endian() {
        let mut data = vec![0u8; 256];
        data[0..4].copy_from_slice(&ELF_MAGIC);
        data[EI_CLASS] = ELFCLASS64;
        data[EI_DATA] = 2;
        let result = ElfImage::parse(&data, None);
        assert!(result.is_err());
    }

    #[test]
    fn multiple_load_segments() {
        let mut builder = ElfBuilder::new();
        builder.load_vaddr = 0x1000;
        builder.load_offset = 0x1000;
        builder.load_data = vec![0xAA; 0x100];
        builder.phdrs.push(PhdrDef {
            p_type: PT_LOAD,
            p_flags: PF_R | PF_X,
            p_offset: 0x1000,
            p_vaddr: 0x1000,
            p_filesz: 0x100,
            p_memsz: 0x100,
            p_align: 0x1000,
        });
        builder.phdrs.push(PhdrDef {
            p_type: PT_LOAD,
            p_flags: PF_R | PF_W,
            p_offset: 0x1100,
            p_vaddr: 0x2000,
            p_filesz: 0x100,
            p_memsz: 0x200,
            p_align: 0x1000,
        });
        let mut file = vec![0u8; 0x1200];
        let phdr_count = 2;
        let write_u16 = |data: &mut [u8], off: usize, v: u16| {
            data[off..off + 2].copy_from_slice(&v.to_le_bytes());
        };
        let write_u32 = |data: &mut [u8], off: usize, v: u32| {
            data[off..off + 4].copy_from_slice(&v.to_le_bytes());
        };
        let write_u64 = |data: &mut [u8], off: usize, v: u64| {
            data[off..off + 8].copy_from_slice(&v.to_le_bytes());
        };
        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;
        write_u16(&mut file, 16, ET_SCE_DYNAMIC);
        write_u16(&mut file, 18, EM_X86_64);
        write_u64(&mut file, 24, 0x1000);
        write_u64(&mut file, 32, 64);
        write_u16(&mut file, 52, 64);
        write_u16(&mut file, 54, 56);
        write_u16(&mut file, 56, phdr_count);

        for (i, ph) in builder.phdrs.iter().enumerate() {
            let off = 64 + i * 56;
            write_u32(&mut file, off, ph.p_type);
            write_u32(&mut file, off + 4, ph.p_flags);
            write_u64(&mut file, off + 8, ph.p_offset);
            write_u64(&mut file, off + 16, ph.p_vaddr);
            write_u64(&mut file, off + 32, ph.p_filesz);
            write_u64(&mut file, off + 40, ph.p_memsz);
            write_u64(&mut file, off + 48, ph.p_align);
        }
        file[0x1000..0x1100].copy_from_slice(&vec![0xAA; 0x100]);
        file[0x1100..0x1200].copy_from_slice(&vec![0xBB; 0x100]);

        let img = ElfImage::parse(&file, None).unwrap();
        assert_eq!(img.program_headers.len(), 2);
        assert_eq!(img.program_headers[0].p_type, PT_LOAD);
        assert_eq!(img.program_headers[1].p_type, PT_LOAD);
        assert_eq!(img.program_headers[0].p_vaddr, 0x1000);
        assert_eq!(img.program_headers[1].p_vaddr, 0x2000);

        let off1 = ElfImage::vaddr_to_offset(&img.program_headers, None, 0x1000);
        assert_eq!(off1, 0x1000);
        let off2 = ElfImage::vaddr_to_offset(&img.program_headers, None, 0x2000);
        assert_eq!(off2, 0x1100);
    }

    #[test]
    fn overlapping_pt_load_segments() {
        let mut builder = ElfBuilder::new();
        builder.load_vaddr = 0x1000;
        builder.load_offset = 0x1000;
        builder.load_data = vec![0xAA; 0x200];
        builder.phdrs.push(PhdrDef {
            p_type: PT_LOAD,
            p_flags: PF_R | PF_X,
            p_offset: 0x1000,
            p_vaddr: 0x1000,
            p_filesz: 0x200,
            p_memsz: 0x200,
            p_align: 0x1000,
        });
        builder.phdrs.push(PhdrDef {
            p_type: PT_LOAD,
            p_flags: PF_R | PF_W,
            p_offset: 0x1100,
            p_vaddr: 0x1800,
            p_filesz: 0x100,
            p_memsz: 0x200,
            p_align: 0x1000,
        });
        let elf = builder.build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.program_headers.len(), 2);
        let o = ElfImage::vaddr_to_offset(&img.program_headers, None, 0x1800);
        assert_eq!(o, 0x1100);
    }

    #[test]
    fn strtab_outside_load_returns_empty() {
        let strtab = build_strtab(&[b"ghost"]);
        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0xDEAD0000),
            (DT_STRSZ, strtab.len() as u64),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.resize(0x100, 0);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.resolve_string(1), "");
    }

    #[test]
    fn reloc_past_eof_yields_empty() {
        let placeholder = build_dynamic_entries(&[(DT_RELA, 0), (DT_RELASZ, 0)]);
        let dyn_size = placeholder.len();
        let rela_vaddr = 0x1000u64 + dyn_size as u64;
        let dynamic = build_dynamic_entries(&[
            (DT_RELA, rela_vaddr + 0x100000),
            (DT_RELASZ, 9999),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.resize(0x200, 0);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.relocations.len(), 0);
    }

    #[test]
    fn duplicate_dynamic_tags_last_wins() {
        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0x9999),
            (DT_STRTAB, 0x1000),
            (DT_STRSZ, 100),
            (DT_STRSZ, 200),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.resize(0x200, 0);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.dynamic_entries.len(), 5);
        assert_eq!(img.strtab_size, 200);
    }

    #[test]
    fn empty_strtab_no_symbols() {
        let strtab = vec![0u8];
        let placeholder_dynamic = build_dynamic_entries(&[
            (DT_STRTAB, 0), (DT_STRSZ, 0), (DT_SYMTAB, 0),
            (DT_SYMENT, 0), (DT_SCE_SYMTABSZ, 0),
        ]);
        let dyn_size = placeholder_dynamic.len();
        let strtab_vaddr = 0x1000u64 + dyn_size as u64;
        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, 1),
            (DT_SYMTAB, strtab_vaddr + 1),
            (DT_SYMENT, 24),
            (DT_SCE_SYMTABSZ, 0),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);
        data.resize(0x400, 0);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.symbols.len(), 0);
    }

    #[test]
    fn zero_size_tls_segment() {
        let mut builder = ElfBuilder::new().with_load(0x1000, vec![0; 0x100]);
        builder.phdrs.push(PhdrDef {
            p_type: PT_TLS,
            p_flags: PF_R,
            p_offset: 0x1000,
            p_vaddr: 0x2000,
            p_filesz: 0,
            p_memsz: 0,
            p_align: 8,
        });
        let elf = builder.build();
        let img = ElfImage::parse(&elf, None).unwrap();
        let tls = img.tls.as_ref().unwrap();
        assert_eq!(tls.filesz, 0);
        assert_eq!(tls.memsz, 0);
    }

    #[test]
    fn non_utf8_strtab_resolves_empty() {
        let mut strtab = vec![0u8];
        strtab.extend_from_slice(&[0xFF, 0xFE, 0x00, b'a', 0x00]);
        let placeholder = build_dynamic_entries(&[(DT_STRTAB, 0), (DT_STRSZ, 0)]);
        let dyn_size = placeholder.len();
        let strtab_vaddr = 0x1000u64 + dyn_size as u64;
        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&strtab);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.resolve_string(1), "");
        assert_eq!(img.resolve_string(4), "a");
    }

    #[test]
    fn reloc_both_rela_and_jmprel() {
        let rela1 = build_rela_entry(0x400000, R_X86_64_GLOB_DAT as u64, 0);
        let rela2 = build_rela_entry(0x400008, R_X86_64_JUMP_SLOT as u64, 0);
        let placeholder = build_dynamic_entries(&[
            (DT_RELA, 0), (DT_RELASZ, 0), (DT_JMPREL, 0), (DT_PLTRELSZ, 0),
        ]);
        let dyn_size = placeholder.len();
        let rela_vaddr = 0x1000u64 + dyn_size as u64;
        let jmprel_vaddr = rela_vaddr + 24;
        let dynamic = build_dynamic_entries(&[
            (DT_RELA, rela_vaddr),
            (DT_RELASZ, 24),
            (DT_JMPREL, jmprel_vaddr),
            (DT_PLTRELSZ, 24),
        ]);
        let mut data = Vec::new();
        data.extend_from_slice(&dynamic);
        data.extend_from_slice(&rela1);
        data.extend_from_slice(&rela2);
        let elf = ElfBuilder::new()
            .with_load(0x1000, data)
            .with_dynamic(0x1000, dynamic.len() as u64)
            .build();
        let img = ElfImage::parse(&elf, None).unwrap();
        assert_eq!(img.relocations.len(), 2);
        assert!(!img.relocations[0].is_plt);
        assert!(img.relocations[1].is_plt);
    }

    proptest! {
        #[test]
        fn any_builder_elf_parses(vaddr in 0x1000u64..0xFFFFFFFFu64) {
            let elf = ElfBuilder::new().with_load(vaddr, vec![0xCC; 64]).build();
            let img = ElfImage::parse(&elf, None).unwrap();
            assert!(img.header.e_type == ET_SCE_DYNAMIC);
            assert_eq!(img.program_headers.len(), 1);
            assert_eq!(img.program_headers[0].p_vaddr, vaddr);
            assert_eq!(img.dynamic_entries.len(), 0);
        }

        #[test]
        fn vaddr_to_offset_within_load(vaddr in 0x1000u64..0x1400u64) {
            let elf = ElfBuilder::new()
                .with_load(0x1000, vec![0; 0x400])
                .build();
            let img = ElfImage::parse(&elf, None).unwrap();
            let offset = ElfImage::vaddr_to_offset(&img.program_headers, None, vaddr);
            assert_eq!(offset, vaddr);
        }

        #[test]
        fn vaddr_outside_load_returns_vaddr(vaddr in 0x5000u64..0xFFFFFFFFu64) {
            let elf = ElfBuilder::new()
                .with_load(0x1000, vec![0; 0x100])
                .build();
            let img = ElfImage::parse(&elf, None).unwrap();
            let offset = ElfImage::vaddr_to_offset(&img.program_headers, None, vaddr);
            assert_eq!(offset, vaddr);
        }

        #[test]
        fn relocation_roundtrip(offset in 0x400000u64..0x400100u64) {
            let rela = build_rela_entry(offset, R_X86_64_GLOB_DAT as u64, 0);
            let placeholder = build_dynamic_entries(&[(DT_RELA, 0), (DT_RELASZ, 0)]);
            let dyn_size = placeholder.len();
            let rela_vaddr = 0x1000u64 + dyn_size as u64;
            let dynamic = build_dynamic_entries(&[(DT_RELA, rela_vaddr), (DT_RELASZ, 24)]);
            let mut data = Vec::new();
            data.extend_from_slice(&dynamic);
            data.extend_from_slice(&rela);
            let elf = ElfBuilder::new()
                .with_load(0x1000, data)
                .with_dynamic(0x1000, dynamic.len() as u64)
                .build();
            let img = ElfImage::parse(&elf, None).unwrap();
            assert_eq!(img.relocations.len(), 1);
            assert_eq!(img.relocations[0].r_offset, offset);
            assert_eq!(img.relocations[0].r_type(), R_X86_64_GLOB_DAT);
        }

        #[test]
        fn strtab_resolve_roundtrip(name in "[a-z]{1,32}") {
            let strtab = build_strtab(&[name.as_bytes()]);
            let placeholder = build_dynamic_entries(&[(DT_STRTAB, 0), (DT_STRSZ, 0)]);
            let dyn_size = placeholder.len();
            let strtab_vaddr = 0x1000u64 + dyn_size as u64;
            let dynamic = build_dynamic_entries(&[
                (DT_STRTAB, strtab_vaddr),
                (DT_STRSZ, strtab.len() as u64),
            ]);
            let mut data = Vec::new();
            data.extend_from_slice(&dynamic);
            data.extend_from_slice(&strtab);
            let elf = ElfBuilder::new()
                .with_load(0x1000, data)
                .with_dynamic(0x1000, dynamic.len() as u64)
                .build();
            let img = ElfImage::parse(&elf, None).unwrap();
            assert_eq!(img.resolve_string(1), name.as_str());
        }

        #[test]
        fn multiple_load_segments_all_resolvable(n1 in 0x1000u64..0x8000u64, n2 in 0x10000u64..0x18000u64) {
            prop_assume!(n2 > n1 + 0x1000);
            let mut builder = ElfBuilder::new();
            builder.load_vaddr = n1;
            builder.load_offset = n1;
            builder.load_data = vec![0xAA; 0x100];
            builder.phdrs.push(PhdrDef {
                p_type: PT_LOAD,
                p_flags: PF_R | PF_X,
                p_offset: n1,
                p_vaddr: n1,
                p_filesz: 0x100,
                p_memsz: 0x100,
                p_align: 0x1000,
            });
            builder.phdrs.push(PhdrDef {
                p_type: PT_LOAD,
                p_flags: PF_R | PF_W,
                p_offset: n2,
                p_vaddr: n2,
                p_filesz: 0x100,
                p_memsz: 0x200,
                p_align: 0x1000,
            });
            let mut file = vec![0u8; (n2 as usize) + 0x100];
            file[0..4].copy_from_slice(&ELF_MAGIC);
            file[EI_CLASS] = ELFCLASS64;
            file[EI_DATA] = ELFDATA2LSB;
            file[EI_VERSION] = 1;
            let write_u16 = |data: &mut [u8], off: usize, v: u16| {
                data[off..off + 2].copy_from_slice(&v.to_le_bytes());
            };
            let write_u32 = |data: &mut [u8], off: usize, v: u32| {
                data[off..off + 4].copy_from_slice(&v.to_le_bytes());
            };
            let write_u64 = |data: &mut [u8], off: usize, v: u64| {
                data[off..off + 8].copy_from_slice(&v.to_le_bytes());
            };
            write_u16(&mut file, 16, ET_SCE_DYNAMIC);
            write_u16(&mut file, 18, EM_X86_64);
            write_u64(&mut file, 24, n1);
            write_u64(&mut file, 32, 64);
            write_u16(&mut file, 52, 64);
            write_u16(&mut file, 54, 56);
            write_u16(&mut file, 56, 2);
            let off1 = 64;
            let off2 = 64 + 56;
            write_u32(&mut file, off1, PT_LOAD);
            write_u32(&mut file, off1 + 4, PF_R | PF_X);
            write_u64(&mut file, off1 + 8, n1);
            write_u64(&mut file, off1 + 16, n1);
            write_u64(&mut file, off1 + 32, 0x100);
            write_u64(&mut file, off1 + 40, 0x100);
            write_u64(&mut file, off1 + 48, 0x1000);
            write_u32(&mut file, off2, PT_LOAD);
            write_u32(&mut file, off2 + 4, PF_R | PF_W);
            write_u64(&mut file, off2 + 8, n2);
            write_u64(&mut file, off2 + 16, n2);
            write_u64(&mut file, off2 + 32, 0x100);
            write_u64(&mut file, off2 + 40, 0x200);
            write_u64(&mut file, off2 + 48, 0x1000);
            file[n1 as usize..n1 as usize + 0x100].fill(0xAA);
            file[n2 as usize..n2 as usize + 0x100].fill(0xBB);
            let img = ElfImage::parse(&file, None).unwrap();
            assert_eq!(img.program_headers.len(), 2);
            let o1 = ElfImage::vaddr_to_offset(&img.program_headers, None, n1);
            assert_eq!(o1, n1);
            let o2 = ElfImage::vaddr_to_offset(&img.program_headers, None, n2);
            assert_eq!(o2, n2);
        }
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    if offset + 8 > data.len() { return 0; }
    u64::from_le_bytes([
        data[offset], data[offset+1], data[offset+2], data[offset+3],
        data[offset+4], data[offset+5], data[offset+6], data[offset+7],
    ])
}
