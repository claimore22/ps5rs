use crate::{read_u16, read_u32, read_u64};
use ps5_format::elf_constants::ELF_MAGIC;
use ps5_format::error::{ParseError, Result};
use ps5_nid::encode_nid;

const STUB_LIBRARY_SUFFIX: &str = "_stub_weak.a";
const AR_MAGIC: &[u8; 8] = b"!<arch>\n";
const ELF64_SYMENT_SIZE: u64 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubSymbol {
    pub nid: String,
    pub name: String,
    pub library: String,
}

pub fn stub_library_name(file_name: &str) -> &str {
    file_name
        .strip_suffix(STUB_LIBRARY_SUFFIX)
        .unwrap_or(file_name)
}

pub fn parse_stub_library(data: &[u8], library: &str) -> Result<Vec<StubSymbol>> {
    if data.starts_with(&ELF_MAGIC) {
        parse_stub_object(data, library)
    } else if data.starts_with(AR_MAGIC) {
        parse_stub_archive(data, library)
    } else {
        Err(ParseError::Custom(format!(
            "stub {library} is neither an ELF object nor an ar archive"
        )))
    }
}

fn parse_stub_archive(data: &[u8], library: &str) -> Result<Vec<StubSymbol>> {
    let mut symbols = Vec::new();
    let mut long_names: Option<&[u8]> = None;
    let mut offset = AR_MAGIC.len();
    while offset + 60 <= data.len() {
        let name_field = &data[offset..offset + 16];
        let size_field = &data[offset + 48..offset + 58];
        let size = std::str::from_utf8(size_field)
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .ok_or_else(|| {
                ParseError::Custom(format!("ar member size invalid at offset {offset}"))
            })?;
        let data_start = offset + 60;
        if data_start + size > data.len() {
            return Err(ParseError::Truncated {
                offset: data_start as u64,
                needed: size as u64,
                available: (data.len() - data_start) as u64,
            });
        }
        let member = &data[data_start..data_start + size];
        let name = name_field
            .split(|&b| b == b' ' || b == 0)
            .next()
            .unwrap_or(&[]);
        match name {
            b"//" => long_names = Some(member),
            b"/" => {}
            _ if name.starts_with(b"/") => {
                let index = std::str::from_utf8(&name[1..])
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| ParseError::Custom("ar long-name index invalid".to_string()))?;
                let table = long_names
                    .ok_or_else(|| ParseError::Custom("ar long-name table missing".to_string()))?;
                let resolved = read_ar_name(table, index).ok_or_else(|| {
                    ParseError::Custom("ar long-name offset out of range".to_string())
                })?;
                symbols.extend(parse_member(member, library, &resolved)?);
            }
            _ => {
                let resolved = name
                    .strip_suffix(b"/")
                    .map(|n| String::from_utf8_lossy(n).into_owned())
                    .unwrap_or_else(|| String::from_utf8_lossy(name).into_owned());
                if !resolved.is_empty() {
                    symbols.extend(parse_member(member, library, &resolved)?);
                }
            }
        }
        offset = data_start + size;
        if offset < data.len() && data[offset] == b'\n' {
            offset += 1;
        }
    }
    Ok(symbols)
}

fn parse_member(data: &[u8], library: &str, member_name: &str) -> Result<Vec<StubSymbol>> {
    if !data.starts_with(&ELF_MAGIC) {
        return Ok(Vec::new());
    }
    parse_stub_object(data, library)
        .map_err(|e| ParseError::Custom(format!("{member_name} in stub {library}: {e}")))
}

fn read_ar_name(table: &[u8], index: usize) -> Option<String> {
    let tail = table.get(index..)?;
    let end = tail
        .iter()
        .position(|&b| b == b'\n' || b == 0)
        .unwrap_or(tail.len());
    Some(String::from_utf8_lossy(&tail[..end]).into_owned())
}

