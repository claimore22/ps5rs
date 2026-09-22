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

pub fn scan(
    root: &Path,
    output: &Path,
    catalog: &Catalog,
    options: &ScanOptions,
) -> Result<ScanResult, std::io::Error> {
    let images_dir = output.join("images");
    if options.append {
        std::fs::create_dir_all(&images_dir)?;
    } else {
        if images_dir.exists() {
            for entry in std::fs::read_dir(&images_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if entry.path().extension().is_some_and(|e| e == "json") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        std::fs::create_dir_all(&images_dir)?;
    }

    let mut image_paths = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut game_params: Vec<GameParam> = Vec::new();
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
            game_params = manifest.games;
        }
    }

    let game_dirs = find_game_dirs(root);
    for game_dir in &game_dirs {
        let game_name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let safe_name = sanitize_filename(game_name);
        if options.append && seen_names.contains(&safe_name) {
            continue;
        }
        let collector_opts = CollectorOptions {
            include_prx: options.include_prx,
        };
        let binaries = find_binaries(game_dir, &collector_opts);
        for bin_path in &binaries {
            if seen_names.contains(&safe_name) {
                continue;
            }
            let data = match std::fs::read(bin_path) {
                Ok(data) => data,
                Err(_) => continue,
            };
            let sha256 = ps5_format::sha256_hex(&data);
            let image = BinaryImageBuilder::build_from_file(&data, &sha256, catalog);
            let doc = assemble_image_document(image, &data, image_type_for(bin_path));
            seen_names.insert(safe_name.clone());
            let json_path = output.join("images").join(format!("{safe_name}.json"));
            let json = serde_json::to_string_pretty(&doc)?;
            std::fs::write(&json_path, format!("{json}\n"))?;
            image_paths.push(json_path);

            let param = param_json::read_param(game_dir).unwrap_or_default();
            let mut param = param;
            if param.name.is_none() {
                param.name = Some(game_name.to_string());
            }
            game_params.push(param);
        }
    }

    let mut shaders: Vec<ps5_schema::ShaderRecord> = Vec::new();
    let mut shadered_games: std::collections::HashSet<String> = std::collections::HashSet::new();
    if options.append
        && let Ok(data) = std::fs::read(output.join("shaders.json"))
        && let Ok(existing) = serde_json::from_slice::<Vec<ps5_schema::ShaderRecord>>(&data)
    {
        for record in &existing {
            shadered_games.insert(record.game.clone());
        }
        shaders = existing;
    }
    for game_dir in &game_dirs {
        let name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        if options.append && shadered_games.contains(name) {
            continue;
        }
        shaders.extend(crate::shader_inventory::inventory_shaders_for_game(
            game_dir, name,
        ));
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
}
