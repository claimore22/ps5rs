use crate::model::GameAnalysis;
use ps5_image::{
    BINARY_IMAGE_VERSION, BinaryImage, BinaryImageDocument, BinaryMetadata, ImageType, ImportEntry,
    SymbolBinding, SymbolType, SymbolVisibility, TlsInfo,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub const DATASET_SCHEMA_VERSION: u32 = 6;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub tool: String,
    pub created_at: String,
    pub image_count: usize,
    #[serde(default)]
    pub module_count: usize,
    #[serde(default)]
    pub games: Vec<crate::param_json::GameParam>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DatasetError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedSchemaVersion(u32),
    MissingManifest,
    MissingImagesDir,
}

impl std::fmt::Display for DatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::UnsupportedSchemaVersion(v) => {
                write!(
                    f,
                    "Unsupported dataset schema version {v} (max supported: {DATASET_SCHEMA_VERSION})"
                )
            }
            Self::MissingManifest => write!(f, "Missing manifest.json in dataset directory"),
            Self::MissingImagesDir => write!(f, "Missing images/ directory in dataset"),
        }
    }
}

impl std::error::Error for DatasetError {}

impl From<std::io::Error> for DatasetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for DatasetError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

fn collect_json_files(
    base_dir: &Path,
    dir: &Path,
    images: &mut Vec<(String, BinaryImageDocument, bool)>,
) -> Result<(), DatasetError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(base_dir, &path, images)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = std::fs::read_to_string(&path)?;
            // `GameAnalysis` files (intermediate scanner) first for
            // back-compat; current `BinaryImageDocument` files second. The two
            // shapes are disjoint (neither has all of the other's required
            // fields), so a file parses as at most one of them. Converted
            // documents take priority over stale duplicates (see
            // `dedupe_images`).
            let (doc, is_new) = if let Ok(game) = serde_json::from_str::<GameAnalysis>(&data) {
                (game_analysis_to_doc(&game), true)
            } else {
                (serde_json::from_str::<BinaryImageDocument>(&data)?, false)
            };
            let rel = path.strip_prefix(base_dir).unwrap_or(&path);
            let key = if doc.parent_image.is_some() {
                // module: use {game_dir}/{file_stem}
                let parent_dir = rel
                    .parent()
                    .and_then(|p| p.file_stem())
                    .or_else(|| rel.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                format!("{parent_dir}/{file_stem}")
            } else {
                // game image: use parent dir name (subdir) or file_stem (flat layout)
                if rel.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
                    // inside a subdirectory — key is the dir name
                    rel.parent()
                        .and_then(|p| p.file_stem())
                        .or_else(|| rel.file_stem())
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    // flat file at images/*.json — key is file_stem (backward compat)
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                }
            };
            images.push((key, doc, is_new));
        }
    }
    Ok(())
}

