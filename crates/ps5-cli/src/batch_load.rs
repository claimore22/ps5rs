use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use ps5_loader::OfflineExportTable;

use crate::fingerprint::{self, ReportFreshness, Staleness};
use crate::load;

fn try_get_elf_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() >= 4 && &data[0..4] == b"\x7fELF" {
        return Ok(data.to_vec());
    }
    ps5_self::extract::extract_elf(data)
        .map(|r| r.elf)
        .map_err(|e| format!("SELF extraction failed: {e}"))
}

fn scan_prx_dir(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return entries;
    };
    for entry in read_dir {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().map(|s| s.to_string_lossy().to_string())
        {
            entries.push((name, path));
        }
    }
    entries
}

fn has_eboot(dir: &Path) -> bool {
    dir.join("eboot.bin").exists()
}

fn resolve_game_dir(dir: &Path, result: &mut Vec<PathBuf>) {
    if has_eboot(dir) {
        result.push(dir.to_owned());
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let subdirs: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    if subdirs.len() == 1 {
        resolve_game_dir(&subdirs[0].path(), result);
    }
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    let mut games = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        eprintln!("error: cannot read games directory: {}", root.display());
        return games;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            resolve_game_dir(&path, &mut games);
        }
    }
    games.sort();
    games
}

fn sanitize_name(dir: &Path) -> String {
    dir.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn game_display_name(dir: &Path) -> String {
    let param = ps5_analysis::param_json::read_param(dir);
    if let Some(ref p) = param
        && let Some(name) = p.compute_display_name()
    {
        return name;
    }
    sanitize_name(dir)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameLoadReport {
    game: String,
    path: String,
    load_report: Option<load::LoadReport>,
    error: Option<String>,
    #[serde(default)]
    total_exports: usize,
    #[serde(default)]
    freshness: Option<ReportFreshness>,
}

fn report_file_name(display: &str) -> String {
    display.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_")
}

#[derive(Serialize)]
struct UnavailableEntry {
    module: String,
    game_count: usize,
    games: Vec<String>,
}

#[derive(Serialize)]
struct WorstEntry {
    game: String,
    stubbed: u32,
    total: u32,
    rate: f64,
}

#[derive(Serialize)]
struct LoadSummary {
    total_games: usize,
    successful: usize,
    failed: usize,
    total_modules: usize,
    total_exports: usize,
    total_imports_resolved: u64,
    total_imports_known: u64,
    total_imports_stubbed: u64,
    avg_resolution_rate: f64,
    top_unavailable: Vec<UnavailableEntry>,
    worst_games: Vec<WorstEntry>,
}

fn find_prx_bytes<'a>(name: &str, files: &'a [(String, Vec<u8>)]) -> Option<&'a [u8]> {
    if let Some((_, bytes)) = files.iter().find(|(f, _)| f == name) {
        return Some(bytes);
    }
    let with_self = format!("{name}.self");
    if let Some((_, bytes)) = files.iter().find(|(f, _)| f == &with_self) {
        return Some(bytes);
    }
    let lower = name.to_lowercase();
    files
        .iter()
        .find(|(f, _)| f.to_lowercase() == lower)
        .map(|(_, bytes)| bytes.as_slice())
}

