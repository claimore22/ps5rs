use crate::dataset::AnalysisDataset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHintReport {
    pub games: Vec<EngineHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHint {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub unreal: bool,
    pub unity: bool,
    pub godot: bool,
    pub engines: Vec<String>,
    pub sce_libraries: Vec<String>,
}

pub fn build_engine_hints(ds: &AnalysisDataset) -> EngineHintReport {
    let games = ds
        .images
        .iter()
        .map(|(name, doc)| {
            let mut hint = analyze_engine(name, doc);
            hint.display_name = Some(ds.display_name_for(name).to_string());
            hint
        })
        .collect();
    EngineHintReport { games }
}

fn analyze_engine(name: &str, doc: &ps5_image::BinaryImageDocument) -> EngineHint {
    let img = &doc.image;
    let lib_names: Vec<String> = img.import_libs.values().cloned().collect();
    let all_libs: Vec<String> = img
        .imports
        .iter()
        .map(|i| i.library_name.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut sce_libraries: Vec<String> = all_libs
        .iter()
        .filter(|l| l.starts_with("libSce") || l.starts_with("libSce"))
        .cloned()
        .collect();
    sce_libraries.sort();

    let mut engines = Vec::new();

    let unreal = {
        let lib_match = all_libs
            .iter()
            .any(|l| l.contains("Unreal") || l.contains("UE4") || l.contains("UE5"));
        let name_match = lib_names
            .iter()
            .any(|l| l.contains("Unreal") || l.contains("UE4") || l.contains("UE5"));
        lib_match || name_match
    };
    if unreal {
        engines.push("Unreal Engine".to_string());
    }

    let unity = all_libs
        .iter()
        .any(|l| l.contains("Unity") || l.contains("UnityEngine") || l.contains("UnityMain"))
        || lib_names
            .iter()
            .any(|l| l.contains("Unity") || l.contains("UnityEngine"));
    if unity {
        engines.push("Unity".to_string());
    }

    let godot = all_libs.iter().any(|l| l.contains("Godot"))
        || lib_names.iter().any(|l| l.contains("Godot"));
    if godot {
        engines.push("Godot".to_string());
    }

    if engines.is_empty() {
        engines.push("Native/SCE".to_string());
    }

    EngineHint {
        name: name.to_string(),
        display_name: None,
        unreal,
        unity,
        godot,
        engines,
        sce_libraries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{DATASET_SCHEMA_VERSION, Manifest};
    use ps5_image::{BinaryImage, BinaryImageDocument, LoadedSegment, Platform, SegmentType};

    fn make_image_with_libs(name: &str, libs: Vec<String>) -> (String, BinaryImageDocument) {
        let mut import_libs = std::collections::HashMap::new();
        for (i, lib) in libs.iter().enumerate() {
            import_libs.insert(i as u16, lib.clone());
        }
        let imports = libs
            .iter()
            .enumerate()
            .flat_map(|(i, lib)| {
                (0..2).map(move |j| ps5_image::ImportEntry {
                    nid_hash: format!("nid_{i}_{j}"),
                    resolved_name: Some(format!("func_{i}_{j}")),
                    library_id: i as u16,
                    library_name: lib.clone(),
                    value: 0,
                    size: 0,
                    shndx: 1,
                    binding: ps5_image::SymbolBinding::Global,
                    sym_type: ps5_image::SymbolType::Func,
                    visibility: ps5_image::SymbolVisibility::Default,
                    ordinal: (i * 2 + j) as u32,
                })
            })
            .collect();

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
            import_libs,
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions: vec![],
        };

        (
            name.to_string(),
            BinaryImageDocument {
                schema_version: DATASET_SCHEMA_VERSION,
                tool: "ps5rs-test".to_string(),
                image,
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
                games: vec![],
            },
            images,
            display_names: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn native_game_no_engine() {
        let ds = test_dataset(vec![make_image_with_libs(
            "GameA",
            vec!["libScePad".into(), "libSceKernel".into()],
        )]);
        let report = build_engine_hints(&ds);
        assert_eq!(report.games.len(), 1);
        assert!(!report.games[0].unreal);
        assert!(!report.games[0].unity);
        assert!(report.games[0].engines.contains(&"Native/SCE".to_string()));
    }

    #[test]
    fn unreal_engine_detected() {
        let ds = test_dataset(vec![make_image_with_libs(
            "UnrealGame",
            vec!["libUnrealEngine".into(), "libScePad".into()],
        )]);
        let report = build_engine_hints(&ds);
        assert!(report.games[0].unreal);
        assert!(!report.games[0].unity);
        assert!(
            report.games[0]
                .engines
                .contains(&"Unreal Engine".to_string())
        );
    }

    #[test]
    fn unity_engine_detected() {
        let ds = test_dataset(vec![make_image_with_libs(
            "UnityGame",
            vec!["UnityEngine".into(), "libScePad".into()],
        )]);
        let report = build_engine_hints(&ds);
        assert!(!report.games[0].unreal);
        assert!(report.games[0].unity);
        assert!(report.games[0].engines.contains(&"Unity".to_string()));
    }

    #[test]
    fn sce_libraries_collected() {
        let ds = test_dataset(vec![make_image_with_libs(
            "GameA",
            vec![
                "libScePad".into(),
                "libSceGnmDriver".into(),
                "libkernel".into(),
            ],
        )]);
        let report = build_engine_hints(&ds);
        let sce = &report.games[0].sce_libraries;
        assert!(sce.contains(&"libScePad".to_string()));
        assert!(sce.contains(&"libSceGnmDriver".to_string()));
    }

    #[test]
    fn serde_roundtrip() {
        let ds = test_dataset(vec![make_image_with_libs(
            "GameA",
            vec!["libScePad".into()],
        )]);
        let report = build_engine_hints(&ds);
        let json = serde_json::to_string_pretty(&report).unwrap();
        let back: EngineHintReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.games.len(), 1);
        assert_eq!(back.games[0].name, "GameA");
    }
}