/// Convert the current `scan` output (`GameAnalysis`) into the stable
/// `BinaryImageDocument` shape consumed by reports and the dashboard.
///
/// The mapping is faithful for everything `GameAnalysis` carries (identity,
/// entry point, full import table with NID resolution, library index,
/// needed files). What it cannot carry is synthesized as empty: segments,
/// exports, relocations, lib versions. The TLS flag survives as a
/// zero-filled `TlsInfo` — converted documents are in-memory only, never
/// written back to disk, and every consumer reads just the boolean.
pub(crate) fn game_analysis_to_doc(game: &GameAnalysis) -> BinaryImageDocument {
    let platform = match game.platform {
        crate::model::Platform::Ps4 => ps5_image::Platform::Ps4,
        crate::model::Platform::Ps5 => ps5_image::Platform::Ps5,
        crate::model::Platform::RawElf => ps5_image::Platform::RawElf,
        crate::model::Platform::Unknown => ps5_image::Platform::Unknown,
    };
    let imports = game
        .imports
        .iter()
        .map(|imp| ImportEntry {
            nid_hash: imp.nid_hash.clone(),
            resolved_name: if imp.resolved_name == "?" {
                None
            } else {
                Some(imp.resolved_name.clone())
            },
            library_id: imp.library_id,
            library_name: imp.library_name.clone(),
            value: 0,
            size: 0,
            shndx: 0,
            binding: SymbolBinding::Global,
            sym_type: SymbolType::Func,
            visibility: SymbolVisibility::Default,
            ordinal: 0,
        })
        .collect();
    let import_libs = game
        .import_libs
        .iter()
        .map(|lib| (lib.id, lib.name.clone()))
        .collect();
    BinaryImageDocument {
        schema_version: BINARY_IMAGE_VERSION,
        tool: "ps5rs".to_string(),
        image_type: ImageType::Eboot,
        parent_image: None,
        string_analysis: None,
        image: BinaryImage {
            sha256: game.sha256.clone(),
            platform,
            is_self: game.is_self,
            file_size: game.file_size,
            entry_point: game.entry_point,
            metadata: BinaryMetadata::default(),
            segments: vec![],
            imports,
            exports: vec![],
            relocations: vec![],
            tls: game.has_tls.then_some(TlsInfo {
                vaddr: 0,
                filesz: 0,
                memsz: 0,
                align: 0,
            }),
            init_va: 0,
            init_array_va: 0,
            init_array_sz: 0,
            fini_va: 0,
            fini_array_va: 0,
            fini_array_sz: 0,
            preinit_array_va: 0,
            preinit_array_sz: 0,
            import_libs,
            needed_files: game.needed_files.clone(),
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions: vec![],
        },
    }
}

/// Extract a `PPSA12345` title ID from arbitrary text (case-insensitive).
pub fn extract_title_id(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut i = 0;
    while i + 9 <= bytes.len() {
        if &bytes[i..i + 4] == b"PPSA" && bytes[i + 4..i + 9].iter().all(|c| c.is_ascii_digit()) {
            return Some(upper[i..i + 9].to_string());
        }
        i += 1;
    }
    None
}

/// Drop stale legacy duplicates: when the same title ID exists both as a
/// new-format (converted) document and as legacy `BinaryImageDocument`
/// file(s) under a different stem, the converted one wins. Module images
/// (`parent_image` set) and files without an ID never participate.
fn dedupe_images(images: &mut Vec<(String, BinaryImageDocument, bool)>) {
    let new_ids: HashSet<String> = images
        .iter()
        .filter(|(_, _, is_new)| *is_new)
        .filter_map(|(key, _, _)| extract_title_id(key))
        .collect();
    if new_ids.is_empty() {
        return;
    }
    images.retain(|(key, doc, is_new)| {
        if *is_new || doc.parent_image.is_some() {
            return true;
        }
        match extract_title_id(key) {
            Some(id) => !new_ids.contains(&id),
            None => true,
        }
    });
}

// ---------------------------------------------------------------------------
// AnalysisDataset
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct AnalysisDataset {
    pub manifest: Manifest,
    pub images: Vec<(String, BinaryImageDocument)>,
    pub display_names: HashMap<String, String>,
}

impl AnalysisDataset {
    pub fn display_name_for<'a>(&'a self, name: &'a str) -> &'a str {
        self.display_names
            .get(name)
            .map(String::as_str)
            .unwrap_or(name)
    }
    /// Open a dataset directory (reads manifest.json + images/*.json).
    pub fn open(root: &Path) -> Result<Self, DatasetError> {
        let manifest_path = root.join("manifest.json");
        if !manifest_path.exists() {
            return Err(DatasetError::MissingManifest);
        }
        let manifest_data = std::fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = serde_json::from_str(&manifest_data)?;

        if manifest.schema_version > DATASET_SCHEMA_VERSION {
            return Err(DatasetError::UnsupportedSchemaVersion(
                manifest.schema_version,
            ));
        }

        let images_dir = root.join("images");
        if !images_dir.exists() {
            return Err(DatasetError::MissingImagesDir);
        }

        let mut images: Vec<(String, BinaryImageDocument, bool)> = Vec::new();
        collect_json_files(&images_dir, &images_dir, &mut images)?;
        dedupe_images(&mut images);
        let mut images: Vec<(String, BinaryImageDocument)> =
            images.into_iter().map(|(key, doc, _)| (key, doc)).collect();
        images.sort_by(|a, b| a.0.cmp(&b.0));

        let mut display_names = HashMap::new();
        for (name, doc) in &images {
            if doc.parent_image.is_some() {
                continue;
            }
            let name_upper = name.to_ascii_uppercase();
            let display = manifest.games.iter().find_map(|g| {
                g.display_name.as_ref().filter(|_| {
                    g.title_id
                        .as_ref()
                        .is_some_and(|tid| name_upper.contains(&tid.to_ascii_uppercase()))
                })
            });
            if let Some(d) = display {
                display_names.insert(name.clone(), d.clone());
            }
        }

        Ok(AnalysisDataset {
            manifest,
            images,
            display_names,
        })
    }

