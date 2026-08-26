use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    pub nid: String,
    pub name: Option<String>,
    pub library: String,
    pub is_system: bool,
}

pub fn extract_imports(elf: &ps5_elf::ElfImage, catalog: &ps5_nid::Catalog) -> Vec<ImportEntry> {
    elf.symbols
        .iter()
        .filter(|s| s.is_import)
        .map(|sym| {
            let nid = sym
                .resolved_name
                .split('#')
                .next()
                .unwrap_or("")
                .to_string();
            let lib_id = ps5_nid::lib_id_from_nid(&sym.resolved_name).unwrap_or(0);
            let lib_name = elf
                .import_libs
                .get(&lib_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let name = catalog
                .resolve(&nid)
                .and_then(|e| e.primary_name().map(|s| s.to_string()));
            ImportEntry {
                nid,
                name,
                library: lib_name.clone(),
                is_system: lib_name.starts_with("libSce") || lib_name.starts_with("libkernel"),
            }
        })
        .collect()
}
