//! SELF/ELF parsing and program loading: magic detection, SELF segment
//! reconstruction, program-header mapping into guest memory and dynamic
//! module loading.

use goblin::container::{Container, Ctx, Endian};
use goblin::elf::Elf;
use goblin::elf::program_header::ProgramHeader as GoblinProgramHeader;
use std::fs;

use crate::error::{BleError, BleResult};
use ps5_memory_safe::{MemoryManager, MemoryProtection};

use super::loader::{
    DynamicSymbol, ElfSection, ElfSymbol, LoadedElf, ProgramHeader, RelocationEntry, page_up,
};

const SELF_MAGIC: [u8; 4] = [0x4f, 0x15, 0x3d, 0x1d];
const SELF_MAGIC_2: [u8; 4] = [0x54, 0x14, 0xf5, 0xee];

const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

const ELF_BASE_ADDRESS: u64 = 0x0000_0009_0000_0000;

struct ElfHeaderInfo {
    phoff: u64,
    phentsize: u16,
    phnum: u16,
}

fn parse_elf64_header(data: &[u8], offset: usize) -> BleResult<ElfHeaderInfo> {
    if data.len() < offset + 64 {
        return Err(BleError::Loader(
            "file too small for ELF header".to_string(),
        ));
    }

    Ok(ElfHeaderInfo {
        phoff: u64::from_le_bytes(data[offset + 0x20..offset + 0x28].try_into().unwrap()),
        phentsize: u16::from_le_bytes(data[offset + 0x36..offset + 0x38].try_into().unwrap()),
        phnum: u16::from_le_bytes(data[offset + 0x38..offset + 0x3a].try_into().unwrap()),
    })
}

#[derive(Debug, Clone)]
pub struct SelfSegment {
    pub type_: u64,
    pub offset: u64,
    pub compressed_size: u64,
    pub decompressed_size: u64,
}

pub struct ElfLoader;

impl ElfLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, path: &str, memory: &MemoryManager) -> BleResult<LoadedElf> {
        let data = fs::read(path)?;

        if data.len() < 16 {
            return Err(BleError::Loader("file too small".to_string()));
        }

        let is_self = Self::check_self_magic(&data);

