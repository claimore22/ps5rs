// Consolidated scanner module – single implementation
use std::path::{Path, PathBuf};

use crate::collector::{CollectorOptions, find_binaries};
use crate::dataset::{DATASET_SCHEMA_VERSION, Manifest};
use crate::param_json::{self, GameParam};
use ps5_image::{BINARY_IMAGE_VERSION, BinaryImageBuilder, BinaryImageDocument, ImageType};
use ps5_nid::Catalog;

pub fn utc_now_iso8601() -> String {
    // Simple UTC timestamp in ISO‑8601 format (seconds precision)
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Approximate date components – sufficient for manifest timestamps.
    let days = secs / 86_400;
    let secs_of_day = secs % 86_400;
    let year = 1970 + days / 365;
    let month = ((days % 365) / 30 + 1).min(12);
    let day = (days % 365) % 30 + 1;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[derive(Default)]
pub struct ScanOptions {
    pub include_prx: bool,
    pub append: bool,
}

pub struct ScanResult {
    pub manifest: Manifest,
    pub image_paths: Vec<PathBuf>,
}

/// Title IDs with durable per-game records (`games/` + `ingest.json`).
/// Images belonging to these titles are retained across rescans even when
/// their source directory is gone (mirrors `ps5-farm` report retention).
fn registered_title_ids(output: &Path) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Ok(data) = std::fs::read(output.join("ingest.json")) else {
        return out;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&data) else {
        return out;
    };
    if let Some(games) = value.get("games").and_then(|g| g.as_object()) {
        for key in games.keys() {
            out.insert(key.to_ascii_uppercase());
        }
    }
    out
}

fn image_stem_title_id(stem: &str) -> Option<String> {
    crate::dataset::extract_title_id(stem)
}

