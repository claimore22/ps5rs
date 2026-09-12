use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::catalog::iso8601_now;
use crate::fingerprint;

pub const INGEST_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEntry {
    pub name: String,
    pub eboot_sha256: String,
    pub game_fingerprint: String,
    pub ingest_id: String,
    pub status: String,
    pub ingested_at: String,
    pub source_deleted: bool,
    pub record_path: String,
    pub report_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestRegistry {
    pub schema: u32,
    #[serde(default)]
    pub games: HashMap<String, IngestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub schema: u32,
    pub title_id: String,
    pub name: String,
    pub eboot_sha256: String,
    pub game_fingerprint: String,
    pub environment_fingerprint: String,
    pub loader_fingerprint: String,
    pub offline_fingerprint: String,
    pub ingest_id: String,
    pub ingested_at: String,
    pub params: ps5_analysis::param_json::GameParam,
    pub load_report_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestStatus {
    NotIngested,
    SameIngest,
    NewVersion,
}

pub fn registry_path(dataset: &Path) -> PathBuf {
    dataset.join("ingest.json")
}

pub fn load_registry(dataset: &Path) -> IngestRegistry {
    let path = registry_path(dataset);
    if let Ok(data) = std::fs::read(&path)
        && let Ok(reg) = serde_json::from_slice::<IngestRegistry>(&data)
        && reg.schema == INGEST_SCHEMA
    {
        return reg;
    }
    IngestRegistry {
        schema: INGEST_SCHEMA,
        games: HashMap::new(),
    }
}

fn save_registry(dataset: &Path, registry: &IngestRegistry) -> Result<(), String> {
    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| format!("cannot serialize registry: {e}"))?;
    std::fs::write(registry_path(dataset), format!("{json}\n"))
        .map_err(|e| format!("cannot write registry: {e}"))
}

pub fn registry_status(registry: &IngestRegistry, title_id: &str, game_fp: &str) -> IngestStatus {
    match registry.games.get(title_id) {
        None => IngestStatus::NotIngested,
        Some(entry) if entry.game_fingerprint == game_fp => IngestStatus::SameIngest,
        Some(_) => IngestStatus::NewVersion,
    }
}

fn find_eboot(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("eboot.bin"))
        {
            return Some(path);
        } else if path.is_dir() {
            subdirs.push(path);
        }
    }
    subdirs.sort();
    subdirs.into_iter().find_map(|d| find_eboot(&d, depth + 1))
}

