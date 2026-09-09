use std::path::Path;

use ps5_schema::ShaderRecord;

pub fn inventory_shaders_for_game(game_dir: &Path, game_name: &str) -> Vec<ShaderRecord> {
    let mut out = Vec::new();

    for bin in ps5_shader::ShaderBinary::from_roms(&game_dir.to_string_lossy()) {
        out.push(ShaderRecord {
            name: format!("{}_sb", game_name),
            stage: bin.stage.as_str().to_string(),
            game: game_name.to_string(),
            path: String::new(),
            format: "sb".to_string(),
            hash: bin.hash.clone(),
            size: bin.size,
            entry_point: String::new(),
            resources: Vec::new(),
            debug_info: None,
            status: "parsed".to_string(),
            confidence: 80,
        });
    }

    for cache in ps5_shader::GlobalShaderCache::from_roms(&game_dir.to_string_lossy()) {
        out.push(ShaderRecord {
            name: format!("{game_name}_globalcache"),
            stage: "unknown".to_string(),
            game: game_name.to_string(),
            path: String::new(),
            format: "globalcache".to_string(),
            hash: String::new(),
            size: cache.data.len(),
            entry_point: String::new(),
            resources: Vec::new(),
            debug_info: None,
            status: "parsed".to_string(),
            confidence: 60,
        });
    }

    for archive in ps5_shader::ShaderArchive::from_roms(&game_dir.to_string_lossy()) {
        for (idx, entry) in archive.shader_entries.iter().enumerate() {
            let stage = match entry.frequency {
                0 => "vertex",
                1 => "pixel",
                2 => "geometry",
                3 => "compute",
                _ => "unknown",
            };
            out.push(ShaderRecord {
                name: format!("{game_name}_archive_{idx}"),
                stage: stage.to_string(),
                game: game_name.to_string(),
                path: String::new(),
                format: "ushaderbytecode".to_string(),
                hash: String::new(),
                size: entry.size as usize,
                entry_point: String::new(),
                resources: Vec::new(),
                debug_info: None,
                status: "parsed".to_string(),
                confidence: 90,
            });
        }
    }

    if let Some(unity) = crate::unity::inventory_unity_game(game_dir) {
        for (ty, count) in unity.by_type {
            if count == 0 {
                continue;
            }
            out.push(ShaderRecord {
                name: format!("{game_name}_unity_{ty}"),
                stage: "unknown".to_string(),
                game: game_name.to_string(),
                path: String::new(),
                format: "unity".to_string(),
                hash: String::new(),
                size: 0,
                entry_point: String::new(),
                resources: vec![format!("{ty}:{count}")],
                debug_info: Some(format!("unity_files:{}", unity.unity_files)),
                status: unity.confidence.clone(),
                confidence: if unity.confidence == "verified" {
                    90
                } else {
                    40
                },
            });
        }
    }

    if let Some(dragon) = crate::dragon::inventory_dragon_game(game_dir)
        && dragon.counts.gmd_models > 0
    {
        out.push(ShaderRecord {
            name: format!("{game_name}_dragon_gmd"),
            stage: "unknown".to_string(),
            game: game_name.to_string(),
            path: String::new(),
            format: "gmd".to_string(),
            hash: String::new(),
            size: 0,
            entry_point: String::new(),
            resources: vec![format!("gmd:{}", dragon.counts.gmd_models)],
            debug_info: None,
            status: "heuristic".to_string(),
            confidence: 30,
        });
    }

    out
}

pub fn inventory_shaders_corpus(root: &Path) -> Vec<ShaderRecord> {
    let mut all = Vec::new();
    let game_dirs = crate::scanner::find_game_dirs_for_artifacts(root);
    for dir in game_dirs {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        all.extend(inventory_shaders_for_game(&dir, name));
    }
    all
}
