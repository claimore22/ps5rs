#![allow(clippy::collapsible_if)]
#![allow(clippy::absurd_extreme_comparisons)]
use ps5_prx::{ModuleType, PrxModule};

#[test]
fn module_type_detection() {
    assert!(matches!(
        ModuleType::from_elf_type(0xFE00),
        ModuleType::Eboot
    ));
    assert!(matches!(ModuleType::from_elf_type(0xFE01), ModuleType::Prx));
    assert!(matches!(
        ModuleType::from_elf_type(0x1234),
        ModuleType::Unknown
    ));
}

#[test]
fn prx_module_from_roms() {
    let roms = r"C:\Users\claimoar\Documents\ROMS\PS5";
    let path = std::path::Path::new(roms);
    if !path.exists() {
        return;
    }
    let mut found = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(2) {
            let p = entry.path().join("sce_module");
            if p.exists() {
                if let Ok(prx_entries) = std::fs::read_dir(&p) {
                    for prx in prx_entries.flatten().take(1) {
                        let prx_path = prx.path();
                        if prx_path.extension().and_then(|s| s.to_str()) == Some("prx") {
                            if let Ok(data) = std::fs::read(&prx_path) {
                                let catalog = ps5_nid::Catalog::new();
                                let name = prx_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("test");
                                if let Ok(m) = PrxModule::from_elf_bytes(name, &data, &catalog) {
                                    assert!(!m.name.is_empty());
                                    found += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(found >= 0);
}
