use crate::dataset::AnalysisDataset;
use crate::scanner::utc_now_iso8601;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub dataset_path: String,
    pub generated_at: String,
    pub tool: String,
    pub total_games: usize,
    pub elf_valid: usize,
    pub parse_errors: Vec<ParseError>,
    pub libversion_found: usize,
    pub nid_resolution_avg: f64,
    pub games: Vec<GameValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameValidation {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub input_sha256: String,
    pub elf_sha256: Option<String>,
    pub self_detected: bool,
    pub elf_valid: bool,
    pub error: Option<String>,
    pub segment_count: usize,
    pub sce_segments: Vec<String>,
    pub lib_versions_count: usize,
    pub imports: usize,
    pub resolved_imports: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub name: String,
    pub error: String,
}

pub fn validate_dataset(ds: &AnalysisDataset, extracted_dir: Option<&Path>) -> ValidationReport {
    let mut games = Vec::new();
    let mut parse_errors = Vec::new();
    let mut elf_valid_count = 0usize;
    let mut libversion_count = 0usize;
    let mut total_resolved = 0usize;
    let mut total_imports = 0usize;

    let extract_manifest = extracted_dir
        .map(|d| d.join("manifest.json"))
        .filter(|p| p.exists())
        .and_then(|p| {
            let data = std::fs::read_to_string(&p).ok()?;
            let m: crate::batch_extract::ExtractionManifest = serde_json::from_str(&data).ok()?;
            Some(m)
        });

    for (name, doc) in &ds.images {
        let img = &doc.image;
        let mut error: Option<String> = None;
        let mut elf_valid = true;

        if img.platform == ps5_image::Platform::Unknown {
            error = Some("unknown platform".to_string());
            elf_valid = false;
        }
        if img.segments.is_empty() {
            error = Some("no segments".to_string());
            elf_valid = false;
        }

        let sce_segments: Vec<String> = img
            .segments
            .iter()
            .filter(|s| {
                matches!(
                    s.seg_type,
                    ps5_image::SegmentType::SCE_Dynlibdata
                        | ps5_image::SegmentType::SCE_Procparam
                        | ps5_image::SegmentType::SCE_Comment
                        | ps5_image::SegmentType::SCE_Libversion
                        | ps5_image::SegmentType::SCE_Relro
                        | ps5_image::SegmentType::SCE_Rela
                )
            })
            .map(|s| format!("{:?}", s.seg_type))
            .collect();

        let resolved_imports = img
            .imports
            .iter()
            .filter(|i| i.resolved_name.is_some())
            .count();

        total_imports += img.imports.len();
        total_resolved += resolved_imports;

        if !elf_valid {
            parse_errors.push(ParseError {
                name: name.clone(),
                error: error.clone().unwrap_or_default(),
            });
        } else {
            elf_valid_count += 1;
        }

        if !img.lib_versions.is_empty() {
            libversion_count += 1;
        }

        let elf_sha256 = extract_manifest.as_ref().and_then(|m| {
            m.entries
                .iter()
                .find(|e| {
                    let elf_stem = std::path::Path::new(&e.elf)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    elf_stem == name
                })
                .map(|e| e.elf_sha256.clone())
                .filter(|s| !s.is_empty())
        });

        games.push(GameValidation {
            name: name.clone(),
            display_name: Some(ds.display_name_for(name).to_string()),
            input_sha256: img.sha256.clone(),
            elf_sha256,
            self_detected: img.is_self,
            elf_valid,
            error,
            segment_count: img.segments.len(),
            sce_segments,
            lib_versions_count: img.lib_versions.len(),
            imports: img.imports.len(),
            resolved_imports,
        });
    }

    let nid_resolution_avg = if total_imports > 0 {
        total_resolved as f64 / total_imports as f64 * 100.0
    } else {
        0.0
    };

    ValidationReport {
        dataset_path: String::new(),
        generated_at: utc_now_iso8601(),
        tool: "ps5rs".to_string(),
        total_games: games.len(),
        elf_valid: elf_valid_count,
        parse_errors,
        libversion_found: libversion_count,
        nid_resolution_avg,
        games,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{DATASET_SCHEMA_VERSION, Manifest};
    use ps5_image::{
        BinaryImage, BinaryImageDocument, LibVersionEntry, LoadedSegment, Platform, SegmentType,
    };

    fn make_test_image(
        name: &str,
        imports_count: usize,
        lib_version_count: usize,
    ) -> (String, BinaryImageDocument) {
        let imports = (0..imports_count)
            .map(|i| ps5_image::ImportEntry {
                nid_hash: format!("nid_{i}"),
                resolved_name: Some(format!("func_{i}")),
                library_id: 1,
                library_name: "libTest".to_string(),
                value: 0,
                size: 0,
                shndx: 1,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: i as u32,
            })
            .collect();

        let lib_versions = (0..lib_version_count)
            .map(|i| LibVersionEntry {
                name: format!("libSce{i}"),
                version_raw: 0x01000000,
                version_string: "1.0.0".to_string(),
            })
            .collect();

        let segments = vec![LoadedSegment {
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
        }];

        let image = BinaryImage {
            sha256: format!("sha256_{name}"),
            platform: Platform::Ps5,
            is_self: true,
            file_size: 1024,
            entry_point: 0x80000000,
            metadata: Default::default(),
            segments,
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
            import_libs: Default::default(),
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions,
        };

        let doc = BinaryImageDocument {
            schema_version: DATASET_SCHEMA_VERSION,
            tool: "ps5rs-test".to_string(),
            image,
            string_analysis: None,
        };

        (name.to_string(), doc)
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
    fn validate_empty_dataset() {
        let ds = test_dataset(vec![]);
        let report = validate_dataset(&ds, None);
        assert_eq!(report.total_games, 0);
        assert_eq!(report.elf_valid, 0);
        assert!(report.parse_errors.is_empty());
    }

    #[test]
    fn validate_single_valid_game() {
        let ds = test_dataset(vec![make_test_image("GameA", 5, 2)]);
        let report = validate_dataset(&ds, None);
        assert_eq!(report.total_games, 1);
        assert_eq!(report.elf_valid, 1);
        assert_eq!(report.libversion_found, 1);
        assert!(report.parse_errors.is_empty());
        assert_eq!(report.nid_resolution_avg, 100.0);
    }

    #[test]
    fn validate_multiple_games_mixed() {
        let ds = test_dataset(vec![
            make_test_image("GameA", 10, 3),
            make_test_image("GameB", 5, 0),
        ]);
        let report = validate_dataset(&ds, None);
        assert_eq!(report.total_games, 2);
        assert_eq!(report.elf_valid, 2);
        assert_eq!(report.libversion_found, 1);
    }

    #[test]
    fn validate_report_serde_roundtrip() {
        let ds = test_dataset(vec![make_test_image("GameA", 3, 1)]);
        let report = validate_dataset(&ds, None);
        let json = serde_json::to_string_pretty(&report).unwrap();
        let back: ValidationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_games, 1);
        assert_eq!(back.elf_valid, 1);
        assert_eq!(back.games[0].name, "GameA");
        assert_eq!(back.games[0].imports, 3);
        assert_eq!(back.games[0].lib_versions_count, 1);
    }

    #[test]
    fn validate_report_has_metadata() {
        let ds = test_dataset(vec![]);
        let report = validate_dataset(&ds, None);
        assert!(!report.generated_at.is_empty());
        assert_eq!(report.tool, "ps5rs");
    }
}
