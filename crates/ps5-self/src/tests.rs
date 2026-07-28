#![allow(clippy::identity_op)]

use super::*;
use ps5_format::elf_constants::*;

fn wu16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn wu32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn wu64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn wu16_at(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn wu32_at(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn wu64_at(buf: &mut [u8], off: usize, v: u64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

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

fn build_elf_bytes(entry: u64, phdrs: &[(u32, u32, u64, u64, u64, u64)], data: &[u8]) -> Vec<u8> {
    let phdr_count = phdrs.len();
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
    wu16_at(&mut file, 56, phdr_count as u16);

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
    file.push(0); // version
    file.push(1); // mode
    file.push(1); // endian = LE
    file.push(0); // attr
    wu32(&mut file, 0); // key_type
    wu16(&mut file, 0x560); // header_size
    wu16(&mut file, 0); // meta_size
    wu64(&mut file, file_size as u64); // file_size
    wu16(&mut file, segments.len() as u16); // num_segments
    wu16(&mut file, 0); // flags
    wu32(&mut file, 0); // pad

    for &(flags, file_offset, file_size, mem_size) in segments {
        wu64(&mut file, flags);
        wu64(&mut file, file_offset);
        wu64(&mut file, file_size);
        wu64(&mut file, mem_size);
    }

    file.extend_from_slice(elf_data);
    file
}

// === Platform detection tests ===

#[test]
fn platform_raw_elf() {
    let elf = build_elf_bytes(0x1000, &[], &[]);
    assert_eq!(SelfPlatform::from_bytes(&elf), SelfPlatform::RawElf);
}

#[test]
fn platform_ps4() {
    let data = build_self(SELF_MAGIC_PS4, &[], &[]);
    assert_eq!(SelfPlatform::from_bytes(&data), SelfPlatform::Ps4);
}

#[test]
fn platform_ps5() {
    let data = build_self(SELF_MAGIC_PS5, &[], &[]);
    assert_eq!(SelfPlatform::from_bytes(&data), SelfPlatform::Ps5);
}

#[test]
fn platform_unknown() {
    assert_eq!(
        SelfPlatform::from_bytes(&[0xFF, 0xFF, 0xFF, 0xFF]),
        SelfPlatform::Unknown(0xFFFFFFFF)
    );
}

#[test]
fn platform_short_data() {
    assert_eq!(SelfPlatform::from_bytes(&[0x7f]), SelfPlatform::Unknown(0));
}

// === Raw ELF parsing through SelfImage ===

#[test]
fn parse_raw_elf() {
    let mut data = vec![0u8; 0x200];
    let dynamic = build_dynamic_entries(&[(DT_STRTAB, 0x1200), (DT_STRSZ, 0x10)]);
    data[0..dynamic.len()].copy_from_slice(&dynamic);

    let elf = build_elf_bytes(
        0x1000,
        &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x200, 0x200)],
        &data,
    );

    let img = SelfImage::parse(&elf).unwrap();
    assert_eq!(img.platform, SelfPlatform::RawElf);
    assert!(!img.is_self());
    assert!(img.segments.is_empty());
}

// === SELF container parsing ===

#[test]
fn parse_ps4_self_single_segment() {
    let mut elf_data = vec![0u8; 0x400];
    let dynamic = build_dynamic_entries(&[(DT_STRTAB, 0x1200), (DT_STRSZ, 0x10)]);
    elf_data[0..dynamic.len()].copy_from_slice(&dynamic);

    let elf = build_elf_bytes(
        0x1000,
        &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x400, 0x400)],
        &elf_data,
    );

    let elf_base: u64 = (32 + 1 * 32) as u64;
    let segments = vec![(0u64 << 20 | SELF_SEGMENT_FLAG_DATA, elf_base, 0x400, 0x400)];

    let self_data = build_self(SELF_MAGIC_PS4, &segments, &elf);
    let img = SelfImage::parse(&self_data).unwrap();

    assert!(img.is_self());
    assert_eq!(img.platform, SelfPlatform::Ps4);
    assert_eq!(img.segments.len(), 1);
    assert!(img.segments[0].is_data());
    assert!(!img.segments[0].is_encrypted());
    assert!(!img.segments[0].is_compressed());
    assert_eq!(img.segments[0].phdr_index(), 0);
}