        if is_self {
            log::info!(target: crate::log_targets::MODULE, "Detected SELF (Secure ELF) container format");
            self.load_self(&data, path, memory)
        } else if data[0..4] == ELF_MAGIC {
            log::info!(target: crate::log_targets::MODULE, "Detected raw ELF format");
            self.load_elf(&data, path, memory)
        } else {
            let magic_hex = data[..8]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            Err(BleError::Loader(format!(
                "unknown file format. First 8 bytes: 0x{}. Not a SELF or ELF container.",
                magic_hex
            )))
        }
    }

    fn check_self_magic(data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }

        let magic_match = data[0..4] == SELF_MAGIC || data[0..4] == SELF_MAGIC_2;
        if !magic_match {
            return false;
        }

        let ident_tail = &data[4..12];
        let known_tail_1: [u8; 8] = [0x00, 0x01, 0x01, 0x12, 0x01, 0x01, 0x00, 0x00];
        let known_tail_2: [u8; 8] = [0x10, 0x01, 0x01, 0x12, 0x01, 0x01, 0x00, 0x10];

        ident_tail == known_tail_1 || ident_tail == known_tail_2
    }

    fn load_self(&self, data: &[u8], path: &str, memory: &MemoryManager) -> BleResult<LoadedElf> {
        let image = self.reconstruct_self(data)?;
        self.load_elf_image(&image, path, memory, true)
    }

    /// Parse the SELF container and return the flat ELF image it wraps.
    fn reconstruct_self(&self, data: &[u8]) -> BleResult<Vec<u8>> {
        let (file_size, segments_num, hdr_size) = Self::parse_self_header(data)?;
        let seg_table_size = segments_num as usize * 32;
        let elf_offset = hdr_size + seg_table_size;

        let segments = Self::parse_self_segments(data, hdr_size, segments_num)?;

        log::info!(target: crate::log_targets::MODULE, "SELF header parsed:");
        log::info!(target: crate::log_targets::MODULE, "  file_size: 0x{:x}", file_size);
        log::info!(target: crate::log_targets::MODULE, "  segments: {}", segments_num);
        log::info!(target: crate::log_targets::MODULE, "  ELF offset: 0x{:x}", elf_offset);

        for (i, seg) in segments.iter().enumerate() {
            log::info!(target: crate::log_targets::MODULE,
                "  segment[{}]: offset=0x{:x}, compressed=0x{:x}, decompressed=0x{:x}, type=0x{:x}",
                i, seg.offset, seg.compressed_size, seg.decompressed_size, seg.type_
            );
        }

        if data.len() < elf_offset + 64 {
            return Err(BleError::Loader(
                "file too small for ELF header".to_string(),
            ));
        }

        if data[elf_offset..elf_offset + 4] != ELF_MAGIC {
            return Err(BleError::Loader(format!(
                "ELF magic not found at expected offset 0x{:x} in SELF container",
                elf_offset
            )));
        }

        let ctx = Ctx::new(Container::Big, Endian::Little);
        let ehdr = parse_elf64_header(data, elf_offset)?;

        if ehdr.phoff == 0 || ehdr.phnum == 0 {
            return Err(BleError::Loader(
                "SELF: ELF has no program headers".to_string(),
            ));
        }

        let phdrs = GoblinProgramHeader::parse(
            data,
            elf_offset + ehdr.phoff as usize,
            ehdr.phnum as usize,
            ctx,
        )
        .map_err(|e| BleError::Loader(format!("SELF: cannot parse program headers: {}", e)))?;

        log::info!(target: crate::log_targets::MODULE, "  ELF program headers: {} (phentsize={})", phdrs.len(), ehdr.phentsize);

        let image = Self::reconstruct_self_image(data, elf_offset, &ehdr, &phdrs, &segments)?;
        log::info!(target: crate::log_targets::MODULE, "Reconstructed ELF image ({} bytes)", image.len());
        Ok(image)
    }

    fn parse_self_header(data: &[u8]) -> BleResult<(u64, u16, usize)> {
        let hdr_size = 32;
        if data.len() < hdr_size {
            return Err(BleError::Loader(
                "file too small for SELF header".to_string(),
            ));
        }

        let file_size = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let segments_num = u16::from_le_bytes(data[24..26].try_into().unwrap());

        Ok((file_size, segments_num, hdr_size))
    }

    fn parse_self_segments(
        data: &[u8],
        offset: usize,
        segments_num: u16,
    ) -> BleResult<Vec<SelfSegment>> {
        let seg_size = 32;
        let total_size = seg_size * segments_num as usize;

        if data.len() < offset + total_size {
            return Err(BleError::Loader(
                "file too small for SELF segments".to_string(),
            ));
        }

        let mut segments = Vec::new();
        for i in 0..segments_num as usize {
            let off = offset + i * seg_size;
            let type_ = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
            let seg_offset = u64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap());
            let compressed_size = u64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap());
            let decompressed_size =
                u64::from_le_bytes(data[off + 24..off + 32].try_into().unwrap());

            segments.push(SelfSegment {
                type_,
                offset: seg_offset,
                compressed_size,
                decompressed_size,
            });
        }

        Ok(segments)
    }

    /// Rebuilds the flat ELF image that the SELF segments describe.
    ///
    /// The ELF program headers inside a SELF use offsets relative to the image
    /// start (virtual layout), while the actual data is scattered across the
    /// SELF segment table at file offsets. Each SELF data segment encodes its
    /// target program header index in the high bits of its type field.
    fn reconstruct_self_image(
        data: &[u8],
        elf_offset: usize,
        ehdr: &ElfHeaderInfo,
        phdrs: &[GoblinProgramHeader],
        segments: &[SelfSegment],
    ) -> BleResult<Vec<u8>> {
        let phdr_table_end = ehdr.phoff as usize + ehdr.phnum as usize * ehdr.phentsize as usize;

        let mut image_size = phdr_table_end.max(64);
        for phdr in phdrs {
            if phdr.p_filesz > 0 {
                image_size = image_size.max(phdr.p_offset as usize + phdr.p_filesz as usize);
            }
        }

        let mut image = vec![0u8; image_size];

        let header_region = phdr_table_end.min(data.len().saturating_sub(elf_offset));
        image[..header_region].copy_from_slice(&data[elf_offset..elf_offset + header_region]);

        let mut placed = 0usize;
        for seg in segments {
            if (seg.type_ & 0x800) == 0 {
                continue;
            }

            let phdr_id = ((seg.type_ >> 20) & 0xFFF) as usize;
            let Some(phdr) = phdrs.get(phdr_id) else {
                log::warn!(target: crate::log_targets::MODULE, "SELF segment type=0x{:x} references missing phdr {}", seg.type_, phdr_id);
                continue;
            };

            if seg.decompressed_size != phdr.p_filesz {
                log::warn!(target: crate::log_targets::MODULE,
                    "SELF segment type=0x{:x} size 0x{:x} != phdr[{}].filesz 0x{:x}",
                    seg.type_, seg.decompressed_size, phdr_id, phdr.p_filesz
                );
            }

            let src_start = seg.offset as usize;
            let src_end = src_start + seg.decompressed_size as usize;
            let dst_start = phdr.p_offset as usize;
            let dst_end = dst_start + phdr.p_filesz as usize;

            if src_end > data.len() || dst_end > image.len() {
                return Err(BleError::Loader(format!(
                    "SELF segment {} out of bounds (src {:#x}..{:#x}, dst {:#x}..{:#x})",
                    phdr_id, src_start, src_end, dst_start, dst_end
                )));
            }

            image[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            placed += 1;
        }
        log::info!(target: crate::log_targets::MODULE, "Placed {} SELF data segments into ELF image", placed);

        // PS5 eboots ship without a section header table; e_shoff/e_shnum are
        // stale garbage that would make the parser read out of bounds. Neutralize
        // them (e_shnum=1 with a harmless index skips goblin's extended-count path).
        image[0x28..0x30].copy_from_slice(&0u64.to_le_bytes()); // e_shoff = 0
        image[0x3c..0x3e].copy_from_slice(&1u16.to_le_bytes()); // e_shnum = 1
        image[0x3e..0x40].copy_from_slice(&1u16.to_le_bytes()); // e_shstrndx = 1

        Ok(image)
    }

    fn load_elf(&self, data: &[u8], path: &str, memory: &MemoryManager) -> BleResult<LoadedElf> {
        self.load_elf_image(data, path, memory, false)
    }

    fn load_elf_image(
        &self,
        elf_data: &[u8],
        path: &str,
        memory: &MemoryManager,
        is_self: bool,
    ) -> BleResult<LoadedElf> {
        self.load_elf_image_at(elf_data, path, memory, is_self, ELF_BASE_ADDRESS)
    }

    fn load_elf_image_at(
        &self,
        elf_data: &[u8],
        path: &str,
        memory: &MemoryManager,
        is_self: bool,
        base_address: u64,
    ) -> BleResult<LoadedElf> {
        let elf = Elf::parse(elf_data)
            .map_err(|e| BleError::Loader(format!("ELF parse error: {}", e)))?;
        self.build_loaded_elf(&elf, elf_data, path, memory, is_self, base_address, true)
    }

    /// Load a dynamic module (PRX/lib) into a fresh allocated region instead of
    /// the fixed eboot base. Mirrors Kyty's LoadProgram for module files.
    pub fn load_module(&self, path: &str, memory: &MemoryManager) -> BleResult<LoadedElf> {
        let data = fs::read(path)?;

        if data.len() < 16 {
            return Err(BleError::Loader("file too small".to_string()));
        }

        let is_self = Self::check_self_magic(&data);
        let elf_data = if is_self {
            log::info!(target: crate::log_targets::MODULE, "Detected SELF container (module)");
            self.reconstruct_self(&data)?
        } else if data[0..4] == ELF_MAGIC {
            data
        } else {
            return Err(BleError::Loader(format!(
                "unknown file format for module: {}",
                path
            )));
        };

        let elf = Elf::parse(&elf_data)
            .map_err(|e| BleError::Loader(format!("ELF parse error: {}", e)))?;

        let mut prog_end = 0u64;
        for phdr in &elf.program_headers {
            if phdr.p_type == goblin::elf::program_header::PT_LOAD && phdr.p_memsz > 0 {
                prog_end = prog_end.max(phdr.p_vaddr + phdr.p_memsz);
            }
        }
        if prog_end == 0 {
            return Err(BleError::Loader(format!(
                "module has no loadable PT_LOAD segments: {}",
                path
            )));
        }

        let region = memory.map_host_memory(
            None,
            page_up(prog_end),
            0x1000,
            MemoryProtection::READ_WRITE,
            "module",
        )?;
        log::info!(target: crate::log_targets::MODULE,
            "Module '{}' base: 0x{:016x} (0x{:x} bytes)",
            path,
            region.address,
            region.size
        );

        self.build_loaded_elf(
            &elf,
            &elf_data,
            path,
            memory,
            is_self,
            region.address,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_loaded_elf(
        &self,
        elf: &Elf,
        elf_data: &[u8],
        path: &str,
        memory: &MemoryManager,
        is_self: bool,
        base_address: u64,
        reserve: bool,
    ) -> BleResult<LoadedElf> {
        let entry_point = base_address + elf.entry;

        let mut sections = Vec::new();
        for (idx, section) in elf.section_headers.iter().enumerate() {
            let section_data = if section.sh_addr != 0
                && section.sh_size > 0
                && section.sh_type != goblin::elf::section_header::SHT_NOBITS
            {
                let sec_offset = section.sh_offset as usize;
                let sec_size = section.sh_size as usize;
                elf_data
                    .get(sec_offset..sec_offset + sec_size)
                    .unwrap_or(&[])
                    .to_vec()
            } else {
                Vec::new()
            };

            let name = elf.shdr_strtab.get_at(idx).unwrap_or("").to_string();

            sections.push(ElfSection {
                name,
                address: section.sh_addr,
                size: section.sh_size,
                flags: section.sh_flags,
                data: section_data,
            });
        }

        let mut symbols = Vec::new();
        for sym in &elf.syms {
            symbols.push(ElfSymbol {
                name: elf.strtab.get_at(sym.st_name).unwrap_or("").to_string(),
                address: sym.st_value,
                size: sym.st_size,
                binding: sym.st_bind(),
                sym_type: sym.st_type(),
            });
        }

        let dynamic_linking = !elf.libraries.is_empty();
        let soname = String::new();
        let needed_libs = elf.libraries.iter().map(|s| s.to_string()).collect();

        let mut dynamic_symbols = Vec::new();
        for sym in elf.dynsyms.to_vec() {
            let name = elf.dynstrtab.get_at(sym.st_name).unwrap_or("").to_string();
            dynamic_symbols.push(DynamicSymbol {
                name,
                value: sym.st_value,
                size: sym.st_size,
                defined: sym.st_shndx != 0,
                bind: sym.st_bind(),
                sym_type: sym.st_type(),
            });
        }

        let mut relocations = Vec::new();
        for r in elf.dynrelas.to_vec() {
            relocations.push(RelocationEntry {
                r_type: r.r_type,
                r_offset: r.r_offset,
                r_addend: r.r_addend.unwrap_or(0),
                symbol: if r.r_sym != 0 { Some(r.r_sym) } else { None },
            });
        }
        for r in elf.dynrels.to_vec() {
            relocations.push(RelocationEntry {
                r_type: r.r_type,
                r_offset: r.r_offset,
                r_addend: 0,
                symbol: if r.r_sym != 0 { Some(r.r_sym) } else { None },
            });
        }
        let rela_count = relocations.len();
        for r in elf.pltrelocs.to_vec() {
            relocations.push(RelocationEntry {
                r_type: r.r_type,
                r_offset: r.r_offset,
                r_addend: r.r_addend.unwrap_or(0),
                symbol: if r.r_sym != 0 { Some(r.r_sym) } else { None },
            });
        }
        let jmprela_count = relocations.len() - rela_count;

        // DT_INIT / DT_FINI as raw vaddrs relative to base. goblin's
        // DynamicInfo converts these to file offsets (vm_to_offset), so read
        // the raw d_tag/d_val pairs directly like Kyty's dynamic_info does.
        let mut init_vaddr = 0u64;
        let mut fini_vaddr = 0u64;
        if let Some(dyn_) = &elf.dynamic {
            for e in &dyn_.dyns {
                if e.d_tag == goblin::elf::dynamic::DT_INIT {
                    init_vaddr = e.d_val;
                } else if e.d_tag == goblin::elf::dynamic::DT_FINI {
                    fini_vaddr = e.d_val;
                }
            }
        }

        let mut program_headers = Vec::new();
        for phdr in &elf.program_headers {
            program_headers.push(ProgramHeader {
                ptype: phdr.p_type,
                flags: phdr.p_flags,
                offset: phdr.p_offset,
                vaddr: phdr.p_vaddr,
                paddr: phdr.p_paddr,
                filesz: phdr.p_filesz,
                memsz: phdr.p_memsz,
                align: phdr.p_align,
            });
        }

        self.load_program_headers(&elf, elf_data, base_address, memory, reserve)?;

        Ok(LoadedElf {
            entry_point,
            base_address,
            size: elf_data.len() as u64,
            init_vaddr,
            fini_vaddr,
            sections,
            symbols,
            dynamic_linking,
            soname,
            needed_libs,
            program_headers,
            eboot_path: path.to_string(),
            is_self,
            dynamic_symbols,
            relocations,
            rela_count,
            jmprela_count,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn load_program_headers(
        &self,
        elf: &Elf,
        elf_data: &[u8],
        base_address: u64,
        memory: &MemoryManager,
        reserve: bool,
    ) -> BleResult<()> {
        // Collect the PT_LOAD segments first so the whole program span can be
        // reserved up front and committed as merged page-aligned ranges. This
        // avoids 64KB reservation-granularity collisions between adjacent
        // segments on Windows.
        let mut loads: Vec<(u64, u64, u64, u64, u32)> = Vec::new(); // vaddr, memsz, filesz, offset, flags
        for phdr in &elf.program_headers {
            if phdr.p_type != goblin::elf::program_header::PT_LOAD {
                continue;
            }
            if phdr.p_memsz == 0 {
                continue;
            }
            loads.push((
                phdr.p_vaddr,
                phdr.p_memsz,
                phdr.p_filesz,
                phdr.p_offset,
                phdr.p_flags,
            ));
        }
        if loads.is_empty() {
            return Ok(());
        }

        let prog_end = loads.iter().map(|l| l.0 + l.1).max().unwrap();
        if reserve {
            memory.map_program_image(base_address, prog_end)?;
        }

        // Commit the ENTIRE reserved program span (including inter-segment
        // alignment gaps and BSS), then write file-backed bytes and apply the
        // final per-segment protections. Games routinely write across segment
        // boundaries at startup (self-relocation, runtime init), so leaving
        // holes uncommitted crashes them.
        memory.commit_range(base_address, prog_end)?;

        for (vaddr, _memsz, filesz, offset, _flags) in &loads {
            if *filesz == 0 {
                continue;
            }
            let src = *offset as usize;
            let seg = elf_data.get(src..src + *filesz as usize).unwrap_or(&[]);
            if !seg.is_empty() {
                memory.write(base_address + vaddr, seg)?;
            }
        }

        for (vaddr, memsz, _filesz, _offset, flags) in &loads {
            if *flags == 0 {
                // PS5 (next-gen) eboots ship PT_LOAD segments with p_flags=0;
                // the kernel maps those with the default RW protection rather
                // than NoAccess, so leave the commit-time protection intact.
                // Games write across the boundary into these segments at
                // startup (runtime data init), and applying READ-only here
                // faults those copies (kyty: skip_protect for next-gen).
                continue;
            }
            // Simplified protection: grant read/write/execute to all PT_LOAD segments
            let final_prot = MemoryProtection::READ_WRITE_EXECUTE;
            memory.protect(base_address + vaddr, *memsz, final_prot)?;
        }
        Ok(())
    }
}

impl Default for ElfLoader {
    fn default() -> Self {
        Self::new()
    }
}
