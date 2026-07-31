use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use ps5_loader::OfflineExportTable;

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

fn find_prx<'a>(name: &str, files: &'a [(String, PathBuf)]) -> Option<&'a PathBuf> {
    if let Some((_, path)) = files.iter().find(|(f, _)| f == name) {
        return Some(path);
    }
    let with_self = format!("{name}.self");
    if let Some((_, path)) = files.iter().find(|(f, _)| f == &with_self) {
        return Some(path);
    }
    let lower = name.to_lowercase();
    files
        .iter()
        .find(|(f, _)| f.to_lowercase() == lower)
        .map(|(_, path)| path)
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

#[derive(Serialize)]
struct GameLoadReport {
    game: String,
    path: String,
    load_report: Option<load::LoadReport>,
    error: Option<String>,
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

pub(crate) fn cmd_batch_load(games_dir: &Path, output_dir: &Path, offline_dir: &Path, json: bool) {
    let games = find_game_dirs(games_dir);
    if games.is_empty() {
        eprintln!(
            "error: no games found (no eboot.bin) in: {}",
            games_dir.display()
        );
        std::process::exit(1);
    }

    eprintln!("Found {} game(s) in {}", games.len(), games_dir.display());

    let mut reports: Vec<GameLoadReport> = Vec::new();
    let mut total_resolved: u64 = 0;
    let mut total_known: u64 = 0;
    let mut total_stubbed: u64 = 0;
    let mut total_modules: usize = 0;
    let mut total_exports: usize = 0;
    let mut all_unavailable: HashMap<String, Vec<String>> = HashMap::new();

    for (idx, game_dir) in games.iter().enumerate() {
        let name = sanitize_name(game_dir);
        let display = game_display_name(game_dir);
        let eboot_path = game_dir.join("eboot.bin");

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
                });
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
                });
                continue;
            }
        };

        let prx_dir = game_dir.join("sce_module");
        let prx_files = if prx_dir.is_dir() {
            scan_prx_dir(&prx_dir)
        } else {
            Vec::new()
        };

        let offline_table = if offline_dir.is_dir() {
            let table = OfflineExportTable::load_from_dir(offline_dir);
            if !table.is_empty() { Some(table) } else { None }
        } else {
            None
        };

        let ctx = match ps5_loader::load_modules(
            &name,
            &elf_bytes,
            |mod_name| {
                let found = find_prx(mod_name, &prx_files);
                found.and_then(|p| {
                    let contents = std::fs::read(p).ok()?;
                    try_get_elf_bytes(&contents).ok()
                })
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
                });
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
        });
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
    top_unavailable.sort_by_key(|b| Reverse(b.game_count));

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
        let safe_name = report
            .game
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_");
        let report_path = games_out.join(format!("{safe_name}.json"));
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