fn read_prx_bytes(game_dir: &Path, eboot_dir: &Path) -> Vec<(String, Vec<u8>)> {
    let prx_dir = {
        let adjacent = eboot_dir.join("sce_module");
        if adjacent.is_dir() {
            adjacent
        } else {
            game_dir.join("sce_module")
        }
    };
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&prx_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().map(|s| s.to_string_lossy().to_string())
            && let Ok(bytes) = std::fs::read(&path)
        {
            out.push((name, bytes));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn short_fp(fp: &str) -> &str {
    fp.get(..12).unwrap_or(fp)
}

/// Archive one game: write the per-game record, reload it from disk,
/// verify it, then commit the registry entry. Sources are never touched;
/// deletion is a separate explicit step (`--mark-deleted`).
pub fn archive_game(
    game_dir: &Path,
    dataset: &Path,
    offline_dir: &Path,
) -> Result<IngestEntry, String> {
    let eboot_path = find_eboot(game_dir, 0)
        .ok_or_else(|| format!("no eboot.bin under {}", game_dir.display()))?;
    let eboot_dir = eboot_path
        .parent()
        .ok_or_else(|| "eboot has no parent".to_string())?;

    let mut params = ps5_analysis::param_json::read_param(game_dir).unwrap_or_default();
    let title_id = params
        .title_id
        .clone()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            format!(
                "no title_id for {} (cannot key archival record)",
                game_dir.display()
            )
        })?;
    if params.name.is_none() {
        params.name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string);
    }
    let display = params
        .compute_display_name()
        .unwrap_or_else(|| title_id.clone());

    let eboot_bytes = std::fs::read(&eboot_path)
        .map_err(|e| format!("cannot read {}: {e}", eboot_path.display()))?;
    let eboot_sha = ps5_format::sha256_hex(&eboot_bytes);
    let prx_files = read_prx_bytes(game_dir, eboot_dir);

    let loader_fp = fingerprint::loader_fingerprint();
    let offline_fp = fingerprint::offline_fingerprint(offline_dir)?;
    let env_fp = fingerprint::environment_fingerprint(&loader_fp, &offline_fp);
    let game_fp = fingerprint::game_fingerprint(&env_fp, &eboot_bytes, &prx_files);

    let mut registry = load_registry(dataset);
    match registry_status(&registry, &title_id, &game_fp) {
        IngestStatus::SameIngest => {
            let entry = registry.games.get(&title_id).cloned().unwrap();
            eprintln!("ALREADY INGESTED {title_id} ({})", entry.ingest_id);
            return Ok(entry);
        }
        IngestStatus::NewVersion => {
            eprintln!("NEW VERSION of {title_id} (fingerprint differs, keeping prior record)");
        }
        IngestStatus::NotIngested => {}
    }

    let report_name = format!(
        "{}.json",
        display.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_")
    );
    let load_report_src = dataset.join("load").join("games").join(&report_name);
    if !load_report_src.is_file() {
        return Err(format!(
            "no load report for {display} (run batch-load first): {}",
            load_report_src.display()
        ));
    }

    let ingested_at = iso8601_now();
    let ingest_id = format!(
        "ingest_{}_{}",
        ingested_at.replace(['-', ':', 'T', 'Z'], ""),
        short_fp(&game_fp)
    );
    let record_dir = dataset
        .join("games")
        .join(&title_id)
        .join("versions")
        .join(short_fp(&game_fp));
    std::fs::create_dir_all(&record_dir)
        .map_err(|e| format!("cannot create {}: {e}", record_dir.display()))?;

    let record = GameRecord {
        schema: INGEST_SCHEMA,
        title_id: title_id.clone(),
        name: display.clone(),
        eboot_sha256: eboot_sha.clone(),
        game_fingerprint: game_fp.clone(),
        environment_fingerprint: env_fp.clone(),
        loader_fingerprint: loader_fp,
        offline_fingerprint: offline_fp,
        ingest_id: ingest_id.clone(),
        ingested_at: ingested_at.clone(),
        params,
        load_report_file: "load_report.json".to_string(),
    };
    let record_json = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("cannot serialize record: {e}"))?;
    std::fs::write(record_dir.join("game.json"), format!("{record_json}\n"))
        .map_err(|e| format!("cannot write game.json: {e}"))?;
    std::fs::copy(&load_report_src, record_dir.join("load_report.json"))
        .map_err(|e| format!("cannot copy load report: {e}"))?;

    verify_record(&record_dir, &eboot_bytes)?;

    let rel_record = record_dir
        .strip_prefix(dataset)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let entry = IngestEntry {
        name: display,
        eboot_sha256: eboot_sha,
        game_fingerprint: game_fp,
        ingest_id,
        status: "complete".to_string(),
        ingested_at,
        source_deleted: false,
        record_path: rel_record,
        report_file: report_name,
    };
    registry.games.insert(title_id.clone(), entry.clone());
    save_registry(dataset, &registry)?;
    eprintln!("ARCHIVED {title_id} -> {}", entry.record_path);
    Ok(entry)
}

/// Reload a written record from disk and prove it matches the live inputs.
/// Any failure means the archival is void and sources must be kept.
fn verify_record(record_dir: &Path, eboot_bytes: &[u8]) -> Result<(), String> {
    let data = std::fs::read(record_dir.join("game.json"))
        .map_err(|e| format!("verify: cannot re-read game.json: {e}"))?;
    let record: GameRecord =
        serde_json::from_slice(&data).map_err(|e| format!("verify: game.json invalid: {e}"))?;
    if record.schema != INGEST_SCHEMA {
        return Err(format!("verify: schema mismatch ({})", record.schema));
    }
    if record.title_id.is_empty() {
        return Err("verify: empty title_id".to_string());
    }
    let live_sha = ps5_format::sha256_hex(eboot_bytes);
    if live_sha != record.eboot_sha256 {
        return Err("verify: eboot sha mismatch".to_string());
    }
    let report_data = std::fs::read(record_dir.join(&record.load_report_file))
        .map_err(|e| format!("verify: cannot re-read load report: {e}"))?;
    let report: serde_json::Value = serde_json::from_slice(&report_data)
        .map_err(|e| format!("verify: load report invalid: {e}"))?;
    let game = report
        .get("game")
        .and_then(|g| g.as_str())
        .unwrap_or_default();
    if game != record.name {
        return Err(format!(
            "verify: load report game mismatch ({game} vs {})",
            record.name
        ));
    }
    eprintln!("VERIFIED {}", record.title_id);
    Ok(())
}

