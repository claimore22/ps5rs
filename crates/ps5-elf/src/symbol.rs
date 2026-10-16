use crate::{read_u16, read_u32, read_u64};
use ps5_format::elf_constants::{STB_GLOBAL, STB_WEAK};
use ps5_format::error::Result;

#[derive(Debug, Clone)]
pub struct SymEntry {
    pub st_name: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
    pub st_value: u64,
    pub st_size: u64,
    pub resolved_name: String,
    pub is_import: bool,
}

impl SymEntry {
    pub fn is_export_candidate(&self) -> bool {
        if self.is_import || self.st_value == 0 {
            return false;
        }
        let bind = self.st_info >> 4;
        if bind != STB_GLOBAL && bind != STB_WEAK {
            return false;
        }
        if !self.resolved_name.contains('#') {
            return true;
        }
        let mut parts = self.resolved_name.split('#');
        matches!(parts.next(), Some(nid) if nid.len() == 11)
            && parts.next().is_some_and(|p| !p.is_empty())
            && parts.next().is_some_and(|p| !p.is_empty())
    }
}

pub fn parse_symbols(
    data: &[u8],
    symtab_offset: u64,
    syment: u64,
    count: usize,
    strtab_offset: u64,
) -> Result<Vec<SymEntry>> {
    let mut symbols = Vec::with_capacity(count);
    let start = symtab_offset as usize;
    let entry_size = syment as usize;
    let data_end = data.len();

    for i in 0..count {
        let pos = start + i * entry_size;
        if pos + entry_size > data_end {
            break;
        }

        let st_name = read_u32(data, pos);
        let st_info = data[pos + 4];
        let st_other = data[pos + 5];
        let st_shndx = read_u16(data, pos + 6);
        let st_value = read_u64(data, pos + 8);
        let st_size = read_u64(data, pos + 16);

        let name = resolve_name(data, st_name, strtab_offset);
        let is_import = st_shndx == 0 && st_value == 0 && name.contains('#');

        symbols.push(SymEntry {
            st_name,
            st_info,
            st_other,
            st_shndx,
            st_value,
            st_size,
            resolved_name: name,
            is_import,
        });
    }

    Ok(symbols)
}

fn resolve_name(data: &[u8], name_offset: u32, strtab_offset: u64) -> String {
    let abs = strtab_offset as usize + name_offset as usize;
    if abs >= data.len() {
        return String::new();
    }
    let slice = &data[abs..];
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, info: u8, shndx: u16, value: u64) -> SymEntry {
        let is_import = shndx == 0 && value == 0 && name.contains('#');
        SymEntry {
            st_name: 0,
            st_info: info,
            st_other: 0,
            st_shndx: shndx,
            st_value: value,
            st_size: 0,
            resolved_name: name.to_string(),
            is_import,
        }
    }

    #[test]
    fn export_candidate_global_nid() {
        assert!(sym("H2e8t5ScQGc#D#A", 0x12, 3, 0x1000).is_export_candidate());
    }

    #[test]
    fn export_candidate_weak() {
        assert!(sym("H2e8t5ScQGc#D#A", 0x22, 3, 0x1000).is_export_candidate());
    }

    #[test]
    fn export_candidate_rejects_local() {
        assert!(!sym("H2e8t5ScQGc#D#A", 0x02, 3, 0x1000).is_export_candidate());
    }

    #[test]
    fn export_candidate_rejects_import() {
        assert!(!sym("pB-yGZ2nQ9o#A#B", 0x12, 0, 0).is_export_candidate());
    }

    #[test]
    fn export_candidate_rejects_malformed_nid() {
        assert!(!sym("short#D#A", 0x12, 3, 0x1000).is_export_candidate());
        assert!(!sym("H2e8t5ScQGc##A", 0x12, 3, 0x1000).is_export_candidate());
        assert!(!sym("H2e8t5ScQGc#D", 0x12, 3, 0x1000).is_export_candidate());
    }

    #[test]
    fn export_candidate_plain_global_name() {
        assert!(sym("my_func", 0x12, 3, 0x1000).is_export_candidate());
    }
}
