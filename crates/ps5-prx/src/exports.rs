use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub nid: String,
    pub name: Option<String>,
    pub library: String,
    pub address: u64,
}

pub fn extract_exports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ExportEntry> {
    elf.symbols
        .iter()
        .filter(|s| !s.is_import && s.st_value != 0)
        .map(|sym| {
            let nid = sym
                .resolved_name
                .split('#')
                .next()
                .unwrap_or("")
                .to_string();
            let name = if nid.is_empty() {
                Some(sym.resolved_name.clone())
            } else {
                catalog
                    .resolve(&nid)
                    .and_then(|e| e.primary_name().map(|s| s.to_string()))
            };
            ExportEntry {
                nid,
                name,
                library: elf.soname.clone().unwrap_or_else(|| "unknown".to_string()),
                address: sym.st_value,
            }
        })
        .collect()
}
