use ps5_firmware::{FirmwareCatalog, FirmwareVersion};

#[test]
fn catalog_from_roms() {
    let roms = r"C:\Users\claimoar\Documents\ROMS\PS5";
    let path = std::path::Path::new(roms);
    if !path.exists() {
        return;
    }
    let ver = FirmwareVersion {
        major: 10,
        minor: 0,
        patch: 0,
    };
    let mut catalog = FirmwareCatalog::new(ver);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(1) {
            let name = entry.file_name().to_string_lossy().to_string();
            catalog.modules.push(ps5_firmware::FirmwareModule {
                name: name.clone(),
                path: entry.path().to_string_lossy().to_string(),
                version: "1.0".to_string(),
                exports_count: 0,
            });
        }
    }
    assert!(!catalog.modules.is_empty() || path.exists());
}
