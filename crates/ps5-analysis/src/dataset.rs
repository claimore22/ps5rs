use ps5_image::BinaryImageDocument;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    images: &mut Vec<(String, BinaryImageDocument)>,
) -> Result<(), DatasetError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(base_dir, &path, images)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = std::fs::read_to_string(&path)?;
            let doc: BinaryImageDocument = serde_json::from_str(&data)?;
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
            images.push((key, doc));
        }
    }
    Ok(())
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

        let mut images = Vec::new();
        collect_json_files(&images_dir, &images_dir, &mut images)?;
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
}
