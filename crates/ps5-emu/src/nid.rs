//! Readable NID → symbol-name resolution backed by the workspace catalog CSV.

use ps5_nid::Catalog;

const NIDS_CSV: &str = include_str!("../../../data/nids.csv");

/// Build a catalog from the workspace `data/nids.csv`, seeded with builtins.
pub fn catalog() -> Catalog {
    let mut cat = Catalog::new();
    cat.load_nids_csv(NIDS_CSV);
    cat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_resolves_sample_nids() {
        let cat = catalog();
        assert!(cat.resolve("To9mmGL+3G8").is_some());
        assert!(cat.resolve("OaQI1HqFAtk").is_some());
        assert!(cat.resolve("YQ0navp+YIc").is_some());
    }

    #[test]
    fn catalog_resolves_names() {
        let cat = catalog();
        let entry = cat
            .resolve("To9mmGL+3G8")
            .expect("sceDbgLoggingHandler NID");
        assert_eq!(entry.primary_name(), Some("sceDbgLoggingHandler"));
    }
}