    pub fn game_images(&self) -> Vec<&(String, BinaryImageDocument)> {
        self.images
            .iter()
            .filter(|(_, doc)| doc.parent_image.is_none())
            .collect()
    }

    pub fn module_images(&self) -> Vec<&(String, BinaryImageDocument)> {
        self.images
            .iter()
            .filter(|(_, doc)| doc.parent_image.is_some())
            .collect()
    }

    pub fn modules_for_game(&self, game_name: &str) -> Vec<&(String, BinaryImageDocument)> {
        self.images
            .iter()
            .filter(|(_, doc)| doc.parent_image.as_deref() == Some(game_name))
            .collect()
    }

    pub fn total_imports(&self) -> usize {
        self.images
            .iter()
            .map(|(_, doc)| doc.image.imports.len())
            .sum()
    }

    pub fn unique_nids(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for (_, doc) in &self.images {
            for imp in &doc.image.imports {
                seen.insert(&imp.nid_hash);
            }
        }
        seen.len()
    }

    pub fn unique_libs(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for (_, doc) in &self.images {
            for imp in &doc.image.imports {
                seen.insert(&imp.library_name);
            }
        }
        seen.len()
    }

    pub fn resolved_count(&self) -> usize {
        self.images
            .iter()
            .flat_map(|(_, doc)| doc.image.imports.iter())
            .filter(|imp| imp.resolved_name.is_some())
            .count()
    }