#[test]
fn parse_ps5_self() {
    let elf_data = vec![0u8; 0x100];
    let elf = build_elf_bytes(
        0x1000,
        &[(PT_LOAD, PF_R | PF_X, 0x1000, 0x1000, 0x100, 0x100)],
        &elf_data,
    );

    let elf_base: u64 = (32 + 1 * 32) as u64;
    let segments = vec![(0u64 << 20 | SELF_SEGMENT_FLAG_DATA, elf_base, 0x100, 0x100)];

    let self_data = build_self(SELF_MAGIC_PS5, &segments, &elf);
    let img = SelfImage::parse(&self_data).unwrap();
    assert_eq!(img.platform, SelfPlatform::Ps5);
    assert_eq!(img.segments.len(), 1);
}

#[test]
fn parse_self_multiple_segments() {
    let elf = build_elf_bytes(
        0x1000,
        &[(PT_LOAD, PF_R, 0x1000, 0x1000, 0x100, 0x100)],
        &vec![0u8; 0x100],
    );

    let elf_base: u64 = (32 + 2 * 32) as u64;
    let segments = vec![
        (0u64 << 20 | SELF_SEGMENT_FLAG_DATA, elf_base, 0x100, 0x100),
        (1u64 << 20 | 0, elf_base + 0x100, 0x50, 0x100),
    ];

    let self_data = build_self(SELF_MAGIC_PS4, &segments, &elf);
    let img = SelfImage::parse(&self_data).unwrap();
    assert_eq!(img.data_segments().len(), 1);
    assert_eq!(img.segments.len(), 2);
}

// === Segment entry tests ===

#[test]
fn segment_parse() {
    let mut seg_bytes = vec![0u8; 32];
    wu64_at(&mut seg_bytes, 0, 0x800);
    wu64_at(&mut seg_bytes, 8, 0x500);
    wu64_at(&mut seg_bytes, 16, 0x200);
    wu64_at(&mut seg_bytes, 24, 0x300);

    let seg = SelfSegmentEntry::parse(&seg_bytes, 0).unwrap();
    assert_eq!(seg.flags, 0x800);
    assert_eq!(seg.file_offset, 0x500);
    assert_eq!(seg.file_size, 0x200);
    assert_eq!(seg.mem_size, 0x300);
    assert!(seg.is_data());
    assert_eq!(seg.phdr_index(), 0);
}

#[test]
fn segment_high_phdr_index() {
    let mut seg_bytes = vec![0u8; 32];
    let flags = (0x1FFu64 << 20) | SELF_SEGMENT_FLAG_DATA;
    wu64_at(&mut seg_bytes, 0, flags);
    wu64_at(&mut seg_bytes, 8, 0x100);
    wu64_at(&mut seg_bytes, 16, 0x200);
    wu64_at(&mut seg_bytes, 24, 0x200);

    let seg = SelfSegmentEntry::parse(&seg_bytes, 0).unwrap();
    assert_eq!(seg.phdr_index(), 0x1FF);
}

#[test]
fn segment_not_data() {
    let seg = SelfSegmentEntry {
        flags: 0,
        file_offset: 100,
        file_size: 200,
        mem_size: 200,
    };
    assert!(!seg.is_data());
}

#[test]
fn segment_encrypted() {
    let seg = SelfSegmentEntry {
        flags: SELF_SEGMENT_FLAG_ENCRYPTED | SELF_SEGMENT_FLAG_DATA,
        file_offset: 0,
        file_size: 0,
        mem_size: 0,
    };
    assert!(seg.is_encrypted());
    assert!(seg.is_data());
}

#[test]
fn segment_compressed() {
    let seg = SelfSegmentEntry {
        flags: SELF_SEGMENT_FLAG_COMPRESSED,
        file_offset: 0,
        file_size: 0,
        mem_size: 0,
    };
    assert!(seg.is_compressed());
    assert!(!seg.is_data());
}

// === Error cases ===

#[test]
fn truncated_self_header() {
    assert!(SelfImage::parse(&[0u8; 10]).is_err());
}

#[test]
fn segment_parse_truncated() {
    let seg_bytes = vec![0u8; 10];
    assert!(SelfSegmentEntry::parse(&seg_bytes, 0).is_err());
}