fn parse_stub_object(data: &[u8], library: &str) -> Result<Vec<StubSymbol>> {
    if data.len() < 64 {
        return Err(ParseError::Truncated {
            offset: 0,
            needed: 64,
            available: data.len() as u64,
        });
    }
    if data[0..4] != ELF_MAGIC {
        return Err(ParseError::InvalidMagic {
            expected: u32::from_le_bytes(ELF_MAGIC),
            actual: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
        });
    }

    let e_shoff = read_u64(data, 0x28);
    let e_shentsize = read_u16(data, 0x3a);
    let e_shnum = read_u16(data, 0x3c);
    let e_shstrndx = read_u16(data, 0x3e);
    if e_shoff == 0 || e_shentsize < 64 || e_shnum == 0 {
        return Err(ParseError::Custom(format!(
            "stub {library} has no section headers"
        )));
    }
    let shdr_end = e_shoff
        .checked_add(e_shentsize as u64 * e_shnum as u64)
        .ok_or_else(|| ParseError::Custom("section table size overflow".to_string()))?;
    if shdr_end > data.len() as u64 {
        return Err(ParseError::Truncated {
            offset: e_shoff,
            needed: shdr_end - e_shoff,
            available: (data.len() as u64).saturating_sub(e_shoff),
        });
    }
    let shdrs = &data[e_shoff as usize..shdr_end as usize];
    let section = |index: usize| -> Option<&[u8]> {
        let start = index * e_shentsize as usize;
        shdrs.get(start..start + 64)
    };

    let shstrtab = section(e_shstrndx as usize)
        .ok_or_else(|| ParseError::Custom("shstrtab index out of range".to_string()))?;
    let shstr_off = sh_offset(shstrtab);
    let shstr_size = sh_size(shstrtab);
    let shstr_end = shstr_off
        .checked_add(shstr_size)
        .ok_or_else(|| ParseError::Custom("shstrtab bounds overflow".to_string()))?;
    if shstr_end > data.len() as u64 {
        return Err(ParseError::Truncated {
            offset: shstr_off,
            needed: shstr_size,
            available: (data.len() as u64).saturating_sub(shstr_off),
        });
    }
    let shstr = &data[shstr_off as usize..shstr_end as usize];

    let mut dynsym = None;
    let mut dynstr = None;
    let mut scenid = None;
    for i in 0..e_shnum as usize {
        let sh = section(i).ok_or(ParseError::Truncated {
            offset: shdrs.len() as u64,
            needed: 1,
            available: 0,
        })?;
        let name = read_cstr(shstr, sh_name(sh) as usize);
        if name.is_empty() {
            continue;
        }
        let off = sh_offset(sh);
        let size = sh_size(sh);
        let end = off
            .checked_add(size)
            .ok_or_else(|| ParseError::Custom("section bounds overflow".to_string()))?;
        if end > data.len() as u64 {
            return Err(ParseError::Truncated {
                offset: off,
                needed: size,
                available: (data.len() as u64).saturating_sub(off),
            });
        }
        let slice = &data[off as usize..end as usize];
        match name.as_str() {
            ".dynsym" => dynsym = Some(slice),
            ".dynstr" => dynstr = Some(slice),
            ".scenid" => scenid = Some(slice),
            _ => {}
        }
    }

    let dynsym = dynsym.ok_or(ParseError::InvalidSymbolTable)?;
    let dynstr = dynstr.ok_or(ParseError::InvalidStringTable)?;
    if !(dynsym.len() as u64).is_multiple_of(ELF64_SYMENT_SIZE) {
        return Err(ParseError::Custom(format!(
            "stub {library} .dynsym size not a multiple of 24"
        )));
    }
    let symbol_count = (dynsym.len() as u64 / ELF64_SYMENT_SIZE) as usize;
    let scenid =
        scenid.ok_or_else(|| ParseError::Custom(format!("stub {library} missing .scenid")))?;
    if scenid.len() != symbol_count * 8 {
        return Err(ParseError::Custom(format!(
            "stub {library} .scenid size {} does not match {} symbols",
            scenid.len(),
            symbol_count
        )));
    }

    let mut symbols = Vec::new();
    for i in 1..symbol_count {
        let sym = &dynsym[i * 24..i * 24 + 24];
        let st_name = read_u32(sym, 0) as usize;
        if st_name == 0 {
            continue;
        }
        let name = read_cstr(dynstr, st_name);
        if name.is_empty() {
            continue;
        }
        let raw = &scenid[i * 8..i * 8 + 8];
        if raw.iter().all(|&b| b == 0) {
            continue;
        }
        let mut reversed = [0u8; 8];
        for k in 0..8 {
            reversed[k] = raw[7 - k];
        }
        symbols.push(StubSymbol {
            nid: encode_nid(reversed),
            name,
            library: library.to_string(),
        });
    }
    Ok(symbols)
}

