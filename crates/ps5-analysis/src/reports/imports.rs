use crate::dataset::AnalysisDataset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInventoryEntry {
    pub library: String,
    pub games: usize,
    pub imports: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInventory {
    pub entries: Vec<LibraryInventoryEntry>,
    pub total_games: usize,
}

pub fn build_import_inventory(ds: &AnalysisDataset) -> LibraryInventory {
    let mut lib_games: std::collections::HashMap<String, std::collections::HashSet<usize>> =
        std::collections::HashMap::new();
    let mut lib_imports: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (i, (_, doc)) in ds.images.iter().enumerate() {
        for imp in &doc.image.imports {
            lib_games
                .entry(imp.library_name.clone())
                .or_default()
                .insert(i);
            *lib_imports.entry(imp.library_name.clone()).or_insert(0) += 1;
        }
    }

    let mut entries: Vec<LibraryInventoryEntry> = lib_games
        .keys()
        .map(|lib| LibraryInventoryEntry {
            library: lib.clone(),
            games: lib_games[lib].len(),
            imports: lib_imports[lib],
        })
        .collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.imports));

    LibraryInventory {
        entries,
        total_games: ds.images.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_image::{BinaryImage, BinaryImageDocument, ImportEntry, Platform};

    fn make_doc_with_imports(imports: Vec<ImportEntry>) -> BinaryImageDocument {
        BinaryImageDocument {
            schema_version: 1,
            tool: "test".into(),
            image: BinaryImage {
                sha256: "aa".repeat(32),
                platform: Platform::Ps5,
                is_self: true,
                file_size: 100,
                entry_point: 0x80000000,
                segments: vec![],
                imports,
                exports: vec![],
                relocations: vec![],
                tls: None,
                init_va: 0,
                init_array_va: 0,
                init_array_sz: 0,
                fini_va: 0,
                fini_array_va: 0,
                fini_array_sz: 0,
                preinit_array_va: 0,
                preinit_array_sz: 0,
                import_libs: std::collections::HashMap::new(),
                needed_files: vec![],
                metadata: ps5_image::BinaryMetadata::default(),
                dynamic_entries: vec![],
                version_defs: vec![],
            },
        }
    }

    fn tempdir_for_test(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ps5rs_imports_test_{label}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn make_dataset(root: &std::path::Path, docs: Vec<BinaryImageDocument>) {
        std::fs::create_dir_all(root.join("images")).unwrap();
        let manifest = crate::dataset::Manifest {
            schema_version: crate::dataset::DATASET_SCHEMA_VERSION,
            tool: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            image_count: docs.len(),
        };
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        for (i, doc) in docs.into_iter().enumerate() {
            let json = serde_json::to_string_pretty(&doc).unwrap();
            std::fs::write(root.join("images").join(format!("game{i}.json")), json).unwrap();
        }
    }

    #[test]
    fn inventory_empty() {
        let root = tempdir_for_test("empty");
        make_dataset(&root, vec![]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let inv = build_import_inventory(&ds);
        assert!(inv.entries.is_empty());
        assert_eq!(inv.total_games, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inventory_single_library() {
        let root = tempdir_for_test("single_lib");
        let doc = make_doc_with_imports(vec![
            ImportEntry {
                nid_hash: "a".into(),
                resolved_name: Some("fA".into()),
                library_id: 1,
                library_name: "libkernel".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
            ImportEntry {
                nid_hash: "b".into(),
                resolved_name: Some("fB".into()),
                library_id: 1,
                library_name: "libkernel".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
        ]);
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let inv = build_import_inventory(&ds);
        assert_eq!(inv.entries.len(), 1);
        assert_eq!(inv.entries[0].library, "libkernel");
        assert_eq!(inv.entries[0].games, 1);
        assert_eq!(inv.entries[0].imports, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inventory_multi_game_multi_lib() {
        let root = tempdir_for_test("multi");
        let doc1 = make_doc_with_imports(vec![
            ImportEntry {
                nid_hash: "a".into(),
                resolved_name: None,
                library_id: 1,
                library_name: "libA".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
            ImportEntry {
                nid_hash: "b".into(),
                resolved_name: None,
                library_id: 2,
                library_name: "libB".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
        ]);
        let doc2 = make_doc_with_imports(vec![
            ImportEntry {
                nid_hash: "a".into(),
                resolved_name: None,
                library_id: 1,
                library_name: "libA".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
        ]);
        make_dataset(&root, vec![doc1, doc2]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let inv = build_import_inventory(&ds);
        assert_eq!(inv.entries.len(), 2);
        assert_eq!(inv.total_games, 2);

        let lib_a = inv.entries.iter().find(|e| e.library == "libA").unwrap();
        assert_eq!(lib_a.games, 2);
        assert_eq!(lib_a.imports, 2);

        let lib_b = inv.entries.iter().find(|e| e.library == "libB").unwrap();
        assert_eq!(lib_b.games, 1);
        assert_eq!(lib_b.imports, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inventory_sorted_by_imports_desc() {
        let root = tempdir_for_test("sorted");
        let doc = make_doc_with_imports(vec![
            ImportEntry {
                nid_hash: "a".into(),
                resolved_name: None,
                library_id: 1,
                library_name: "libSmall".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
            ImportEntry {
                nid_hash: "b".into(),
                resolved_name: None,
                library_id: 2,
                library_name: "libBig".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
            ImportEntry {
                nid_hash: "c".into(),
                resolved_name: None,
                library_id: 2,
                library_name: "libBig".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
            ImportEntry {
                nid_hash: "d".into(),
                resolved_name: None,
                library_id: 2,
                library_name: "libBig".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            },
        ]);
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let inv = build_import_inventory(&ds);
        assert_eq!(inv.entries[0].library, "libBig");
        assert_eq!(inv.entries[0].imports, 3);
        assert_eq!(inv.entries[1].library, "libSmall");
        assert_eq!(inv.entries[1].imports, 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