pub fn cmd_archive(
    game: &Option<PathBuf>,
    dataset: &Path,
    offline_dir: &Path,
    mark_deleted: &Option<String>,
) {
    if let Some(title_id) = mark_deleted {
        let mut registry = load_registry(dataset);
        match registry.games.get_mut(title_id) {
            Some(entry) if entry.status == "complete" => {
                entry.source_deleted = true;
                if save_registry(dataset, &registry).is_err() {
                    eprintln!("error: cannot save registry");
                    std::process::exit(1);
                }
                eprintln!("MARKED DELETED {title_id}");
            }
            Some(_) => {
                eprintln!("error: {title_id} ingest not complete, refusing");
                std::process::exit(1);
            }
            None => {
                eprintln!("error: {title_id} not in registry, refusing");
                std::process::exit(1);
            }
        }
        return;
    }
    let Some(game_dir) = game else {
        eprintln!("error: archive requires --game <dir> or --mark-deleted <title-id>");
        std::process::exit(1);
    };
    match archive_game(game_dir, dataset, offline_dir) {
        Ok(entry) => {
            eprintln!("ingest_id: {}", entry.ingest_id);
        }
        Err(e) => {
            eprintln!("error: archive failed: {e}");
            eprintln!("sources kept (no successful archival record => no deletion)");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_status_distinguishes_cases() {
        let mut reg = IngestRegistry {
            schema: INGEST_SCHEMA,
            games: HashMap::new(),
        };
        assert_eq!(
            registry_status(&reg, "PPSA00001", "fp1"),
            IngestStatus::NotIngested
        );
        reg.games.insert(
            "PPSA00001".to_string(),
            IngestEntry {
                name: "Game".to_string(),
                eboot_sha256: "abc".to_string(),
                game_fingerprint: "fp1".to_string(),
                ingest_id: "ingest_1".to_string(),
                status: "complete".to_string(),
                ingested_at: "t".to_string(),
                source_deleted: false,
                record_path: "games/PPSA00001".to_string(),
                report_file: "Game.json".to_string(),
            },
        );
        assert_eq!(
            registry_status(&reg, "PPSA00001", "fp1"),
            IngestStatus::SameIngest
        );
        assert_eq!(
            registry_status(&reg, "PPSA00001", "fp2"),
            IngestStatus::NewVersion
        );
        assert_eq!(
            registry_status(&reg, "PPSA99999", "fp1"),
            IngestStatus::NotIngested
        );
    }

    #[test]
    fn registry_roundtrip_and_bad_schema_falls_back_empty() {
        let dir = std::env::temp_dir().join(format!("ps5rs_ingest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut reg = IngestRegistry {
            schema: INGEST_SCHEMA,
            games: HashMap::new(),
        };
        reg.games.insert(
            "PPSA00001".to_string(),
            IngestEntry {
                name: "Game".to_string(),
                eboot_sha256: "abc".to_string(),
                game_fingerprint: "fp1".to_string(),
                ingest_id: "ingest_1".to_string(),
                status: "complete".to_string(),
                ingested_at: "t".to_string(),
                source_deleted: true,
                record_path: "x".to_string(),
                report_file: "Game.json".to_string(),
            },
        );
        save_registry(&dir, &reg).unwrap();
        let back = load_registry(&dir);
        assert!(back.games["PPSA00001"].source_deleted);
        std::fs::write(registry_path(&dir), b"{\"schema\": 999}").unwrap();
        assert!(load_registry(&dir).games.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