pub fn scan(
    root: &Path,
    output: &Path,
    catalog: &Catalog,
    options: &ScanOptions,
) -> Result<ScanResult, std::io::Error> {
    let images_dir = output.join("images");
    let retained = registered_title_ids(output);
    let game_dirs = find_game_dirs(root);
    eprintln!("Found {} game(s) in {}", game_dirs.len(), root.display());
    let current_titles: std::collections::HashSet<String> = game_dirs
        .iter()
        .filter_map(|d| d.file_name().and_then(|n| n.to_str()))
        .filter_map(crate::dataset::extract_title_id)
        .collect();
    if options.append {
        std::fs::create_dir_all(&images_dir)?;
    } else if images_dir.exists() {
        for entry in std::fs::read_dir(&images_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = entry.path();
            if !path.extension().is_some_and(|e| e == "json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            // Keep archived images only when their source is absent: a
            // present source is rescanned fresh so fingerprints stay current.
            if image_stem_title_id(stem)
                .is_some_and(|id| retained.contains(&id) && !current_titles.contains(&id))
            {
                continue;
            }
            let _ = std::fs::remove_file(&path);
        }
        std::fs::create_dir_all(&images_dir)?;
    } else {
        std::fs::create_dir_all(&images_dir)?;
    }

    let mut image_paths = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut params_by_title: std::collections::HashMap<String, GameParam> =
        std::collections::HashMap::new();
    let mut untitled_params: Vec<GameParam> = Vec::new();
    let mut created_at: Option<String> = None;
    if options.append {
        if let Ok(entries) = std::fs::read_dir(&images_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    seen_names.insert(stem.to_string());
                }
            }
        }
        if let Ok(data) = std::fs::read_to_string(output.join("manifest.json"))
            && let Ok(manifest) = serde_json::from_str::<Manifest>(&data)
        {
            created_at = Some(manifest.created_at);
            for param in manifest.games {
                match param.title_id.clone() {
                    Some(id) => {
                        params_by_title
                            .entry(id.to_ascii_uppercase())
                            .or_insert(param);
                    }
                    None => untitled_params.push(param),
                }
            }
        }
    } else {
        if let Ok(entries) = std::fs::read_dir(&images_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    seen_names.insert(stem.to_string());
                    if let Some(id) = image_stem_title_id(stem) {
                        seen_titles.insert(id);
                    }
                }
            }
        }
        if let Ok(data) = std::fs::read_to_string(output.join("manifest.json"))
            && let Ok(manifest) = serde_json::from_str::<Manifest>(&data)
        {
            created_at = Some(manifest.created_at);
            for param in manifest.games {
                if let Some(id) = param.title_id.clone()
                    && retained.contains(&id.to_ascii_uppercase())
                    && seen_titles.contains(&id.to_ascii_uppercase())
                {
                    params_by_title
                        .entry(id.to_ascii_uppercase())
                        .or_insert(param);
                }
            }
        }
    }

    for (idx, game_dir) in game_dirs.iter().enumerate() {
        let game_name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let safe_name = sanitize_filename(game_name);
        if seen_names.contains(&safe_name) {
            eprintln!(
                "[{}/{}] {game_name} ... SKIPPED (already scanned)",
                idx + 1,
                game_dirs.len()
            );
            continue;
        }
        let collector_opts = CollectorOptions {
            include_prx: options.include_prx,
        };
        let binaries = find_binaries(game_dir, &collector_opts);
        if binaries.is_empty() {
            eprintln!(
                "[{}/{}] {game_name} ... SKIPPED (no binaries)",
                idx + 1,
                game_dirs.len()
            );
            continue;
        }
        eprint!("[{}/{}] {game_name} ... ", idx + 1, game_dirs.len());
        for bin_path in &binaries {
            if seen_names.contains(&safe_name) {
                continue;
            }
            let data = match std::fs::read(bin_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("FAILED (read: {e})");
                    continue;
                }
            };
            let size_mb = data.len() as f64 / 1_048_576.0;
            let sha256 = ps5_format::sha256_hex(&data);
            let image = BinaryImageBuilder::build_from_file(&data, &sha256, catalog);
            let doc = assemble_image_document(image, &data, image_type_for(bin_path));
            seen_names.insert(safe_name.clone());
            let json_path = output.join("images").join(format!("{safe_name}.json"));
            let json = serde_json::to_string_pretty(&doc)?;
            std::fs::write(&json_path, format!("{json}\n"))?;
            image_paths.push(json_path);
            eprintln!(
                "OK ({:.1} MB, {} imports)",
                size_mb,
                doc.image.imports.len()
            );

            let param = param_json::read_param(game_dir).unwrap_or_default();
            let mut param = param;
            if param.name.is_none() {
                param.name = Some(crate::names::scrub_scene_tags(game_name));
            }
            if param.title_id.as_ref().is_none_or(|t| t.trim().is_empty())
                && let Some(id) = crate::dataset::extract_title_id(game_name)
            {
                param.title_id = Some(id.clone());
                param.display_name = param
                    .compute_display_name()
                    .or_else(|| param.name.clone().map(|n| format!("{n} - [{id}]")));
            }
            match param.title_id.clone() {
                Some(id) => {
                    params_by_title.insert(id.to_ascii_uppercase(), param);
                    seen_titles.insert(id.to_ascii_uppercase());
                }
                None => untitled_params.push(param),
            }
        }
    }

    let mut shaders: Vec<ps5_schema::ShaderRecord> = Vec::new();
    let mut shadered_games: std::collections::HashSet<String> = std::collections::HashSet::new();
    if options.append
        && let Ok(data) = std::fs::read(output.join("shaders.json"))
        && let Ok(existing) = serde_json::from_slice::<Vec<ps5_schema::ShaderRecord>>(&data)
    {
        for record in &existing {
            shadered_games.insert(crate::names::scrub_scene_tags(&record.game));
        }
        shaders = existing;
    }
    let pending_shaders: Vec<(&PathBuf, String)> = game_dirs
        .iter()
        .map(|game_dir| {
            let name = crate::names::scrub_scene_tags(
                game_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
            );
            (game_dir, name)
        })
        .filter(|(_, name)| !(options.append && shadered_games.contains(name)))
        .collect();
    eprintln!(
        "Inventorying shaders for {} game(s)...",
        pending_shaders.len()
    );
    for (game_dir, name) in &pending_shaders {
        eprint!("  {name} ... ");
        let before = shaders.len();
        shaders.extend(crate::shader_inventory::inventory_shaders_for_game(
            game_dir, name,
        ));
        eprintln!("OK ({} records)", shaders.len() - before);
    }
    if !shaders.is_empty() {
        let shaders_json = serde_json::to_string_pretty(&shaders)?;
        std::fs::write(output.join("shaders.json"), format!("{shaders_json}\n"))?;
    }

    let total_images = std::fs::read_dir(&images_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .count()
        })
        .unwrap_or(image_paths.len());
    let mut game_params: Vec<GameParam> = params_by_title.into_values().collect();
    game_params.sort_by(|a, b| {
        a.title_id
            .cmp(&b.title_id)
            .then_with(|| a.name.cmp(&b.name))
    });
    game_params.extend(untitled_params);
    let manifest = Manifest {
        schema_version: DATASET_SCHEMA_VERSION,
        tool: "ps5rs".to_string(),
        created_at: created_at.unwrap_or_else(utc_now_iso8601),
        image_count: total_images,
        module_count: 0,
        games: game_params,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(output.join("manifest.json"), format!("{manifest_json}\n"))?;

    Ok(ScanResult {
        manifest,
        image_paths,
    })
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    find_game_dirs_inner(root)
}

