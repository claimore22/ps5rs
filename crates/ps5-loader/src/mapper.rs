use crate::memory::{MemoryRegion, ProcessMemory, SegmentFlags};
use crate::relocation::{RelocationRecord, RelocationSummary};
/// Whether a module is the main executable (EBOOT) or a shared library (PRX).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    Eboot,
    Prx,
}

/// How the module name was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleNameSource {
    SceModuleInfo,
    Filename,
    Unknown,
}

/// Lifecycle state of a loaded module during the multi-phase load pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// PT_LOAD segments mapped, no relocations applied.
    Mapped,
    /// RELATIVE relocations applied.  Imports not yet resolved.
    Relocated,
    /// Exports registered and imports resolved (GLOB_DAT/JUMP_SLOT patched).
    Linked,
    /// Module init routines have been called (PREINIT_ARRAY, INIT, etc.)
    Initialized,
}

/// A single import binding: an (optional) resolved address for an import symbol.
#[derive(Debug, Clone)]
pub struct ImportBinding {
    pub library: String,
    pub nid: u64,
    pub name: Option<String>,
    pub address: Option<u64>,
}

/// A loaded PS5 module (EBOOT or PRX) with mapped memory, relocations, and imports.
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// Module name (e.g. "eboot.bin" or "libScePad.prx").
    pub name: String,
    /// How `name` was determined.
    pub name_source: ModuleNameSource,
    /// Whether this is the main executable or a shared library.
    pub module_type: ModuleType,
    /// Mapped virtual address space with segment data.
    pub memory: ProcessMemory,
    /// Original virtual address of the first `PT_LOAD` segment.
    pub preferred_base: u64,
    /// Difference between runtime base and `preferred_base`.
    pub load_bias: u64,
    /// Module entry point (`e_entry`), if non-zero.
    pub entry_point: Option<u64>,
    /// Import bindings (library + NID per imported symbol).
    pub imports: Vec<ImportBinding>,
    /// Relocation records produced by [`apply_relocations`](crate::apply_relocations).
    pub relocations: Vec<RelocationRecord>,
    /// Summary of the last relocation pass.
    pub relocation_summary: Option<RelocationSummary>,
    /// DT_SONAME of the module, if present.
    pub soname: Option<String>,
    /// Alternative names (filename, SONAME, stripped extensions).
    pub aliases: Vec<String>,
    /// Current lifecycle phase.
    pub state: ModuleState,
    /// Number of exported symbols registered from this module.
    pub exports_count: usize,
    /// Number of imports resolved against the export table.
    pub imports_resolved: u32,
    /// Number of imports known via offline export table.
    pub imports_known: u32,
    /// Number of imports assigned stub addresses.
    pub imports_stubbed: u32,
}

impl LoadedModule {
    /// Return the canonical identity: `soname` if set, otherwise `name`.
    pub fn canonical_name(&self) -> &str {
        self.soname.as_deref().unwrap_or(&self.name)
    }
}

/// Container that will hold multiple loaded modules (EBOOT + chain of PRX deps).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LoadedProcess {
    pub modules: Vec<LoadedModule>,
}

#[derive(Debug)]
pub struct LoaderError(pub String);

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LoaderError {}