    pub fn resolution_rate(&self) -> f64 {
        let total = self.total_imports();
        if total == 0 {
            return 0.0;
        }
        self.resolved_count() as f64 / total as f64 * 100.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_image::{BinaryImage, ImportEntry, Platform};
    use std::path::PathBuf;

    fn make_image_doc(sha256: &str, imports: Vec<ImportEntry>) -> BinaryImageDocument {
        BinaryImageDocument {
            schema_version: 1,
            tool: "ps5rs".to_string(),
            image_type: ps5_image::ImageType::Eboot,
            parent_image: None,
            string_analysis: None,
            image: BinaryImage {
                sha256: sha256.to_string(),
                platform: Platform::Ps5,
                is_self: true,
                file_size: 1024,
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

    fn tempdir_for_test(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ps5rs_dataset_test_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn make_dataset_dir(root: &Path, docs: &[(&str, BinaryImageDocument)]) {
        std::fs::create_dir_all(root.join("images")).unwrap();

        let manifest = Manifest {
            schema_version: DATASET_SCHEMA_VERSION,
            tool: "ps5rs".to_string(),
            created_at: "2026-07-25T00:00:00Z".to_string(),
            image_count: docs.len(),
            module_count: 0,
            games: vec![],
        };
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(root.join("manifest.json"), manifest_json).unwrap();

        for (name, doc) in docs {
            let json = serde_json::to_string_pretty(doc).unwrap();
            std::fs::write(root.join("images").join(format!("{name}.json")), json).unwrap();
        }
    }

    #[test]
    fn open_dataset_roundtrip() {
        let root = tempdir_for_test("roundtrip");
        let doc1 = make_image_doc(
            &"aa".repeat(32),
            vec![ImportEntry {
                nid_hash: "abc".into(),
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
        );
        let doc2 = make_image_doc(&"bb".repeat(32), vec![]);
        make_dataset_dir(&root, &[("game1", doc1), ("game2", doc2)]);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.manifest.schema_version, DATASET_SCHEMA_VERSION);
        assert_eq!(ds.manifest.image_count, 2);
        assert_eq!(ds.images.len(), 2);
        assert_eq!(ds.images[0].0, "game1");
        assert_eq!(ds.images[0].1.image.imports.len(), 1);
        assert_eq!(ds.images[1].0, "game2");
        assert!(ds.images[1].1.image.imports.is_empty());
        assert_eq!(ds.display_name_for("game1"), "game1");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_dataset_missing_manifest() {
        let root = tempdir_for_test("no_manifest");
        std::fs::create_dir_all(root.join("images")).unwrap();

        let result = AnalysisDataset::open(&root);
        assert!(matches!(result, Err(DatasetError::MissingManifest)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_dataset_missing_images_dir() {
        let root = tempdir_for_test("no_images");
        std::fs::create_dir_all(&root).unwrap();
        let manifest = Manifest {
            schema_version: 1,
            tool: "ps5rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            image_count: 0,
            module_count: 0,
            games: vec![],
        };
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let result = AnalysisDataset::open(&root);
        assert!(matches!(result, Err(DatasetError::MissingImagesDir)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_dataset_rejects_future_schema() {
        let root = tempdir_for_test("future_schema");
        std::fs::create_dir_all(root.join("images")).unwrap();
        let manifest = Manifest {
            schema_version: 999,
            tool: "ps5rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            image_count: 0,
            module_count: 0,
            games: vec![],
        };
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let result = AnalysisDataset::open(&root);
        assert!(matches!(
            result,
            Err(DatasetError::UnsupportedSchemaVersion(999))
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stats_empty_dataset() {
        let root = tempdir_for_test("stats_empty");
        make_dataset_dir(&root, &[]);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.total_imports(), 0);
        assert_eq!(ds.unique_nids(), 0);
        assert_eq!(ds.unique_libs(), 0);
        assert_eq!(ds.resolution_rate(), 0.0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stats_with_imports() {
        let root = tempdir_for_test("stats_imports");
        let doc = make_image_doc(
            &"cc".repeat(32),
            vec![
                ImportEntry {
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
                },
                ImportEntry {
                    nid_hash: "bbb".into(),
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
                    nid_hash: "aaa".into(),
                    resolved_name: Some("funcA".into()),
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
            ],
        );
        make_dataset_dir(&root, &[("game1", doc)]);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.total_imports(), 3);
        assert_eq!(ds.unique_nids(), 2); // aaa, bbb
        assert_eq!(ds.unique_libs(), 2); // libA, libB
        assert_eq!(ds.resolved_count(), 2);
        assert!((ds.resolution_rate() - 66.66).abs() < 0.1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dataset_images_sorted_by_filename() {
        let root = tempdir_for_test("sorted");
        let d1 = make_image_doc(&"11".repeat(32), vec![]);
        let d2 = make_image_doc(&"22".repeat(32), vec![]);
        let d3 = make_image_doc(&"33".repeat(32), vec![]);
        make_dataset_dir(&root, &[("game-c", d3), ("game-a", d1), ("game-b", d2)]);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.images.len(), 3);
        assert_eq!(ds.images[0].0, "game-a");
        assert_eq!(ds.images[1].0, "game-b");
        assert_eq!(ds.images[2].0, "game-c");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let m = Manifest {
            schema_version: 1,
            tool: "ps5rs".to_string(),
            created_at: "2026-07-25T12:00:00Z".to_string(),
            image_count: 42,
            module_count: 0,
            games: vec![],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.image_count, 42);
        assert_eq!(back.created_at, "2026-07-25T12:00:00Z");
    }

    fn write_game_analysis_json(root: &Path, stem: &str, game: &crate::model::GameAnalysis) {
        let json = serde_json::to_string_pretty(game).unwrap();
        std::fs::write(root.join("images").join(format!("{stem}.json")), json).unwrap();
    }

    #[test]
    fn open_new_format_only_maps_fields() {
        let root = tempdir_for_test("new_format");
        make_dataset_dir(&root, &[]);
        let mut game = crate::model::make_game(
            "Cool",
            vec![
                crate::model::make_import("aaa", "funcA", 7, "libA"),
                crate::model::make_import("bbb", "?", 9, "libB"),
            ],
        );
        game.sha256 = "cc".repeat(32);
        game.display_name = Some("Cool - [PPSA12345]".to_string());
        game.import_libs = vec![crate::model::LibInfo {
            id: 7,
            name: "libA".to_string(),
        }];
        game.needed_files = vec!["libA.prx".to_string()];
        game.has_tls = true;
        write_game_analysis_json(&root, "Cool-_-v1_[PPSA12345]", &game);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.images.len(), 1);
        let (key, doc) = &ds.images[0];
        assert_eq!(key, "Cool-_-v1_[PPSA12345]");
        assert_eq!(doc.schema_version, BINARY_IMAGE_VERSION);
        assert_eq!(doc.image.sha256, "cc".repeat(32));
        assert_eq!(doc.image.platform, ps5_image::Platform::Ps5);
        assert_eq!(doc.image.imports.len(), 2);
        assert_eq!(doc.image.imports[0].resolved_name.as_deref(), Some("funcA"));
        assert_eq!(doc.image.imports[1].resolved_name, None);
        assert_eq!(
            doc.image.import_libs.get(&7).map(String::as_str),
            Some("libA")
        );
        assert_eq!(doc.image.needed_files, vec!["libA.prx".to_string()]);
        assert!(doc.image.tls.is_some());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_mixed_format_new_format_wins() {
        let root = tempdir_for_test("mixed_format");
        let legacy = make_image_doc(
            &"aa".repeat(32),
            vec![ImportEntry {
                nid_hash: "old-nid".into(),
                resolved_name: Some("oldFunc".into()),
                library_id: 1,
                library_name: "libOld".into(),
                value: 0,
                size: 0,
                shndx: 0,
                binding: ps5_image::SymbolBinding::Global,
                sym_type: ps5_image::SymbolType::Func,
                visibility: ps5_image::SymbolVisibility::Default,
                ordinal: 0,
            }],
        );
        make_dataset_dir(&root, &[("Cool-Game-PPSA12345-v1_[TAG]", legacy)]);
        let game = crate::model::make_game(
            "Cool",
            vec![crate::model::make_import("new-nid", "newFunc", 2, "libNew")],
        );
        write_game_analysis_json(&root, "Cool-Game-_-v1_[PPSA12345]", &game);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.images.len(), 1);
        assert_eq!(ds.images[0].0, "Cool-Game-_-v1_[PPSA12345]");
        assert_eq!(ds.images[0].1.image.imports.len(), 1);
        assert_eq!(ds.images[0].1.image.imports[0].nid_hash, "new-nid");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_mixed_format_keeps_unmatched_legacy() {
        let root = tempdir_for_test("mixed_keep");
        let legacy = make_image_doc(&"aa".repeat(32), vec![]);
        make_dataset_dir(&root, &[("Some-Game-PPSA99999-v1", legacy)]);
        let game = crate::model::make_game("Cool", vec![]);
        write_game_analysis_json(&root, "Cool-_-v1_[PPSA12345]", &game);

        let ds = AnalysisDataset::open(&root).unwrap();
        assert_eq!(ds.images.len(), 2);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_title_id_cases() {
        assert_eq!(
            extract_title_id("Cool-_-v1_[PPSA12345]"),
            Some("PPSA12345".to_string())
        );
        assert_eq!(
            extract_title_id("Cool-PPSA12345-v1"),
            Some("PPSA12345".to_string())
        );
        assert_eq!(
            extract_title_id("cool-ppsa12345"),
            Some("PPSA12345".to_string())
        );
        assert_eq!(extract_title_id("NoIdHere"), None);
        assert_eq!(extract_title_id("PPSA123"), None);
    }
}