fn read_prx_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    for (name, path) in scan_prx_dir(dir) {
        if let Ok(bytes) = std::fs::read(&path) {
            out.push((name, bytes));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn purge_stale_reports(games_out: &Path, expected: &std::collections::HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(games_out) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_json = path.extension().is_some_and(|e| e == "json");
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if is_json && !expected.contains(&name) && std::fs::remove_file(&path).is_ok() {
            eprintln!("PURGED stale report {name}");
        }
    }
}

/// Attempt to reuse a stored report without invoking the loader.
/// Returns the stored report (with display metadata refreshed to current
/// values) iff its fingerprint proves freshness; otherwise returns the
/// reason it must be reloaded.
fn try_reuse(
    games_out: &Path,
    report_name: &str,
    env_fp: &str,
    game_fp: &str,
    display: &str,
    eboot_display: &str,
) -> Result<GameLoadReport, Staleness> {
    let data =
        std::fs::read(games_out.join(report_name)).map_err(|_| Staleness::GameInputsChanged)?;
    let mut stored: GameLoadReport =
        serde_json::from_slice(&data).map_err(|_| Staleness::GameInputsChanged)?;
    let freshness = stored
        .freshness
        .as_ref()
        .ok_or(Staleness::EnvironmentChanged)?;
    let status = fingerprint::check_freshness(freshness, env_fp, game_fp);
    if status != Staleness::Fresh {
        return Err(status);
    }
    if stored.error.is_some() || stored.load_report.is_none() {
        return Err(Staleness::GameInputsChanged);
    }
    stored.game = display.to_string();
    stored.path = eboot_display.to_string();
    Ok(stored)
}

fn staleness_reason(s: Staleness) -> &'static str {
    match s {
        Staleness::Fresh => "fresh",
        Staleness::FormatChanged => "report format changed",
        Staleness::EnvironmentChanged => "environment changed",
        Staleness::GameInputsChanged => "game inputs changed",
    }
}