impl From<crate::relocation::RelocationError> for LoaderError {
    fn from(e: crate::relocation::RelocationError) -> Self {
        LoaderError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LoaderError>;

/// Parse an ELF binary and build a [`LoadedModule`] with mapped memory regions.
///
/// This is the first phase of loading.  It creates a [`ProcessMemory`] from the
/// `PT_LOAD` segments, zero-fills `.bss`, and determines the module type and
/// entry point.  Relocation is **not** applied — call
/// [`apply_relocations`](crate::apply_relocations) separately.
pub fn load_elf(name: &str, elf_bytes: &[u8]) -> Result<LoadedModule> {
    tracing::info!(name, size = elf_bytes.len(), "load_elf: start");

    let image = ps5_elf::ElfImage::parse(elf_bytes, None)
        .map_err(|e| LoaderError(format!("ELF parse failed: {e}")))?;

    let module_type = if image.header.is_shared() {
        ModuleType::Prx
    } else {
        ModuleType::Eboot
    };

    let mut regions: Vec<MemoryRegion> = Vec::new();

    for ph in &image.program_headers {
        if !ph.is_load() {
            continue;
        }

        let filesz = ph.p_filesz as usize;
        let memsz = ph.p_memsz as usize;
        let seg_start = ph.p_offset as usize;
        let seg_end = elf_bytes.len().min(seg_start.saturating_add(filesz));
        let copy_len = seg_end.saturating_sub(seg_start);

        let mut data = vec![0u8; memsz];
        if copy_len > 0 {
            let write_len = copy_len.min(memsz);
            data[..write_len].copy_from_slice(&elf_bytes[seg_start..seg_start + write_len]);
        }

        regions.push(MemoryRegion {
            vaddr: ph.p_vaddr,
            size: memsz,
            file_offset: ph.p_offset,
            file_size: filesz,
            permissions: SegmentFlags::from_p_flags(ph.p_flags),
            data,
        });
    }

    regions.sort_by_key(|r| r.vaddr);

    let preferred_base = regions
        .iter()
        .map(|r| r.vaddr)
        .min()
        .unwrap_or(0);
    let entry_point = if image.header.e_entry > 0 {
        Some(image.header.e_entry)
    } else {
        None
    };

    tracing::debug!(
        module_type = ?module_type,
        region_count = regions.len(),
        preferred_base,
        entry = ?entry_point,
        "load_elf: mapped {} segments",
        regions.len(),
    );

    let module_name = name.to_string();
    let soname = image.soname.clone();

    let mut aliases = Vec::new();
    aliases.push(module_name.clone());
    if let Some(ref sn) = soname {
        if sn != &module_name {
            aliases.push(sn.clone());
        }
    }
    if let Some(stripped) = module_name.strip_suffix(".prx") {
        if !aliases.contains(&stripped.to_string()) {
            aliases.push(stripped.to_string());
        }
    }

    Ok(LoadedModule {
        name: module_name,
        name_source: ModuleNameSource::Filename,
        module_type,
        memory: ProcessMemory::new(regions),
        preferred_base,
        load_bias: 0,
        entry_point,
        imports: Vec::new(),
        relocations: Vec::new(),
        relocation_summary: None,
        soname,
        aliases,
        state: ModuleState::Mapped,
        exports_count: 0,
        imports_resolved: 0,
        imports_known: 0,
        imports_stubbed: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
    const EI_CLASS: usize = 4;
    const EI_DATA: usize = 5;
    const EI_VERSION: usize = 6;
    const ELFCLASS64: u8 = 2;
    const ELFDATA2LSB: u8 = 1;
    const EM_X86_64: u16 = 62;
    const ET_SCE_DYNEXEC: u16 = 0xFE10;
    const ET_SCE_DYNAMIC: u16 = 0xFE18;
    const PT_LOAD: u32 = 1;
    const PF_R: u32 = 4;
    const PF_W: u32 = 2;
    const PF_X: u32 = 1;

    fn put_u16(buf: &mut [u8], off: usize, val: u16) {
        buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
    }

    fn put_u32(buf: &mut [u8], off: usize, val: u32) {
        buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    fn put_u64(buf: &mut [u8], off: usize, val: u64) {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }

    fn build_elf(e_type: u16, entry: u64, phdrs: &[(u32, u32, u64, u64, u64, u64)], payloads: &[&[u8]]) -> Vec<u8> {
        assert_eq!(phdrs.len(), payloads.len());

        let phdr_count = phdrs.len() as u16;
        let phdr_size = 56;
        let phdr_offset: usize = 64;
        let phdr_end: usize = phdr_offset + phdr_count as usize * phdr_size;

        let mut max_end = phdr_end;
        for &(_, _, p_offset, _, p_filesz, _) in phdrs {
            let end = (p_offset as usize).saturating_add(p_filesz as usize);
            if end > max_end {
                max_end = end;
            }
        }

        let mut buf = vec![0u8; max_end];

        buf[0..4].copy_from_slice(&ELF_MAGIC);
        buf[EI_CLASS] = ELFCLASS64;
        buf[EI_DATA] = ELFDATA2LSB;
        buf[EI_VERSION] = 1;
        put_u16(&mut buf, 16, e_type);
        put_u16(&mut buf, 18, EM_X86_64);
        put_u32(&mut buf, 20, 1);
        put_u64(&mut buf, 24, entry);
        put_u64(&mut buf, 32, phdr_offset as u64);
        put_u64(&mut buf, 40, 0);
        put_u32(&mut buf, 48, 0);
        put_u16(&mut buf, 52, 64);
        put_u16(&mut buf, 54, phdr_size as u16);
        put_u16(&mut buf, 56, phdr_count);

        for (i, &(p_type, p_flags, p_offset, p_vaddr, p_filesz, p_memsz)) in phdrs.iter().enumerate() {
            let off = phdr_offset + i * phdr_size;
            put_u32(&mut buf, off, p_type);
            put_u32(&mut buf, off + 4, p_flags);
            put_u64(&mut buf, off + 8, p_offset);
            put_u64(&mut buf, off + 16, p_vaddr);
            put_u64(&mut buf, off + 24, p_vaddr);
            put_u64(&mut buf, off + 32, p_filesz);
            put_u64(&mut buf, off + 40, p_memsz);
            put_u64(&mut buf, off + 48, 0x1000);

            let dst = p_offset as usize;
            let len = p_filesz as usize;
            let copy_len = len.min(payloads[i].len()).min(buf.len().saturating_sub(dst));
            if copy_len > 0 {
                buf[dst..dst + copy_len].copy_from_slice(&payloads[i][..copy_len]);
            }
        }

        buf
    }

    #[test]
    fn load_single_rx_segment() {
        let payload = vec![0xCC; 0x200];
        let elf = build_elf(
            ET_SCE_DYNEXEC,
            0x800001000,
            &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x200, 0x200)],
            &[&payload],
        );

        let module = load_elf("eboot.bin", &elf).unwrap();
        assert_eq!(module.name, "eboot.bin");
        assert_eq!(module.module_type, ModuleType::Eboot);
        assert_eq!(module.preferred_base, 0x800000000);
        assert_eq!(module.entry_point, Some(0x800001000));
        assert!(module.relocations.is_empty());
        assert!(module.relocation_summary.is_none());

        assert_eq!(module.memory.regions.len(), 1);
        let r = &module.memory.regions[0];
        assert_eq!(r.vaddr, 0x800000000);
        assert_eq!(r.size, 0x200);
        assert_eq!(r.permissions, SegmentFlags::from_p_flags(PF_R | PF_X));
        assert_eq!(r.data, payload);
    }

    #[test]
    fn load_two_segments_rx_rw() {
        let code = vec![0xCC; 0x200];
        let data = vec![0xDD; 0x100];
        let elf = build_elf(
            ET_SCE_DYNEXEC,
            0x800001000,
            &[
                (PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x200, 0x200),
                (PT_LOAD, PF_R | PF_W, 0x1200, 0x800100000, 0x100, 0x100),
            ],
            &[&code, &data],
        );

        let module = load_elf("eboot.bin", &elf).unwrap();
        assert_eq!(module.memory.regions.len(), 2);

        let r0 = &module.memory.regions[0];
        assert_eq!(r0.vaddr, 0x800000000);
        assert_eq!(r0.permissions, SegmentFlags::from_p_flags(PF_R | PF_X));
        assert_eq!(r0.data, code);

        let r1 = &module.memory.regions[1];
        assert_eq!(r1.vaddr, 0x800100000);
        assert_eq!(r1.permissions, SegmentFlags::from_p_flags(PF_R | PF_W));
        assert_eq!(r1.data, data);
    }

    #[test]
    fn load_bss_zero_filled() {
        let payload = vec![0xBB; 0x100];
        let elf = build_elf(
            ET_SCE_DYNEXEC,
            0x800001000,
            &[(PT_LOAD, PF_R | PF_W, 0x1000, 0x800000000, 0x100, 0x300)],
            &[&payload],
        );

        let module = load_elf("eboot.bin", &elf).unwrap();
        let r = &module.memory.regions[0];
        assert_eq!(r.size, 0x300);
        assert_eq!(r.file_size, 0x100);
        assert_eq!(&r.data[..0x100], &payload[..]);
        assert_eq!(&r.data[0x100..0x300], &[0u8; 0x200]);
    }

    #[test]
    fn load_shared_elf_is_prx() {
        let elf = build_elf(
            ET_SCE_DYNAMIC,
            0,
            &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x100, 0x100)],
            &[&[0xAA; 0x100]],
        );

        let module = load_elf("libSceTest.prx", &elf).unwrap();
        assert_eq!(module.module_type, ModuleType::Prx);
    }

    #[test]
    fn entry_zero_when_no_entry() {
        let elf = build_elf(
            ET_SCE_DYNAMIC,
            0,
            &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x100, 0x100)],
            &[&[0xAA; 0x100]],
        );

