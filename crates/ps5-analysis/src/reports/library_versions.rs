use crate::dataset::AnalysisDataset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryVersionReport {
    pub entries: Vec<LibraryVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryVersionEntry {
    pub library: String,
    pub version_raw: u32,
    pub version_string: String,
    pub game_count: usize,
    pub games: Vec<String>,
}

pub fn build_library_versions(ds: &AnalysisDataset) -> LibraryVersionReport {
    let mut map: HashMap<(String, u32), LibraryVersionEntry> = HashMap::new();

    for (name, doc) in &ds.images {
        let display = ds.display_name_for(name).to_string();
        for lv in &doc.image.lib_versions {
            let key = (lv.name.clone(), lv.version_raw);
            let entry = map.entry(key).or_insert_with(|| LibraryVersionEntry {
                library: lv.name.clone(),
                version_raw: lv.version_raw,
                version_string: lv.version_string.clone(),
                game_count: 0,
                games: Vec::new(),
            });
            if !entry.games.contains(&display) {
                entry.games.push(display.clone());
                entry.game_count = entry.games.len();
            }
        }
    }

    let mut entries: Vec<LibraryVersionEntry> = map.into_values().collect();
    entries.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(a.library.cmp(&b.library))
    });

    LibraryVersionReport { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{DATASET_SCHEMA_VERSION, Manifest};
    use ps5_image::{
        BinaryImage, BinaryImageDocument, LibVersionEntry, LoadedSegment, Platform, SegmentType,
    };

    fn make_image_with_lib_versions(
        name: &str,
        lib_versions: Vec<LibVersionEntry>,
    ) -> (String, BinaryImageDocument) {
        let image = BinaryImage {
            sha256: format!("sha256_{name}"),
            platform: Platform::Ps5,
            is_self: true,
            file_size: 1024,
            entry_point: 0x80000000,
            metadata: Default::default(),
            segments: vec![LoadedSegment {
                vaddr: 0x100000,
                file_offset: 0x1000,
                filesz: 0x1000,
                memsz: 0x1000,
                is_executable: true,
                is_writable: false,
                seg_type: SegmentType::Load,
                p_paddr: 0,
                p_align: 0x1000,
                is_encrypted: false,
                is_compressed: false,
                phdr_index: Some(0),
            }],
            imports: vec![],
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
            import_libs: Default::default(),
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions,
        };
        (
            name.to_string(),
            BinaryImageDocument {
                schema_version: DATASET_SCHEMA_VERSION,
                tool: "ps5rs-test".to_string(),
                image_type: ps5_image::ImageType::Eboot,
                parent_image: None,
                image,
                string_analysis: None,
            },
        )
    }

    fn test_dataset(images: Vec<(String, BinaryImageDocument)>) -> AnalysisDataset {
        AnalysisDataset {
            manifest: Manifest {
                schema_version: DATASET_SCHEMA_VERSION,
                tool: "ps5rs-test".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                image_count: images.len(),
                module_count: 0,
                games: vec![],
            },
            images,
            display_names: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn empty_dataset_no_versions() {
        let ds = test_dataset(vec![]);
        let report = build_library_versions(&ds);
        assert!(report.entries.is_empty());
    }

    #[test]
    fn single_game_single_library() {
        let ds = test_dataset(vec![make_image_with_lib_versions(
            "GameA",
            vec![LibVersionEntry {
                name: "libScePad".to_string(),
                version_raw: 0x01000000,
                version_string: "1.0.0".to_string(),
            }],
        )]);
        let report = build_library_versions(&ds);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].library, "libScePad");
        assert_eq!(report.entries[0].game_count, 1);
        assert_eq!(report.entries[0].games, vec!["GameA"]);
    }

    #[test]
    fn multiple_games_shared_library() {
        let ds = test_dataset(vec![
            make_image_with_lib_versions(
                "GameA",
                vec![LibVersionEntry {
                    name: "libScePad".to_string(),
                    version_raw: 0x01000000,
                    version_string: "1.0.0".to_string(),
                }],
            ),
            make_image_with_lib_versions(
                "GameB",
                vec![LibVersionEntry {
                    name: "libScePad".to_string(),
                    version_raw: 0x01000000,
                    version_string: "1.0.0".to_string(),
                }],
            ),
            make_image_with_lib_versions(
                "GameC",
                vec![LibVersionEntry {
                    name: "libScePad".to_string(),
                    version_raw: 0x02000000,
                    version_string: "2.0.0".to_string(),
                }],
            ),
        ]);
        let report = build_library_versions(&ds);
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].game_count, 2);
        assert_eq!(report.entries[0].version_string, "1.0.0");
        assert_eq!(report.entries[1].game_count, 1);
        assert_eq!(report.entries[1].version_string, "2.0.0");
    }

    #[test]
    fn sorted_by_game_count_desc() {
        let ds = test_dataset(vec![
            make_image_with_lib_versions(
                "GameA",
                vec![
                    LibVersionEntry {
                        name: "libSceCommon".to_string(),
                        version_raw: 0x01000000,
                        version_string: "1.0.0".to_string(),
                    },
                    LibVersionEntry {
                        name: "libScePad".to_string(),
                        version_raw: 0x01000000,
                        version_string: "1.0.0".to_string(),
                    },
                ],
            ),
            make_image_with_lib_versions(
                "GameB",
                vec![LibVersionEntry {
                    name: "libSceCommon".to_string(),
                    version_raw: 0x01000000,
                    version_string: "1.0.0".to_string(),
                }],
            ),
        ]);
        let report = build_library_versions(&ds);
        assert_eq!(report.entries[0].library, "libSceCommon");
        assert_eq!(report.entries[0].game_count, 2);
        assert_eq!(report.entries[1].library, "libScePad");
        assert_eq!(report.entries[1].game_count, 1);
    }

    #[test]
    fn no_duplicate_games() {
        let ds = test_dataset(vec![make_image_with_lib_versions(
            "GameA",
            vec![LibVersionEntry {
                name: "libScePad".to_string(),
                version_raw: 0x01000000,
                version_string: "1.0.0".to_string(),
            }],
        )]);
        let report = build_library_versions(&ds);
        assert_eq!(report.entries[0].games.len(), 1);
        assert_eq!(report.entries[0].game_count, 1);
    }

    #[test]
    fn serde_roundtrip() {
        let ds = test_dataset(vec![make_image_with_lib_versions(
            "GameA",
            vec![LibVersionEntry {
                name: "libScePad".to_string(),
                version_raw: 0x01000000,
                version_string: "1.0.0".to_string(),
            }],
        )]);
        let report = build_library_versions(&ds);
        let json = serde_json::to_string_pretty(&report).unwrap();
        let back: LibraryVersionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].library, "libScePad");
    }
}
