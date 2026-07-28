use std::path::PathBuf;

pub(crate) const NIDS_CSV: &str = include_str!("../../../data/nids.csv");

pub(crate) fn load_catalog(extra_nids: &[PathBuf]) -> ps5_nid::Catalog {
    let mut cat = ps5_nid::Catalog::new();
    let loaded = cat.load_nids_csv(NIDS_CSV);
    eprintln!("Loaded {} NID mappings from built-in catalog", loaded);

    for path in extra_nids {
        match cat.load_nids_csv_file(path) {
            Ok(n) => eprintln!("Loaded {} NID mappings from {}", n, path.display()),
            Err(e) => {
                eprintln!("error: cannot load {}: {e}", path.display());
                std::process::exit(1);
            }
        }
    }
    cat
}
