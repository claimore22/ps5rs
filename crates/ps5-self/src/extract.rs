use ps5_format::elf_constants::*;
use ps5_format::error::{ParseError, Result};
use crate::SelfImage;

pub struct ExtractResult {
    pub elf: Vec<u8>,
    pub was_self: bool,
    pub encrypted_segments: usize,
    pub compressed_segments: usize,
}

pub fn extract_elf(data: &[u8]) -> Result<ExtractResult> {
    let img = SelfImage::parse(data)?;

    let mut encrypted_segments = 0usize;
    let mut compressed_segments = 0usize;
    for seg in &img.segments {
        if seg.is_encrypted() { encrypted_segments += 1; }
        if seg.is_compressed() { compressed_segments += 1; }
    }

    if !img.is_self() {
        return Ok(ExtractResult {
            elf: data.to_vec(),
            was_self: false,
            encrypted_segments: 0,
            compressed_segments: 0,
        });
    }

    let elf_base = 32 + img.self_header.num_segments as usize * 32;
    let elf_slice = &data[elf_base..];

    let phdr_count = img.elf.program_headers.len();

    let mut phdr_file_offsets = vec![0u64; phdr_count];
    for seg in &img.segments {
        if !seg.is_data() { continue; }
        let idx = seg.phdr_index() as usize;
        if idx < phdr_count {
            phdr_file_offsets[idx] = seg.file_offset.saturating_sub(elf_base as u64);
        }
    }

    let mut max_offset = 0u64;
    for ph in &img.elf.program_headers {
        if ph.p_filesz > 0 {
            let end = ph.p_offset.saturating_add(ph.p_filesz);
            if end > max_offset {
                max_offset = end;
            }
        }
    }

    if max_offset == 0 {
        return Err(ParseError::Custom("no loadable segments found".into()));
    }

    let mut output = vec![0u8; max_offset as usize];

    let copy_len = 64.min(elf_slice.len());
    output[0..copy_len].copy_from_slice(&elf_slice[..copy_len]);

    let phoff = img.elf.header.e_phoff as usize;
    let phdr_table_size = phdr_count * img.elf.header.phentsize as usize;
    if phoff + phdr_table_size <= elf_slice.len() && phoff + phdr_table_size <= output.len() {
        output[phoff..phoff + phdr_table_size]
            .copy_from_slice(&elf_slice[phoff..phoff + phdr_table_size]);
    }

    for (i, ph) in img.elf.program_headers.iter().enumerate() {
        if ph.p_filesz == 0 || ph.p_type == PT_NULL {
            continue;
        }

        let src_offset = phdr_file_offsets[i] as usize;
        let dst_offset = ph.p_offset as usize;
        let size = ph.p_filesz as usize;

        if src_offset + size <= elf_slice.len() && dst_offset + size <= output.len() {
            output[dst_offset..dst_offset + size]
                .copy_from_slice(&elf_slice[src_offset..src_offset + size]);
        }
    }

    let _ = ps5_elf::ElfHeader::parse(&output, 0)
        .map_err(|e| ParseError::Custom(format!("extracted ELF validation failed: {e}")))?;

    Ok(ExtractResult {
        elf: output,
        was_self: true,
        encrypted_segments,
        compressed_segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_format::self_constants::*;

    fn wu16(buf: &mut Vec<u8>, v: u16) { buf.extend_from_slice(&v.to_le_bytes()); }
    fn wu32(buf: &mut Vec<u8>, v: u32) { buf.extend_from_slice(&v.to_le_bytes()); }
    fn wu64(buf: &mut Vec<u8>, v: u64) { buf.extend_from_slice(&v.to_le_bytes()); }
    fn wu16_at(buf: &mut [u8], off: usize, v: u16) { buf[off..off+2].copy_from_slice(&v.to_le_bytes()); }
    fn wu32_at(buf: &mut [u8], off: usize, v: u32) { buf[off..off+4].copy_from_slice(&v.to_le_bytes()); }
    fn wu64_at(buf: &mut [u8], off: usize, v: u64) { buf[off..off+8].copy_from_slice(&v.to_le_bytes()); }

    fn build_dynamic_entries(entries: &[(u64, u64)]) -> Vec<u8> {
        let mut buf = Vec::new();
        for &(tag, val) in entries {
            wu64(&mut buf, tag);
            wu64(&mut buf, val);
        }
        wu64(&mut buf, 0);
        wu64(&mut buf, 0);
        buf
    }

    fn build_elf(entry: u64, phdrs: &[(u32, u32, u64, u64, u64, u64)], data: &[u8]) -> Vec<u8> {
        let data_start: usize = 0x1000;
        let total = data_start + data.len();
        let mut file = vec![0u8; total];

        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;
        wu16_at(&mut file, 16, ET_SCE_DYNAMIC);
        wu16_at(&mut file, 18, EM_X86_64);
        wu32_at(&mut file, 20, 1);
        wu64_at(&mut file, 24, entry);
        wu64_at(&mut file, 32, 64);
        wu16_at(&mut file, 52, 64);
        wu16_at(&mut file, 54, 56);
        wu16_at(&mut file, 56, phdrs.len() as u16);

        for (i, &(p_type, p_flags, p_offset, p_vaddr, p_filesz, p_memsz)) in phdrs.iter().enumerate() {
            let off = 64 + i * 56;
            wu32_at(&mut file, off, p_type);
            wu32_at(&mut file, off + 4, p_flags);
            wu64_at(&mut file, off + 8, p_offset);
            wu64_at(&mut file, off + 16, p_vaddr);
            wu64_at(&mut file, off + 32, p_filesz);
            wu64_at(&mut file, off + 40, p_memsz);
            wu64_at(&mut file, off + 48, 0x1000);
        }

        file[data_start..data_start + data.len()].copy_from_slice(data);
        file
    }

    fn build_self(magic: u32, segments: &[(u64, u64, u64, u64)], elf_data: &[u8]) -> Vec<u8> {
        let mut file = Vec::new();
        let file_size = 32 + segments.len() * 32 + elf_data.len();

        file.extend_from_slice(&magic.to_be_bytes());
        file.push(0);
        file.push(1);
        file.push(1);
        file.push(0);
        wu32(&mut file, 0);
        wu16(&mut file, 0x560);
        wu16(&mut file, 0);
        wu64(&mut file, file_size as u64);
        wu16(&mut file, segments.len() as u16);
        wu16(&mut file, 0);
        wu32(&mut file, 0);

        for &(flags, file_offset, file_size, mem_size) in segments {
            wu64(&mut file, flags);
            wu64(&mut file, file_offset);
            wu64(&mut file, file_size);
            wu64(&mut file, mem_size);
        }

        file.extend_from_slice(elf_data);
        file
    }

    #[test]
    fn raw_elf_passthrough() {
        let dynamic = build_dynamic_entries(&[(DT_STRTAB, 0x1200), (DT_STRSZ, 0x10)]);
        let mut data = vec![0u8; 0x200];
        data[0..dynamic.len()].copy_from_slice(&dynamic);

        let elf = build_elf(0x1000, &[
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x200, 0x200),
        ], &data);

        let result = extract_elf(&elf).unwrap();
        assert!(!result.was_self);
        assert_eq!(result.elf, elf);
        assert_eq!(result.encrypted_segments, 0);
        assert_eq!(result.compressed_segments, 0);
    }

    #[test]
    fn self_single_load_segment() {
        let load_data = vec![0xAA; 0x100];

        let elf = build_elf(0x1000, &[
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x100, 0x100),
        ], &load_data);

        let elf_base: u64 = (32 + 1 * 32) as u64;
        let data_offset = elf_base + 0x1000;
        let segments = vec![
            (0u64 << 20 | SELF_SEGMENT_FLAG_DATA, data_offset, 0x100, 0x100),
        ];

        let self_data = build_self(SELF_MAGIC_PS5, &segments, &elf);
        let result = extract_elf(&self_data).unwrap();

        assert!(result.was_self);
        assert_eq!(result.encrypted_segments, 0);
        assert_eq!(result.compressed_segments, 0);

        assert!(result.elf.len() >= 0x1100);
        assert_eq!(&result.elf[0..4], &ELF_MAGIC);
        assert_eq!(result.elf[EI_CLASS], ELFCLASS64);

        let extracted_data = &result.elf[0x1000..0x1100];
        assert_eq!(extracted_data, load_data.as_slice());
    }

    #[test]
    fn self_two_load_segments() {
        let code = vec![0xCC; 0x200];
        let data_seg = vec![0xDD; 0x100];

        let elf = build_elf(0x1000, &[
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x80001000, 0x200, 0x200),
            (PT_LOAD, PF_R | PF_W, 0x1200, 0x80501000, 0x100, 0x200),
        ], &[code.as_slice(), data_seg.as_slice()].concat());

        let elf_base: u64 = (32 + 2 * 32) as u64;
        let code_offset = elf_base + 0x1000;
        let data_offset = elf_base + 0x1200;
        let segments = vec![
            (0u64 << 20 | SELF_SEGMENT_FLAG_DATA, code_offset, 0x200, 0x200),
            (1u64 << 20 | SELF_SEGMENT_FLAG_DATA, data_offset, 0x100, 0x200),
        ];

        let self_data = build_self(SELF_MAGIC_PS5, &segments, &elf);
        let result = extract_elf(&self_data).unwrap();

        assert!(result.was_self);
        assert!(result.elf.len() >= 0x1300);
        assert_eq!(&result.elf[0x1000..0x1200], code.as_slice());
        assert_eq!(&result.elf[0x1200..0x1300], data_seg.as_slice());
    }

    #[test]
    fn self_encrypted_segment_counted() {
        let load_data = vec![0xAA; 0x100];
        let dynamic = build_dynamic_entries(&[(DT_STRTAB, 0x1200), (DT_STRSZ, 0x10)]);
        let mut elf_payload = Vec::new();
        elf_payload.extend_from_slice(&dynamic);
        elf_payload.resize(0x200, 0);
        elf_payload.extend_from_slice(&load_data);

        let elf = build_elf(0x1000, &[
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x100, 0x100),
        ], &elf_payload);

        let elf_base: u64 = (32 + 1 * 32) as u64;
        let data_offset = elf_base + 0x1000;
        let segments = vec![
            (SELF_SEGMENT_FLAG_ENCRYPTED | SELF_SEGMENT_FLAG_DATA, data_offset, 0x100, 0x100),
        ];

        let self_data = build_self(SELF_MAGIC_PS5, &segments, &elf);
        let result = extract_elf(&self_data).unwrap();

        assert!(result.was_self);
        assert_eq!(result.encrypted_segments, 1);
        assert_eq!(result.compressed_segments, 0);
    }

    #[test]
    fn truncated_self_returns_error() {
        assert!(extract_elf(&[0u8; 10]).is_err());
    }

    #[test]
    fn unknown_magic_returns_error() {
        assert!(extract_elf(&[0xFF; 100]).is_err());
    }

    #[test]
    fn round_trip_self_to_valid_elf() {
        let mut strtab = vec![0u8];
        strtab.extend_from_slice(b"libSceTest\0");
        let strtab_vaddr = 0x1200u64;

        let dynamic = build_dynamic_entries(&[
            (DT_STRTAB, strtab_vaddr),
            (DT_STRSZ, strtab.len() as u64),
            (DT_NEEDED, 1),
            (DT_INIT, 0x80001050),
            (DT_FINI, 0x80001060),
        ]);

        let mut load_data = Vec::new();
        load_data.extend_from_slice(&dynamic);
        load_data.resize(0x200, 0);
        load_data.extend_from_slice(&strtab);
        load_data.resize(0x400, 0);

        let elf = build_elf(0x80001000, &[
            (PT_LOAD, PF_R | PF_X, 0x1000, 0x80001000, 0x400, 0x400),
            (PT_DYNAMIC, PF_R, 0x1000, 0x80001000, dynamic.len() as u64, dynamic.len() as u64),
        ], &load_data);

        let elf_base: u64 = (32 + 2 * 32) as u64;
        let data_offset = elf_base + 0x1000;
        let segments = vec![
            (0u64 << 20 | SELF_SEGMENT_FLAG_DATA, data_offset, 0x400, 0x400),
            (1u64 << 20 | SELF_SEGMENT_FLAG_DATA, data_offset, 0x400, 0x400),
        ];

        let self_data = build_self(SELF_MAGIC_PS5, &segments, &elf);
        let result = extract_elf(&self_data).unwrap();
        assert!(result.was_self);

        let parsed = ps5_elf::ElfImage::parse(&result.elf, None).unwrap();
        assert_eq!(parsed.header.e_entry, 0x80001000);
        assert_eq!(parsed.program_headers.len(), 2);
        assert_eq!(parsed.program_headers[0].p_type, PT_LOAD);
        assert_eq!(parsed.program_headers[0].p_vaddr, 0x80001000);
        assert_eq!(parsed.program_headers[0].p_filesz, 0x400);
        assert!(parsed.program_headers[0].is_executable());
        assert_eq!(parsed.program_headers[1].p_type, PT_DYNAMIC);
        assert!(!parsed.dynamic_entries.is_empty());

        let init_entry = parsed.dynamic_entries.iter().find(|e| e.d_tag == DT_INIT);
        assert!(init_entry.is_some());
        assert_eq!(init_entry.unwrap().d_val, 0x80001050);

        let needed = parsed.dynamic_entries.iter().find(|e| e.d_tag == DT_NEEDED);
        assert!(needed.is_some());
        assert_eq!(needed.unwrap().d_val, 1);
    }
}