        let module = load_elf("test.prx", &elf).unwrap();
        assert_eq!(module.entry_point, None);
    }

    #[test]
    fn load_elf_with_multiple_payload_offsets() {
        let code = vec![0x11; 0x100];
        let rodata = vec![0x22; 0x80];
        let data = vec![0x33; 0x40];

        let phdrs = [
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x100, 0x100),
            (PT_LOAD, PF_R, 0x2000, 0x800100000, 0x80, 0x80),
            (PT_LOAD, PF_R | PF_W, 0x3000, 0x800200000, 0x40, 0x100),
        ];

        let elf = build_elf(
            ET_SCE_DYNEXEC,
            0x800001000,
            &phdrs,
            &[&code, &rodata, &data],
        );

        let module = load_elf("eboot.bin", &elf).unwrap();
        assert_eq!(module.memory.regions.len(), 3);

        assert_eq!(module.memory.regions[0].vaddr, 0x800000000);
        assert_eq!(module.memory.regions[0].data, code);

        assert_eq!(module.memory.regions[1].vaddr, 0x800100000);
        assert_eq!(module.memory.regions[1].data, rodata);

        assert_eq!(module.memory.regions[2].vaddr, 0x800200000);
        assert_eq!(&module.memory.regions[2].data[..0x40], &data[..]);
        assert_eq!(&module.memory.regions[2].data[0x40..0x100], &[0u8; 0xC0]);
    }

    #[test]
    fn preferred_base_is_min_vaddr() {
        let elf = build_elf(
            ET_SCE_DYNEXEC,
            0x800001000,
            &[
                (PT_LOAD, PF_R | PF_X, 0x1000, 0x800000000, 0x100, 0x100),
                (PT_LOAD, PF_R | PF_W, 0x1000, 0x800200000, 0x100, 0x100),
            ],
            &[&[0xAA; 0x100], &[0xBB; 0x100]],
        );

        let module = load_elf("eboot.bin", &elf).unwrap();
        assert_eq!(module.preferred_base, 0x800000000);
    }

    #[test]
    fn truncated_payload_zeros_remainder() {
        let buf = {
            let mut b = vec![0u8; 0x1100];
            b[0..4].copy_from_slice(&ELF_MAGIC);
            b[EI_CLASS] = ELFCLASS64;
            b[EI_DATA] = ELFDATA2LSB;
            b[EI_VERSION] = 1;
            put_u16(&mut b, 16, ET_SCE_DYNEXEC);
            put_u16(&mut b, 18, EM_X86_64);
            put_u32(&mut b, 20, 1);
            put_u64(&mut b, 24, 0x800001000);
            put_u64(&mut b, 32, 64);
            put_u16(&mut b, 52, 64);
            put_u16(&mut b, 54, 56);
            put_u16(&mut b, 56, 1);

            put_u32(&mut b, 64, PT_LOAD);
            put_u32(&mut b, 68, PF_R | PF_X);
            put_u64(&mut b, 72, 0x1000);
            put_u64(&mut b, 80, 0x800000000);
            put_u64(&mut b, 88, 0x800000000);
            put_u64(&mut b, 96, 0x400);
            put_u64(&mut b, 104, 0x400);
            put_u64(&mut b, 112, 0x1000);

            // Only write 0x100 of payload data instead of 0x400
            b[0x1000..0x1100].fill(0xEE);
            b
        };

        let module = load_elf("truncated.elf", &buf).unwrap();
        let r = &module.memory.regions[0];
        assert_eq!(r.size, 0x400);
        assert_eq!(&r.data[..0x100], &[0xEE; 0x100]);
        assert_eq!(&r.data[0x100..0x400], &[0u8; 0x300]);
    }

    #[test]
    fn invalid_elf_returns_error() {
        let err = load_elf("bad.elf", &[0xFF; 64]).unwrap_err();
        assert!(err.to_string().contains("invalid magic"));
    }

    #[test]
    fn not_x86_64_returns_error() {
        let mut buf = vec![0u8; 64];
        buf[0..4].copy_from_slice(&ELF_MAGIC);
        buf[EI_CLASS] = ELFCLASS64;
        buf[EI_DATA] = ELFDATA2LSB;
        buf[EI_VERSION] = 1;
        put_u16(&mut buf, 18, 0x28);
        let err = load_elf("bad_arch.elf", &buf).unwrap_err();
        assert!(err.to_string().contains("not x86-64"));
    }
}