fn sh_name(sh: &[u8]) -> u32 {
    read_u32(sh, 0)
}

fn sh_offset(sh: &[u8]) -> u64 {
    read_u64(sh, 0x18)
}

fn sh_size(sh: &[u8]) -> u64 {
    read_u64(sh, 0x20)
}

fn read_cstr(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }
    let tail = &data[offset..];
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    String::from_utf8_lossy(&tail[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENID_SCEAGCINIT: [u8; 8] = [83, 187, 216, 43, 81, 209, 114, 219];
    const SCENID_SCEKERNELADDUSEREVENT: [u8; 8] = [64, 112, 54, 242, 58, 191, 30, 225];

    #[test]
    fn parses_known_scenid_pairs() {
        let data = build_stub_object(&[
            ("sceAgcInit", SCENID_SCEAGCINIT),
            ("sceKernelAddUserEvent", SCENID_SCEKERNELADDUSEREVENT),
        ]);
        let symbols = parse_stub_library(&data, "libSceAgc").unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].nid, "23LRUSvYu1M");
        assert_eq!(symbols[0].name, "sceAgcInit");
        assert_eq!(symbols[0].library, "libSceAgc");
        assert_eq!(symbols[1].nid, "4R6-OvI2cEA");
        assert_eq!(symbols[1].name, "sceKernelAddUserEvent");
        assert_eq!(symbols[1].library, "libSceAgc");
    }

    #[test]
    fn skips_anonymous_and_empty_names() {
        let data = build_stub_object_raw(
            &["", "sceAgcInit"],
            Some(&[[0u8; 8], SCENID_SCEAGCINIT, [0u8; 8]].concat()),
        );
        let symbols = parse_stub_library(&data, "libSceAgc").unwrap();
        assert!(symbols.is_empty());
    }

    #[test]
    fn errors_on_missing_scenid() {
        let data = build_stub_object_raw(&["sceAgcInit"], None);
        let err = parse_stub_library(&data, "libSceAgc").unwrap_err();
        assert!(err.to_string().contains("missing .scenid"));
    }

    #[test]
    fn errors_on_scenid_size_mismatch() {
        let data = build_stub_object_raw(&["a", "b"], Some(&[0u8; 8 * 4]));
        let err = parse_stub_library(&data, "lib").unwrap_err();
        assert!(err.to_string().contains("does not match"));
    }

    #[test]
    fn errors_on_dynsym_size_not_multiple_of_24() {
        let data = build_stub_object(&[("sceAgcInit", SCENID_SCEAGCINIT)]);
        let e_shoff = read_u64(&data, 0x28) as usize;
        let e_shentsize = read_u16(&data, 0x3a) as usize;
        let dynsym_idx = find_section_index(&data, ".dynsym");
        let mut data = data;
        write_u64(&mut data, e_shoff + dynsym_idx * e_shentsize + 0x20, 25);
        let err = parse_stub_library(&data, "lib").unwrap_err();
        assert!(err.to_string().contains("multiple of 24"));
    }

    #[test]
    fn rejects_truncated_elf() {
        let data = build_stub_object(&[("sceAgcInit", SCENID_SCEAGCINIT)]);
        let err = parse_stub_library(&data[..data.len() - 10], "lib").unwrap_err();
        assert!(matches!(err, ParseError::Truncated { .. }));
    }

    #[test]
    fn rejects_non_elf_input() {
        let err = parse_stub_library(b"not an elf or archive", "lib").unwrap_err();
        assert!(err.to_string().contains("neither"));
    }

    #[test]
    fn rejects_invalid_magic_elf() {
        let mut data = build_stub_object(&[("sceAgcInit", SCENID_SCEAGCINIT)]);
        data[0] = 0x4f;
        let err = parse_stub_object(&data, "lib").unwrap_err();
        assert!(matches!(err, ParseError::InvalidMagic { .. }));
    }

    #[test]
    fn parses_archive_with_multiple_members() {
        let inner_a = build_stub_object(&[("sceAgcInit", SCENID_SCEAGCINIT)]);
        let inner_b = build_stub_object(&[("sceKernelAddUserEvent", SCENID_SCEKERNELADDUSEREVENT)]);
        let archive = build_archive(&[
            ("libSceAgc_stub_weak.a_first_member.o", &inner_a),
            ("libSceAgc_stub_weak.a_second_member.o", &inner_b),
        ]);
        let symbols = parse_stub_library(&archive, "libSceAgc").unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].nid, "23LRUSvYu1M");
        assert_eq!(symbols[0].library, "libSceAgc");
        assert_eq!(symbols[1].nid, "4R6-OvI2cEA");
    }

    #[test]
    fn parses_empty_archive() {
        let archive = build_archive(&[]);
        let symbols = parse_stub_library(&archive, "lib").unwrap();
        assert!(symbols.is_empty());
    }

    #[test]
    fn returns_both_conflicting_names() {
        let data = build_stub_object_raw(
            &["sceAgcInit", "sceAgcInitRenamed"],
            Some(&[[0u8; 8], SCENID_SCEAGCINIT, SCENID_SCEAGCINIT].concat()),
        );
        let symbols = parse_stub_library(&data, "libSceAgc").unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].nid, symbols[1].nid);
        assert_ne!(symbols[0].name, symbols[1].name);
    }

    #[test]
    fn strips_stub_suffix() {
        assert_eq!(stub_library_name("libSceAgc_stub_weak.a"), "libSceAgc");
        assert_eq!(stub_library_name("libkernel_stub_weak.a"), "libkernel");
        assert_eq!(stub_library_name("no_suffix"), "no_suffix");
    }

    fn find_section_index(data: &[u8], target: &str) -> usize {
        let e_shoff = read_u64(data, 0x28) as usize;
        let e_shentsize = read_u16(data, 0x3a) as usize;
        let e_shnum = read_u16(data, 0x3c) as usize;
        let shstrndx = read_u16(data, 0x3e) as usize;
        let shstr_hdr = &data[e_shoff + shstrndx * e_shentsize..];
        let shstr_off = sh_offset(shstr_hdr) as usize;
        let shstr_size = sh_size(shstr_hdr) as usize;
        let shstr = &data[shstr_off..shstr_off + shstr_size];
        for i in 0..e_shnum {
            let sh = &data[e_shoff + i * e_shentsize..];
            let name = read_cstr(shstr, sh_name(sh) as usize);
            if name == target {
                return i;
            }
        }
        panic!("section {target} not found");
    }

    fn build_stub_object(symbols: &[(&str, [u8; 8])]) -> Vec<u8> {
        let names: Vec<&str> = symbols.iter().map(|(n, _)| *n).collect();
        let mut scenid = vec![0u8; 8];
        for (_, b) in symbols {
            scenid.extend_from_slice(b);
        }
        build_stub_object_raw(&names, Some(&scenid))
    }

    fn build_stub_object_raw(names: &[&str], scenid: Option<&[u8]>) -> Vec<u8> {
        let mut dynstr = vec![0u8];
        let mut st_names = Vec::with_capacity(names.len());
        for name in names {
            st_names.push(dynstr.len());
            dynstr.extend_from_slice(name.as_bytes());
            dynstr.push(0);
        }
        let symbol_count = names.len() + 1;
        let mut section_names = vec![".shstrtab", ".dynstr", ".dynsym"];
        if scenid.is_some() {
            section_names.push(".scenid");
        }
        let mut shstr = vec![0u8];
        let mut name_offsets = std::collections::HashMap::new();
        for name in &section_names {
            name_offsets.insert(*name, shstr.len());
            shstr.extend_from_slice(name.as_bytes());
            shstr.push(0);
        }
        let scenid_bytes = scenid.unwrap_or(&[]);
        let mut off = 64usize;
        let dynstr_off = off;
        off += dynstr.len();
        let dynsym_off = align8(off);
        off = dynsym_off + symbol_count * 24;
        let scenid_off = align8(off);
        off = scenid_off + scenid_bytes.len();
        let shstr_off = align8(off);
        off = shstr_off + shstr.len();
        let shdr_off = align8(off);
        off = shdr_off + section_names.len() * 64;

        let mut data = vec![0u8; off];
        data[0..4].copy_from_slice(&ELF_MAGIC);
        data[4] = 2;
        data[5] = 1;
        data[6] = 1;
        write_u16(&mut data, 16, 2);
        write_u16(&mut data, 18, 0x3e);
        write_u32(&mut data, 20, 1);
        write_u64(&mut data, 40, shdr_off as u64);
        write_u16(&mut data, 52, 64);
        write_u16(&mut data, 58, 64);
        write_u16(&mut data, 60, section_names.len() as u16);
        write_u16(&mut data, 62, 0);

        data[dynstr_off..dynstr_off + dynstr.len()].copy_from_slice(&dynstr);
        for (i, &st_name) in st_names.iter().enumerate() {
            let base = dynsym_off + (i + 1) * 24;
            write_u32(&mut data, base, st_name as u32);
            data[base + 4] = 0x12;
        }
        data[scenid_off..scenid_off + scenid_bytes.len()].copy_from_slice(scenid_bytes);
        data[shstr_off..shstr_off + shstr.len()].copy_from_slice(&shstr);

        let sh = |data: &mut [u8],
                  idx: usize,
                  name: &str,
                  ty: u32,
                  off: u64,
                  size: u64,
                  link: u32,
                  align: u64| {
            let base = shdr_off + idx * 64;
            write_u32(data, base, name_offsets[name] as u32);
            write_u32(data, base + 4, ty);
            write_u64(data, base + 0x18, off);
            write_u64(data, base + 0x20, size);
            write_u32(data, base + 0x28, link);
            write_u64(data, base + 0x30, align);
        };
        sh(
            &mut data,
            0,
            ".shstrtab",
            3,
            shstr_off as u64,
            shstr.len() as u64,
            0,
            1,
        );
        sh(
            &mut data,
            1,
            ".dynstr",
            3,
            dynstr_off as u64,
            dynstr.len() as u64,
            0,
            1,
        );
        sh(
            &mut data,
            2,
            ".dynsym",
            11,
            dynsym_off as u64,
            (symbol_count * 24) as u64,
            1,
            8,
        );
        if scenid.is_some() {
            sh(
                &mut data,
                3,
                ".scenid",
                1,
                scenid_off as u64,
                scenid_bytes.len() as u64,
                0,
                8,
            );
        }
        data
    }

    fn build_archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = b"!<arch>\n".to_vec();
        let mut strtab = Vec::new();
        let mut offsets = Vec::new();
        for (name, _) in members {
            offsets.push(strtab.len());
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }
        if !members.is_empty() {
            push_ar_member(&mut out, b"//", &strtab);
        }
        for (i, (name, data)) in members.iter().enumerate() {
            let field = if name.len() <= 16 {
                name.as_bytes().to_vec()
            } else {
                format!("/{}", offsets[i]).into_bytes()
            };
            push_ar_member(&mut out, &field, data);
        }
        out
    }

    fn push_ar_member(out: &mut Vec<u8>, name: &[u8], data: &[u8]) {
        let mut header = [b' '; 60];
        header[0..name.len()].copy_from_slice(name);
        let size = data.len().to_string();
        header[48..48 + size.len()].copy_from_slice(size.as_bytes());
        header[58] = b'`';
        header[59] = b'\n';
        out.extend_from_slice(&header);
        out.extend_from_slice(data);
        if data.len() % 2 == 1 {
            out.push(b'\n');
        }
    }

    fn align8(v: usize) -> usize {
        (v + 7) & !7
    }

    fn write_u16(data: &mut [u8], off: usize, v: u16) {
        data[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn write_u32(data: &mut [u8], off: usize, v: u32) {
        data[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn write_u64(data: &mut [u8], off: usize, v: u64) {
        data[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
}