pub(crate) fn cmd_batch_load(
    games_dir: &Path,
    output_dir: &Path,
    offline_dir: &Path,
    json: bool,
    force: bool,
) {
    let games = find_game_dirs(games_dir);
    if games.is_empty() {
        eprintln!(
            "error: no games found (no eboot.bin) in: {}",
            games_dir.display()
        );
        std::process::exit(1);
    }

    eprintln!("Found {} game(s) in {}", games.len(), games_dir.display());

    let games_out = output_dir.join("games");
    if !json && let Err(e) = std::fs::create_dir_all(&games_out) {
        eprintln!(
            "error: cannot create output directory {}: {e}",
            games_out.display()
        );
        std::process::exit(1);
    }

    let loader_fp = fingerprint::loader_fingerprint();
    let offline_fp = match fingerprint::offline_fingerprint(offline_dir) {
        Ok(fp) => fp,
        Err(e) => {
            eprintln!("error: cannot fingerprint offline dir: {e}");
            std::process::exit(1);
        }
    };
    let env_fp = fingerprint::environment_fingerprint(&loader_fp, &offline_fp);
    if !offline_dir.is_dir() {
        eprintln!("note: no offline export dir, imports resolve without offline data");
    }

    let offline_table = if offline_dir.is_dir() {
        let table = OfflineExportTable::load_from_dir(offline_dir);
        if !table.is_empty() { Some(table) } else { None }
    } else {
        None
    };

    let mut reports: Vec<GameLoadReport> = Vec::new();
    let mut expected_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_resolved: u64 = 0;
    let mut total_known: u64 = 0;
    let mut total_stubbed: u64 = 0;
    let mut total_modules: usize = 0;
    let mut total_exports: usize = 0;
    let mut all_unavailable: HashMap<String, Vec<String>> = HashMap::new();
    let mut reused: usize = 0;
    let mut reloaded: usize = 0;

    for (idx, game_dir) in games.iter().enumerate() {
        let name = sanitize_name(game_dir);
        let display = game_display_name(game_dir);
        let eboot_path = game_dir.join("eboot.bin");
        let report_name = format!("{}.json", report_file_name(&display));
        expected_files.insert(report_name.clone());

        eprint!("[{}/{}] {} ... ", idx + 1, games.len(), display);

        let data = match std::fs::read(&eboot_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("FAILED (read: {e})");
                reports.push(GameLoadReport {
                    game: display,
                    path: eboot_path.to_string_lossy().to_string(),
                    load_report: None,
                    error: Some(format!("read failed: {e}")),
                    total_exports: 0,
                    freshness: None,
                });
                reloaded += 1;
                continue;
            }
        };

        let elf_bytes = match try_get_elf_bytes(&data) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("FAILED ({e})");
                reports.push(GameLoadReport {
                    game: display,
                    path: eboot_path.to_string_lossy().to_string(),
                    load_report: None,
                    error: Some(e),
                    total_exports: 0,
                    freshness: None,
                });
                reloaded += 1;
                continue;
            }
        };

        let prx_files = read_prx_bytes(&game_dir.join("sce_module"));
        let game_fp = fingerprint::game_fingerprint(&env_fp, &elf_bytes, &prx_files);

        if !force && !json {
            let report_path = games_out.join(&report_name);
            if !report_path.exists() {
                eprintln!("RELOADED (no report)");
            } else {
                let eboot_display = eboot_path.to_string_lossy().to_string();
                match try_reuse(
                    &games_out,
                    &report_name,
                    &env_fp,
                    &game_fp,
                    &display,
                    &eboot_display,
                ) {
                    Ok(stored) => {
                        eprintln!("REUSED (fresh)");
                        let lr = stored
                            .load_report
                            .as_ref()
                            .expect("try_reuse only returns reports with content");
                        total_resolved += lr.totals.resolved as u64;
                        total_known += lr.totals.known as u64;
                        total_stubbed += lr.totals.stubbed as u64;
                        total_modules += lr.modules.len();
                        total_exports += stored.total_exports;
                        for unavailable in &lr.graph.unavailable {
                            all_unavailable
                                .entry(unavailable.clone())
                                .or_default()
                                .push(display.clone());
                        }
                        reports.push(stored);
                        reused += 1;
                        continue;
                    }
                    Err(status) => {
                        eprintln!("RELOADED ({})", staleness_reason(status));
                    }
                }
            }
        }

        let ctx = match ps5_loader::load_modules(
            &name,
            &elf_bytes,
            |mod_name| {
                find_prx_bytes(mod_name, &prx_files)
                    .and_then(|contents| try_get_elf_bytes(contents).ok())
            },
            offline_table.as_ref(),
        ) {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("FAILED (load: {e})");
                reports.push(GameLoadReport {
                    game: display,
                    path: eboot_path.to_string_lossy().to_string(),
                    load_report: None,
                    error: Some(format!("load failed: {e}")),
                    total_exports: 0,
                    freshness: None,
                });
                reloaded += 1;
                continue;
            }
        };

        let report = load::build_report(&ctx);
        total_resolved += ctx.resolved_imports as u64;
        total_known += ctx.known_imports as u64;
        total_stubbed += ctx.stubbed_imports as u64;
        total_modules += ctx.modules.len();
        total_exports += ctx.exports.len();

        let unavailable: Vec<String> = ctx
            .graph
            .unavailable_modules()
            .map(|s| s.to_string())
            .collect();
        for mod_name in &unavailable {
            all_unavailable
                .entry(mod_name.clone())
                .or_default()
                .push(display.clone());
        }

        eprintln!(
            "OK ({} modules, {} resolved/{} known/{} stubbed)",
            ctx.modules.len(),
            ctx.resolved_imports,
            ctx.known_imports,
            ctx.stubbed_imports
        );

        reports.push(GameLoadReport {
            game: display,
            path: eboot_path.to_string_lossy().to_string(),
            load_report: Some(report),
            error: None,
            total_exports: ctx.exports.len(),
            freshness: Some(ReportFreshness {
                report_format: fingerprint::REPORT_FORMAT,
                loader_fingerprint: loader_fp.clone(),
                offline_fingerprint: offline_fp.clone(),
                environment_fingerprint: env_fp.clone(),
                game_fingerprint: game_fp,
            }),
        });
        reloaded += 1;
    }

    if !json {
        purge_stale_reports(&games_out, &expected_files);
    }

    let successful = reports.iter().filter(|r| r.load_report.is_some()).count();
    let failed = reports.len() - successful;

    let mut top_unavailable: Vec<UnavailableEntry> = all_unavailable
        .into_iter()
        .map(|(module, games)| UnavailableEntry {
            game_count: games.len(),
            games,
            module,
        })
        .collect();
    top_unavailable.sort_by(|a, b| {
        Reverse(a.game_count)
            .cmp(&Reverse(b.game_count))
            .then_with(|| a.module.cmp(&b.module))
    });

    let mut worst_games: Vec<WorstEntry> = reports
        .iter()
        .filter_map(|r| {
            r.load_report.as_ref().map(|lr| {
                let total = lr.totals.resolved + lr.totals.known + lr.totals.stubbed;
                let stub_rate = if total > 0 {
                    (lr.totals.stubbed as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                WorstEntry {
                    game: r.game.clone(),
                    stubbed: lr.totals.stubbed,
                    total,
                    rate: stub_rate,
                }
            })
        })
        .collect();
    worst_games.sort_by(|a, b| {
        b.rate
            .partial_cmp(&a.rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.game.cmp(&b.game))
    });

    let total_imports = total_resolved + total_known + total_stubbed;
    let avg_rate = if total_imports > 0 {
        ((total_resolved + total_known) as f64 / total_imports as f64) * 100.0
    } else {
        100.0
    };

    let summary = LoadSummary {
        total_games: games.len(),
        successful,
        failed,
        total_modules,
        total_exports,
        total_imports_resolved: total_resolved,
        total_imports_known: total_known,
        total_imports_stubbed: total_stubbed,
        avg_resolution_rate: avg_rate,
        top_unavailable: top_unavailable.into_iter().take(20).collect(),
        worst_games: worst_games.into_iter().take(10).collect(),
    };

    if json {
        let output = serde_json::json!({
            "games": reports,
            "summary": summary,
        });
        let json_str = serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
            eprintln!("error: JSON serialization failed: {e}");
            std::process::exit(1);
        });
        println!("{json_str}");
        return;
    }

    if let Err(e) = std::fs::create_dir_all(output_dir) {
        eprintln!(
            "error: cannot create output directory {}: {e}",
            output_dir.display()
        );
        std::process::exit(1);
    }
    let games_out = output_dir.join("games");
    if let Err(e) = std::fs::create_dir_all(&games_out) {
        eprintln!(
            "error: cannot create games output directory {}: {e}",
            games_out.display()
        );
        std::process::exit(1);
    }

    for report in &reports {
        let report_path = games_out.join(format!("{}.json", report_file_name(&report.game)));
        if let Ok(json_str) = serde_json::to_string_pretty(report)
            && let Err(e) = std::fs::write(&report_path, &json_str)
        {
            eprintln!("error: cannot write report {}: {e}", report_path.display());
        }
    }

    let summary_path = output_dir.join("summary.json");
    if let Ok(json_str) = serde_json::to_string_pretty(&summary)
        && let Err(e) = std::fs::write(&summary_path, &json_str)
    {
        eprintln!(
            "error: cannot write summary {}: {e}",
            summary_path.display()
        );
    }

    eprintln!();
    eprintln!("Reports: {reused} reused, {reloaded} reloaded");
    eprintln!("=== Summary ===");
    eprintln!(
        "Total games: {} ({} successful, {} failed)",
        summary.total_games, summary.successful, summary.failed
    );
    eprintln!("Total modules loaded: {}", summary.total_modules);
    eprintln!("Total exports: {}", summary.total_exports);
    eprintln!(
        "Total imports resolved: {} known: {} stubbed: {}",
        summary.total_imports_resolved, summary.total_imports_known, summary.total_imports_stubbed
    );
    eprintln!(
        "Average resolution rate: {:.1}%",
        summary.avg_resolution_rate
    );
    eprintln!("Output: {}", output_dir.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::REPORT_FORMAT;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ps5rs_batchload_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fresh_stamp(env_fp: &str, game_fp: &str) -> ReportFreshness {
        ReportFreshness {
            report_format: REPORT_FORMAT,
            loader_fingerprint: "loader".to_string(),
            offline_fingerprint: "offline".to_string(),
            environment_fingerprint: env_fp.to_string(),
            game_fingerprint: game_fp.to_string(),
        }
    }

    fn stored_report(game: &str, freshness: Option<ReportFreshness>) -> GameLoadReport {
        let load_report: load::LoadReport = serde_json::from_value(serde_json::json!({
            "modules": [],
            "graph": {"nodes": [], "unavailable": ["libFoo"], "edges": []},
            "totals": {"modules": 0, "resolved": 1, "known": 2, "stubbed": 3,
                       "exports": 0, "unavailable": 1}
        }))
        .unwrap();
        GameLoadReport {
            game: game.to_string(),
            path: "old/path".to_string(),
            load_report: Some(load_report),
            error: None,
            total_exports: 0,
            freshness,
        }
    }

    #[test]
    fn try_reuse_accepts_fresh_report_and_refreshes_metadata() {
        let dir = test_dir("reuse_ok");
        let report = stored_report("Game", Some(fresh_stamp("env", "game")));
        std::fs::write(
            dir.join("Game.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();
        let reused = try_reuse(&dir, "Game.json", "env", "game", "Game", "new/path").unwrap();
        assert_eq!(reused.game, "Game");
        assert_eq!(reused.path, "new/path");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn try_reuse_rejects_each_staleness_cause() {
        let dir = test_dir("reuse_reject");
        let mut report = stored_report("Game", Some(fresh_stamp("env", "game")));
        let path = dir.join("Game.json");

        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        assert_eq!(
            try_reuse(&dir, "Game.json", "env", "other", "Game", "p").unwrap_err(),
            Staleness::GameInputsChanged
        );
        assert_eq!(
            try_reuse(&dir, "Game.json", "other", "game", "Game", "p").unwrap_err(),
            Staleness::EnvironmentChanged
        );

        report.freshness.as_mut().unwrap().report_format = REPORT_FORMAT + 1;
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        assert_eq!(
            try_reuse(&dir, "Game.json", "env", "game", "Game", "p").unwrap_err(),
            Staleness::FormatChanged
        );

        report.freshness = None;
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
        assert!(try_reuse(&dir, "Game.json", "env", "game", "Game", "p").is_err());

        std::fs::write(&path, b"{not json").unwrap();
        assert!(try_reuse(&dir, "Game.json", "env", "game", "Game", "p").is_err());

        assert!(try_reuse(&dir, "Missing.json", "env", "game", "Game", "p").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn try_reuse_rejects_error_reports() {
        let dir = test_dir("reuse_error");
        let mut report = stored_report("Game", Some(fresh_stamp("env", "game")));
        report.error = Some("boom".to_string());
        std::fs::write(
            dir.join("Game.json"),
            serde_json::to_string_pretty(&report).unwrap(),
        )
        .unwrap();
        assert!(try_reuse(&dir, "Game.json", "env", "game", "Game", "p").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn purge_removes_only_unexpected_files() {
        let dir = test_dir("purge");
        std::fs::write(dir.join("Keep.json"), b"{}").unwrap();
        std::fs::write(dir.join("Stale.json"), b"{}").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hi").unwrap();
        let mut expected = std::collections::HashSet::new();
        expected.insert("Keep.json".to_string());
        purge_stale_reports(&dir, &expected);
        assert!(dir.join("Keep.json").exists());
        assert!(!dir.join("Stale.json").exists());
        assert!(dir.join("notes.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_file_name_sanitizes_display_names() {
        assert_eq!(
            report_file_name("ANIMAL WELL - [PPSA22520]"),
            "ANIMAL_WELL_-_[PPSA22520]"
        );
        assert_eq!(report_file_name("A/B:C"), "A_B_C");
    }
}
