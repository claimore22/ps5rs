use crate::dataset::AnalysisDataset;
use ps5_image::Detection;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_libs: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_system: Option<Detection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depot: Option<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk_hints: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_versions: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_paths: Vec<Detection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_forks: Vec<Detection>,
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

    // ELF-based SCE library detection
    let mut sce_libraries: Vec<String> = all_libs
        .iter()
        .filter(|l| l.starts_with("libSce"))
        .cloned()
        .collect();
    sce_libraries.sort();

    // ELF-based engine detection
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

    // String-based analysis — enriches ELF data, fills gaps for encrypted eboots
    let string_analysis = doc.string_analysis.as_ref();

    if let Some(sa) = string_analysis {
        // Merge string-based engine detection if ELF found nothing
        if engines.is_empty()
            && let Some(ref engine) = sa.engine
        {
            engines.push(engine.value.clone());
        }

        // Merge SCE libraries from strings
        for lib in &sa.sce_libraries {
            if !sce_libraries.contains(lib) {
                sce_libraries.push(lib.clone());
            }
        }
        sce_libraries.sort();
    }

    if engines.is_empty() {
        engines.push("Native/SCE".to_string());
    }

    let unreal = unreal
        || string_analysis
            .and_then(|sa| sa.engine.as_ref())
            .is_some_and(|e| e.value.contains("Unreal"));
    let unity = unity
        || string_analysis
            .and_then(|sa| sa.engine.as_ref())
            .is_some_and(|e| e.value == "Unity");
    let godot = godot
        || string_analysis
            .and_then(|sa| sa.engine.as_ref())
            .is_some_and(|e| e.value == "Godot");

    EngineHint {
        name: name.to_string(),
        display_name: None,
        unreal,
        unity,
        godot,
        engines,
        sce_libraries,
        third_party_libs: string_analysis
            .map(|sa| sa.third_party_libs.clone())
            .unwrap_or_default(),
        build_system: string_analysis.and_then(|sa| sa.build_system.clone()),
        source_depot: string_analysis.and_then(|sa| sa.source_depot.clone()),
        sdk_hints: string_analysis
            .map(|sa| sa.sdk_hints.clone())
            .unwrap_or_default(),
        detected_versions: string_analysis
            .map(|sa| sa.detected_versions.clone())
            .unwrap_or_default(),
        source_paths: string_analysis
            .map(|sa| sa.source_paths.clone())
            .unwrap_or_default(),
        project_paths: string_analysis
            .map(|sa| sa.project_paths.clone())
            .unwrap_or_default(),
        custom_forks: string_analysis
            .map(|sa| sa.custom_forks.clone())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{DATASET_SCHEMA_VERSION, Manifest};
    use ps5_image::{
        BinaryImage, BinaryImageDocument, Detection, LoadedSegment, Platform, SegmentType,
        StringAnalysis,
    };

    fn make_image_with_libs(name: &str, libs: Vec<String>) -> (String, BinaryImageDocument) {
        make_image_with_libs_and_strings(name, libs, None)
    }

    fn make_image_with_libs_and_strings(
        name: &str,
        libs: Vec<String>,
        string_analysis: Option<StringAnalysis>,
    ) -> (String, BinaryImageDocument) {
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
                image_type: ps5_image::ImageType::Eboot,
                parent_image: None,
                image,
                string_analysis,
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
    fn string_engine_fills_elf_gap() {
        let sa = StringAnalysis {
            engine: Some(Detection {
                value: "Unreal Engine 4".to_string(),
                score: 0,
                confidence: 0,
                evidence: vec!["UnrealEngine4Runtime".to_string()],
            }),
            ..Default::default()
        };
        let ds = test_dataset(vec![make_image_with_libs_and_strings(
            "EncryptedGame",
            vec!["libkernel".into()],
            Some(sa),
        )]);
        let report = build_engine_hints(&ds);
        assert!(report.games[0].unreal);
        assert!(
            report.games[0]
                .engines
                .contains(&"Unreal Engine 4".to_string())
        );
    }

    #[test]
    fn string_build_system_detected() {
        let sa = StringAnalysis {
            build_system: Some(Detection {
                value: "Jenkins".to_string(),
                score: 0,
                confidence: 0,
                evidence: vec!["X:/Jenkins/sharedspace/".to_string()],
            }),
            ..Default::default()
        };
        let ds = test_dataset(vec![make_image_with_libs_and_strings(
            "GameA",
            vec!["libkernel".into()],
            Some(sa),
        )]);
        let report = build_engine_hints(&ds);
        assert!(report.games[0].build_system.is_some());
        assert_eq!(
            report.games[0].build_system.as_ref().unwrap().value,
            "Jenkins"
        );
    }

    #[test]
    fn string_third_party_detected() {
        let sa = StringAnalysis {
            third_party_libs: vec![Detection {
                value: "PhysX".to_string(),
                score: 0,
                confidence: 0,
                evidence: vec!["PhysX 3.4".to_string()],
            }],
            ..Default::default()
        };
        let ds = test_dataset(vec![make_image_with_libs_and_strings(
            "GameA",
            vec!["libkernel".into()],
            Some(sa),
        )]);
        let report = build_engine_hints(&ds);
        assert_eq!(report.games[0].third_party_libs.len(), 1);
        assert_eq!(report.games[0].third_party_libs[0].value, "PhysX");
    }

    #[test]
    fn sce_libraries_merge_elf_and_string() {
        let sa = StringAnalysis {
            sce_libraries: vec![
                "libScePad.prx".to_string(),
                "libSceAgcDriver.prx".to_string(),
            ],
            ..Default::default()
        };
        let ds = test_dataset(vec![make_image_with_libs_and_strings(
            "GameA",
            vec!["libScePad".into(), "libkernel".into()],
            Some(sa),
        )]);
        let report = build_engine_hints(&ds);
        let sce = &report.games[0].sce_libraries;
        // ELF gives "libScePad", strings gives "libScePad.prx" — both present
        assert!(sce.contains(&"libScePad".to_string()));
        assert!(sce.contains(&"libScePad.prx".to_string()));
        assert!(sce.contains(&"libSceAgcDriver.prx".to_string()));
    }

    #[test]
    fn backward_compat_no_string_analysis() {
        let ds = test_dataset(vec![make_image_with_libs(
            "GameA",
            vec!["libScePad".into()],
        )]);
        let report = build_engine_hints(&ds);
        assert!(report.games[0].third_party_libs.is_empty());
        assert!(report.games[0].build_system.is_none());
        assert!(report.games[0].source_depot.is_none());
        assert!(report.games[0].sdk_hints.is_empty());
        assert!(report.games[0].detected_versions.is_empty());
        assert!(report.games[0].source_paths.is_empty());
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
