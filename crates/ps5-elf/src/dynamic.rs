use ps5_format::error::Result;

use crate::read_u64;

#[derive(Debug, Clone, Copy)]
pub struct DynEntry {
    pub d_tag: u64,
    pub d_val: u64,
}

pub fn parse_dynamic(
    data: &[u8],
    phdr: &crate::ProgramHeader,
    file_offset: u64,
) -> Result<Vec<DynEntry>> {
    let mut entries = Vec::new();
    let start = file_offset as usize;
    let end = start + phdr.p_filesz as usize;
    let data_end = data.len();

    let mut offset = start;
    loop {
        if offset + 16 > data_end.min(end) {
            break;
        }
        let tag = read_u64(data, offset);
        let val = read_u64(data, offset + 8);
        entries.push(DynEntry {
            d_tag: tag,
            d_val: val,
        });
        if tag == 0 {
            break;
        }
        offset += 16;
    }

    Ok(entries)
}

pub fn parse_import_libs(
    entries: &[DynEntry],
    strtab: &[u8],
) -> std::collections::HashMap<u16, String> {
    let mut libs = std::collections::HashMap::new();
    for entry in entries {
        if entry.d_tag == ps5_format::self_constants::DT_SCE_NEEDED_LIB {
            let name_off = (entry.d_val & 0xffffffff) as usize;
            let id = ((entry.d_val >> 48) & 0xffff) as u16;
            let name = read_cstr(strtab, name_off);
            libs.insert(id, name);
        }
    }
    libs
}

pub fn parse_needed_files(entries: &[DynEntry], strtab: &[u8]) -> Vec<String> {
    let mut files = Vec::new();
    for entry in entries {
        if entry.d_tag == ps5_format::elf_constants::DT_NEEDED {
            let name_off = (entry.d_val & 0xffffffff) as usize;
            files.push(read_cstr(strtab, name_off));
        }
    }
    files
}

fn read_cstr(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }
    let slice = &data[offset..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).into_owned()
}
