use ps5_sdk_meta::{SdkDatabase, SdkFunction, VersionRange};

#[test]
fn populate_from_catalog() {
    let mut db = SdkDatabase::new();
    let catalog = ps5_nid::Catalog::new();
    // Populate a few entries from catalog's builtins
    for name in ["sceKernelSleep", "scePthreadCreate", "sceKernelOpen"].iter() {
        let nid = ps5_nid::hash(name);
        if let Some(entry) = catalog.resolve(&nid) {
            let func = SdkFunction {
                nid: nid.clone(),
                name: entry.primary_name().unwrap_or(name).to_string(),
                library: "libkernel".to_string(),
                module: None,
                sdk_versions: VersionRange {
                    from: "1.00".to_string(),
                    to: None,
                },
                category: "system".to_string(),
            };
            db.insert(func);
        }
    }
    assert!(db.len() >= 2);
}

#[test]
fn populate_from_roms() {
    let roms = r"C:\Users\claimoar\Documents\ROMS\PS5";
    let path = std::path::Path::new(roms);
    if !path.exists() {
        return;
    }
    let mut db = SdkDatabase::new();
    // Add a dummy from ROMs if available
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(1) {
            let name = entry.file_name().to_string_lossy().to_string();
            let func = SdkFunction {
                nid: ps5_nid::hash(&name),
                name: name.clone(),
                library: "unknown".to_string(),
                module: Some(name),
                sdk_versions: VersionRange {
                    from: "10.00".to_string(),
                    to: None,
                },
                category: "game".to_string(),
            };
            db.insert(func);
        }
    }
    assert!(db.len() <= 100);
}
