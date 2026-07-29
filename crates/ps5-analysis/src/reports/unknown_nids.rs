use crate::dataset::AnalysisDataset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownNidEntry {
    pub nid_hash: String,
    pub count: usize,
    pub libraries: Vec<String>,
    pub games: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownNidReport {
    pub entries: Vec<UnknownNidEntry>,
    pub total_unknown: usize,
    pub total_imports: usize,
}

pub fn build_unknown_nids(ds: &AnalysisDataset) -> UnknownNidReport {
    let mut nid_data: HashMap<
        String,
        (
            usize,
            std::collections::HashSet<String>,
            std::collections::HashSet<String>,
        ),
    > = HashMap::new();
    let mut total_unknown = 0usize;
    let total_imports = ds.total_imports();

    for (name, doc) in &ds.images {
        for imp in &doc.image.imports {
            if imp.resolved_name.is_none() {
                total_unknown += 1;
                let entry = nid_data.entry(imp.nid_hash.clone()).or_insert_with(|| {
                    (
                        0,
                        std::collections::HashSet::new(),
                        std::collections::HashSet::new(),
                    )
                });
                entry.0 += 1;
                entry.1.insert(imp.library_name.clone());
                entry.2.insert(name.clone());
            }
        }
    }

    let mut entries: Vec<UnknownNidEntry> = nid_data
        .into_iter()
        .map(|(nid_hash, (count, libs, games))| {
            let mut libraries: Vec<String> = libs.into_iter().collect();
            libraries.sort();
            let mut games: Vec<String> = games.into_iter().collect();
            games.sort();
            games.dedup();
            UnknownNidEntry {
                nid_hash,
                count,
                libraries,
                games,
            }
        })
        .collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.count));

    UnknownNidReport {
        entries,
        total_unknown,
        total_imports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_image::{BinaryImage, BinaryImageDocument, ImportEntry, Platform};

    fn make_doc(imports: Vec<ImportEntry>, sha_suffix: &str) -> BinaryImageDocument {
        BinaryImageDocument {
            schema_version: 1,
            tool: "test".into(),
            image_type: ps5_image::ImageType::Eboot,
            parent_image: None,
            string_analysis: None,
            image: BinaryImage {
                sha256: sha_suffix.repeat(32),
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
                lib_versions: vec![],
            },
        }
    }

    fn tempdir_for_test(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ps5rs_unknown_test_{label}_{}", std::process::id()));
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
            module_count: 0,
            games: vec![],
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
    fn unknown_empty_when_all_resolved() {
        let root = tempdir_for_test("all_resolved");
        let doc = make_doc(
            vec![ImportEntry {
                nid_hash: "aaa".into(),
                resolved_name: Some("funcA".into()),
                library_id: 1,
                library_name: "libA".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            }],
            "a",
        );
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert!(report.entries.is_empty());
        assert_eq!(report.total_unknown, 0);
        assert_eq!(report.total_imports, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_single_nid() {
        let root = tempdir_for_test("single_nid");
        let doc = make_doc(
            vec![ImportEntry {
                nid_hash: "deadbeef".into(),
                resolved_name: None,
                library_id: 1,
                library_name: "libkernel".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            }],
            "a",
        );
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].nid_hash, "deadbeef");
        assert_eq!(report.entries[0].count, 1);
        assert_eq!(report.entries[0].libraries, vec!["libkernel"]);
        assert_eq!(report.entries[0].games, vec!["game0"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_aggregates_across_games() {
        let root = tempdir_for_test("cross_game");
        let doc1 = make_doc(
            vec![
                ImportEntry {
                    nid_hash: "aaa".into(),
                    resolved_name: None,
                    library_id: 1,
                    library_name: "libX".into(),
                    value: 0,
                    size: 0,
                    shndx: 0,
                    binding: ps5_image::SymbolBinding::Global,
                    sym_type: ps5_image::SymbolType::Func,
                    visibility: ps5_image::SymbolVisibility::Default,
                    ordinal: 0,
                },
                ImportEntry {
                    nid_hash: "aaa".into(),
                    resolved_name: None,
                    library_id: 2,
                    library_name: "libY".into(),
                    value: 0,
                    size: 0,
                    shndx: 0,
                    binding: ps5_image::SymbolBinding::Global,
                    sym_type: ps5_image::SymbolType::Func,
                    visibility: ps5_image::SymbolVisibility::Default,
                    ordinal: 0,
                },
            ],
            "a",
        );
        let doc2 = make_doc(
            vec![ImportEntry {
                nid_hash: "aaa".into(),
                resolved_name: None,
                library_id: 1,
                library_name: "libX".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            }],
            "b",
        );
        make_dataset(&root, vec![doc1, doc2]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].count, 3);
        assert_eq!(report.entries[0].libraries, vec!["libX", "libY"]);
        assert_eq!(report.entries[0].games, vec!["game0", "game1"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_sorted_by_count_desc() {
        let root = tempdir_for_test("sorted");
        let doc = make_doc(
            vec![
                ImportEntry {
                    nid_hash: "rare".into(),
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
                    nid_hash: "common".into(),
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
                    nid_hash: "common".into(),
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
                ImportEntry {
                    nid_hash: "common".into(),
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
            ],
            "a",
        );
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert_eq!(report.entries[0].nid_hash, "common");
        assert_eq!(report.entries[0].count, 3);
        assert_eq!(report.entries[0].games, vec!["game0"]);
        assert_eq!(report.entries[1].nid_hash, "rare");
        assert_eq!(report.entries[1].count, 1);
        assert_eq!(report.entries[1].games, vec!["game0"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_deduplicates_games_within_single_title() {
        let root = tempdir_for_test("dedup_games");
        let doc = make_doc(
            vec![
                ImportEntry {
                    nid_hash: "dup".into(),
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
                    nid_hash: "dup".into(),
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
            ],
            "a",
        );
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].count, 2);
        assert_eq!(report.entries[0].games, vec!["game0"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_mixed_resolved_and_unresolved() {
        let root = tempdir_for_test("mixed");
        let doc = make_doc(
            vec![
                ImportEntry {
                    nid_hash: "known".into(),
                    resolved_name: Some("sceFunc".into()),
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
                    nid_hash: "unknown_hash".into(),
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
            ],
            "a",
        );
        make_dataset(&root, vec![doc]);
        let ds = AnalysisDataset::open(&root).unwrap();
        let report = build_unknown_nids(&ds);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].nid_hash, "unknown_hash");
        assert_eq!(report.entries[0].games, vec!["game0"]);
        assert_eq!(report.total_unknown, 1);
        assert_eq!(report.total_imports, 2);
        let _ = std::fs::remove_dir_all(&root);
    }
}
