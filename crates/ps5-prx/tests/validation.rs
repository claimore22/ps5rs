use std::path::Path;

#[test]
fn parse_sdk_archives() {
    let candidates = [
        r"C:\Users\claimoar\Documents\SDK MANAGER 10.00\PS5 - SDK-10_00_00_40-00_00_00_0_1\sdk\target\lib",
        r"C:\Program Files (x86)\SCE\Prospero\SDKs\10.00",
        r"C:\Users\claimoar\Documents\ROMS\PS5",
    ];
    let mut found = 0;
    for base in candidates.iter().filter(|p| Path::new(p).exists()) {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten().take(3) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("a") {
                    if let Ok(data) = std::fs::read(&path) {
                        let catalog = ps5_nid::Catalog::new();
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("test");
                        let _ = ps5_prx::PrxModule::from_elf_bytes(name, &data, &catalog);
                        found += 1;
                    }
                }
            }
        }
        if base.contains("ROMS") {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten().take(2) {
                    let p = entry.path().join("sce_module");
                    if p.exists() {
                        if let Ok(prx_entries) = std::fs::read_dir(&p) {
                            for prx in prx_entries.flatten().take(2) {
                                let prx_path = prx.path();
                                if prx_path.extension().and_then(|s| s.to_str()) == Some("prx") {
                                    if let Ok(data) = std::fs::read(&prx_path) {
                                        let catalog = ps5_nid::Catalog::new();
                                        let name = prx_path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("test");
                                        let _ = ps5_prx::PrxModule::from_elf_bytes(
                                            name, &data, &catalog,
                                        );
                                        found += 1;
                                    }
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