pub(crate) fn find_game_dirs_for_artifacts(root: &Path) -> Vec<PathBuf> {
    find_game_dirs_inner(root)
}

fn find_game_dirs_inner(root: &Path) -> Vec<PathBuf> {
    // Accept a single game directory directly: if the root itself holds an
    // eboot, it *is* the game (e.g. `scan <corpus/Game>` to add just one).
    if has_eboot(root) {
        return vec![root.to_path_buf()];
    }
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resolve_game_dir(&path, &mut dirs);
            }
        }
    }
    dirs.sort();
    dirs
}

fn resolve_game_dir(dir: &Path, result: &mut Vec<PathBuf>) {
    if has_eboot(dir) {
        result.push(dir.to_path_buf());
        return;
    }
    let children: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => return,
    };
    if children.len() == 1 {
        resolve_game_dir(&children[0], result);
    }
}

fn has_eboot(dir: &Path) -> bool {
    dir.join("eboot.bin").exists()
}

/// Classify an image by its file extension for the document wrapper.
fn image_type_for(path: &Path) -> ImageType {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("prx") => ImageType::Prx,
        Some("sprx") => ImageType::Sprx,
        _ => ImageType::Eboot,
    }
}

/// Assemble the full `BinaryImageDocument` written per scanned binary:
/// parsed image plus string fingerprints. Stamps the image interchange
/// version so the file round-trips through `ps5_image::json::import_json`.
pub(crate) fn assemble_image_document(
    image: ps5_image::BinaryImage,
    data: &[u8],
    image_type: ImageType,
) -> BinaryImageDocument {
    BinaryImageDocument {
        schema_version: BINARY_IMAGE_VERSION,
        tool: "ps5rs".to_string(),
        image,
        string_analysis: Some(crate::string_patterns::analyze_strings(data)),
        image_type,
        parent_image: None,
    }
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    // 1️⃣ Extract an ID if present.
    let mut base = name.to_string();
    let mut id_opt: Option<String> = None;
    // a) ID extraction precedence: "[PPSA12345]" bracket > raw "PPSA12345" >
    // any other bracket pair (legacy fallback). A non-PPSA bracket (e.g.
    // "[SuperPSX]") never shadows a real title ID.
    let mut first_bracket: Option<(usize, usize)> = None;
    let mut ppsa_bracket: Option<(usize, usize)> = None;
    let mut search_from = 0;
    while let Some(rel) = base[search_from..].find('[') {
        let start = search_from + rel;
        match base[start..].find(']') {
            Some(rel_end) => {
                let end = start + rel_end;
                let inner = &base[start + 1..end];
                if inner.len() == 9
                    && inner.starts_with("PPSA")
                    && inner[4..].chars().all(|c| c.is_ascii_digit())
                {
                    ppsa_bracket = Some((start, end));
                    break;
                }
                if first_bracket.is_none() {
                    first_bracket = Some((start, end));
                }
                search_from = end + 1;
            }
            None => break,
        }
    }
    if let Some((start, end)) = ppsa_bracket {
        id_opt = Some(base[start..end + 1].to_string());
        base = format!("{} {}", &base[..start], &base[end + 1..]);
    } else {
        let other = first_bracket.map(|(s, e)| base[s..e + 1].to_string());
        if let Some((start, end)) = first_bracket {
            base = format!("{} {}", &base[..start], &base[end + 1..]);
        }
        // b) Unbracketed raw ID "PPSA12345"
        if let Some(idx) = base.find("PPSA") {
            let tail = &base[idx..];
            if tail.len() >= 9 && tail[5..9].chars().all(|c| c.is_ascii_digit()) {
                let raw = &tail[..9];
                id_opt = Some(format!("[{}]", raw));
                base = format!("{} {}", &base[..idx], &base[idx + 9..]);
            }
        }
        if id_opt.is_none() {
            id_opt = other;
        }
    }
    // 2️⃣ Normalise the remaining base string.
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in base.chars() {
        if ch.is_ascii_alphanumeric() || "-._[]".contains(ch) {
            out.push(ch);
            prev_underscore = false;
        } else if (ch.is_whitespace() || matches!(ch, '/' | '\\')) && !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    let mut sanitized = out.trim_matches('_').to_string();
    // 3️⃣ Append the ID (if any) with a clean separator.
    if let Some(id) = id_opt {
        if !sanitized.is_empty() {
            sanitized.push('_');
        }
        sanitized.push_str(&id);
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::{assemble_image_document, image_type_for, sanitize_filename};
    use ps5_image::{BINARY_IMAGE_VERSION, BinaryImage, BinaryMetadata, ImageType, Platform};

    #[test]
    fn basic() {
        assert_eq!(sanitize_filename("My Game"), "My_Game");
        assert_eq!(sanitize_filename("Game-A"), "Game-A");
        assert_eq!(sanitize_filename("a_b"), "a_b");
    }

    #[test]
    fn path_separators() {
        assert_eq!(sanitize_filename("a/b"), "a_b");
        assert_eq!(sanitize_filename("../escape"), ".._escape");
    }

    #[test]
    fn ppsa_brackets() {
        assert_eq!(sanitize_filename("Cool [PPSA12345]"), "Cool_[PPSA12345]");
    }

    #[test]
    fn ppsa_raw() {
        assert_eq!(sanitize_filename("Cool PPSA12345"), "Cool_[PPSA12345]");
    }

    #[test]
    fn ppsa_bracket_preferred_over_other_tags() {
        assert_eq!(
            sanitize_filename("[SuperPSX]-PRAGMATA-PPSA02530-USA-Game (v01.2000)-PS5"),
            "-PRAGMATA-_-USA-Game_v01.2000-PS5_[PPSA02530]"
        );
    }

    #[test]
    fn non_ppsa_bracket_falls_back_to_first_pair() {
        assert_eq!(sanitize_filename("Cool [Demo]"), "Cool_[Demo]");
    }

    #[test]
    fn find_game_dirs_accepts_single_game_dir() {
        let tmp = scan_test_root("single_dir");
        let game = tmp.join("SoloGame-PPSA99999-USA-Game-PS5");
        write_fake_eboot(&game);
        let found = super::find_game_dirs_inner(&game);
        assert_eq!(found, vec![game]);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_game_dirs_corpus_root_unchanged() {
        let tmp = scan_test_root("corpus_root");
        let corpus = tmp.join("corpus");
        write_fake_eboot(&corpus.join("GameA-PPSA10001-USA-Game-PS5"));
        write_fake_eboot(&corpus.join("GameB-PPSA10002-USA-Game-PS5"));
        let mut found = super::find_game_dirs_inner(&corpus);
        found.sort();
        assert_eq!(found.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn minimal_image(sha256: &str) -> BinaryImage {
        BinaryImage {
            sha256: sha256.to_string(),
            platform: Platform::Ps5,
            is_self: true,
            file_size: 2048,
            entry_point: 0x80000000,
            metadata: BinaryMetadata::default(),
            segments: vec![],
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
            import_libs: std::collections::HashMap::new(),
            needed_files: vec![],
            dynamic_entries: vec![],
            version_defs: vec![],
            lib_versions: vec![],
        }
    }

    #[test]
    fn assemble_document_stamps_version_and_type() {
        let doc = assemble_image_document(
            minimal_image(&"ab".repeat(32)),
            b"UnityEngine\x00puts\x00",
            ImageType::Eboot,
        );
        assert_eq!(doc.schema_version, BINARY_IMAGE_VERSION);
        assert_eq!(doc.tool, "ps5rs");
        assert!(matches!(doc.image_type, ImageType::Eboot));
        assert!(doc.parent_image.is_none());
        assert_eq!(doc.image.sha256, "ab".repeat(32));
        let strings = doc.string_analysis.expect("string fingerprints attached");
        assert!(strings.sce_libraries.is_empty());
    }

    #[test]
    fn image_type_for_extension() {
        use std::path::Path;
        assert!(matches!(
            image_type_for(Path::new("eboot.bin")),
            ImageType::Eboot
        ));
        assert!(matches!(
            image_type_for(Path::new("libc.prx")),
            ImageType::Prx
        ));
        assert!(matches!(
            image_type_for(Path::new("mod.sprx")),
            ImageType::Sprx
        ));
        assert!(matches!(
            image_type_for(Path::new("LIBC.PRX")),
            ImageType::Prx
        ));
    }

    fn scan_test_root(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ps5rs_scan_test_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_fake_eboot(game_dir: &std::path::Path) {
        std::fs::create_dir_all(game_dir).unwrap();
        std::fs::write(game_dir.join("eboot.bin"), b"fake-elf-bytes").unwrap();
    }

    fn write_manifest(dataset: &std::path::Path, games: Vec<crate::param_json::GameParam>) {
        let manifest = crate::dataset::Manifest {
            schema_version: crate::dataset::DATASET_SCHEMA_VERSION,
            tool: "ps5rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            image_count: games.len(),
            module_count: 0,
            games,
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(dataset.join("manifest.json"), json).unwrap();
    }

    fn param_with_title(title_id: &str, name: &str) -> crate::param_json::GameParam {
        crate::param_json::GameParam {
            name: Some(name.to_string()),
            title_id: Some(title_id.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn registered_images_survive_non_append_rescan() {
        let tmp = scan_test_root("retain");
        let corpus = tmp.join("corpus");
        let dataset = tmp.join("dataset");
        std::fs::create_dir_all(dataset.join("images")).unwrap();
        write_fake_eboot(&corpus.join("LiveGame-PPSA33333-USA-Game-PS5"));

        std::fs::write(
            dataset.join("images").join("Kept_[PPSA11111].json"),
            b"{\"kept\":true}",
        )
        .unwrap();
        std::fs::write(
            dataset.join("images").join("Dropped_[PPSA22222].json"),
            b"{\"dropped\":true}",
        )
        .unwrap();
        std::fs::write(
            dataset.join("ingest.json"),
            serde_json::json!({
                "schema": 1,
                "games": {
                    "PPSA11111": {
                        "name": "Kept", "eboot_sha256": "a", "game_fingerprint": "b",
                        "ingest_id": "x", "status": "complete", "ingested_at": "t",
                        "source_deleted": true, "record_path": "r", "report_file": "f",
                        "corpus_dir": "c"
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        write_manifest(
            &dataset,
            vec![
                param_with_title("PPSA11111", "Kept"),
                param_with_title("PPSA22222", "Dropped"),
            ],
        );

        let catalog = ps5_nid::Catalog::new();
        let options = super::ScanOptions {
            include_prx: false,
            append: false,
        };
        let result = super::scan(&corpus, &dataset, &catalog, &options).unwrap();

        assert!(
            dataset
                .join("images")
                .join("Kept_[PPSA11111].json")
                .exists(),
            "registered image must be retained"
        );
        assert!(
            !dataset
                .join("images")
                .join("Dropped_[PPSA22222].json")
                .exists(),
            "unregistered image must be cleaned"
        );
        assert_eq!(result.manifest.image_count, 2);
        assert_eq!(result.manifest.games.len(), 2);
        let ids: Vec<_> = result
            .manifest
            .games
            .iter()
            .filter_map(|g| g.title_id.clone())
            .collect();
        assert!(ids.contains(&"PPSA11111".to_string()));
        assert!(ids.contains(&"PPSA33333".to_string()));
        assert!(!ids.contains(&"PPSA22222".to_string()));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn title_id_falls_back_to_directory_name() {
        let tmp = scan_test_root("title_fallback");
        let corpus = tmp.join("corpus");
        let dataset = tmp.join("dataset");
        write_fake_eboot(&corpus.join("WUCHANG.Fallen.Feathers-PPSA09519-EUR-Game(v01.01)-PS5"));

        let catalog = ps5_nid::Catalog::new();
        let options = super::ScanOptions {
            include_prx: false,
            append: false,
        };
        let result = super::scan(&corpus, &dataset, &catalog, &options).unwrap();

        assert_eq!(result.manifest.games.len(), 1);
        assert_eq!(
            result.manifest.games[0].title_id.as_deref(),
            Some("PPSA09519")
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn manifest_has_no_duplicate_titles_after_append() {
        let tmp = scan_test_root("dedup");
        let corpus = tmp.join("corpus");
        let dataset = tmp.join("dataset");
        write_fake_eboot(&corpus.join("GameA-PPSA10001-USA-Game-PS5"));
        write_fake_eboot(&corpus.join("GameB-PPSA10002-USA-Game-PS5"));

        let catalog = ps5_nid::Catalog::new();
        let first = super::ScanOptions {
            include_prx: false,
            append: false,
        };
        super::scan(&corpus, &dataset, &catalog, &first).unwrap();
        let again = super::ScanOptions {
            include_prx: false,
            append: true,
        };
        let result = super::scan(&corpus, &dataset, &catalog, &again).unwrap();

        let mut ids: Vec<_> = result
            .manifest
            .games
            .iter()
            .filter_map(|g| g.title_id.clone())
            .collect();
        ids.sort();
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids, deduped, "manifest must not duplicate title IDs");
        assert_eq!(result.manifest.image_count, 2);
        assert_eq!(result.manifest.games.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
