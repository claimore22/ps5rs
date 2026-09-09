use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    pub is_system: bool,
}

pub fn extract_dependencies(elf: &ps5_elf::ElfImage, module_name: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();
    for needed in &elf.needed_files {
        deps.push(Dependency {
            from: module_name.to_string(),
            to: needed.clone(),
            is_system: needed.starts_with("libSce") || needed.starts_with("libkernel"),
        });
    }
    for lib in elf.import_libs.values() {
        if !elf.needed_files.contains(lib) {
            deps.push(Dependency {
                from: module_name.to_string(),
                to: lib.clone(),
                is_system: true,
            });
        }
    }
    deps
}
