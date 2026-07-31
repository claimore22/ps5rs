use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::catalog::{iso8601_now, load_catalog};
use crate::util::write_to_output_or_stdout;

const SCHEMA_VERSION: u32 = 1;

const MAX_EBOOT_DEPTH: usize = 6;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    fn as_str(&self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

#[derive(Serialize)]
struct UnknownNidsReport {
    schema_version: u32,
    tool: &'static str,
    games_dir: String,
    generated_at: String,
    summary: Summary,
    games: Vec<GameReport>,
    unknown_nids: Vec<UnknownNidEntry>,
    library_breakdown: Vec<LibraryCount>,
    remu_comparison: Option<RemuComparison>,
    extraction_diffs: Vec<ExtractionDiff>,
}

#[derive(Serialize)]
struct Summary {
    games_found: usize,
    games_parsed: usize,
    games_failed: usize,
    imports_total: usize,
    known_total: usize,
    unknown_total: usize,
    unique_unknown_nids: usize,
}

#[derive(Serialize)]
struct GameReport {
    name: String,
    file: String,
    imports: usize,
    known: usize,
    unknown: usize,
    unknown_nids: Vec<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct UnknownNidEntry {
    nid: String,
    frequency: usize,
    libraries: Vec<String>,
    games: Vec<String>,
    remu_name: Option<String>,
    confidence: Confidence,
}

#[derive(Serialize)]
struct LibraryCount {
    library: String,
    unknown_count: usize,
}

#[derive(Serialize)]
struct RemuComparison {
    enabled: bool,
    remu_known_not_ours: Vec<RemuNameEntry>,
    neither_known: Vec<String>,
}

#[derive(Serialize)]
struct RemuNameEntry {
    nid: String,
    remu_name: String,
}

#[derive(Serialize)]
struct ExtractionDiff {
    game: String,
    nids_in_remu_not_native: Vec<String>,
}

#[derive(Default)]
struct Acc {
    frequency: usize,
    libraries: HashSet<String>,
    games: HashSet<String>,
}

struct RemuInventory {
    resolved: HashMap<String, String>,
    all: HashSet<String>,
}

fn is_nid(value: &str) -> bool {
    value.len() == 11
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-')
}

/// Confidence ranks evidence: a library association is one point, a REmu name
/// is two, and wide adoption is two more. Scores below four without a REmu
/// name are research candidates rather than safe catalog imports.
fn confidence(
    frequency: usize,
    has_library: bool,
    has_remu_name: bool,
    games_scanned: usize,
) -> Confidence {
    let mut score = 0usize;
    if has_library {
        score += 1;
    }
    if has_remu_name {
        score += 2;
    }
    if frequency >= 3 {
        score += 1;
    }
    if games_scanned > 0 && frequency * 10 >= games_scanned * 3 {
        score += 2;
    }
    if score >= 4 {
        Confidence::High
    } else if score > 0 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn collect_eboots(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if dir.join("eboot.bin").is_file() {
        out.push(dir.join("eboot.bin"));
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            collect_eboots(&entry.path(), out, depth - 1);
        }
    }
}

fn is_variant_path(rel: &Path) -> bool {
    rel.components().any(|c| {
        let name = c.as_os_str().to_string_lossy().to_lowercase();
        name.contains("decrypted") || name.contains("patch") || name.contains("union")
    })
}

fn select_eboot(game_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    collect_eboots(game_dir, &mut candidates, MAX_EBOOT_DEPTH);
    candidates.retain(|p| {
        let rel = p.strip_prefix(game_dir).unwrap_or(p);
        !is_variant_path(rel)
    });
    candidates.sort_by_key(|p| {
        let depth = p
            .strip_prefix(game_dir)
            .map_or(usize::MAX, |r| r.components().count());
        (depth, p.to_string_lossy().to_lowercase())
    });
    candidates.into_iter().next()
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        eprintln!("error: cannot read games directory: {}", root.display());
        return Vec::new();
    };
    let mut games: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    games.sort();
    games
}

fn display_name(game_dir: &Path) -> String {
    let param = ps5_analysis::param_json::read_param(game_dir);
    if let Some(name) = param.and_then(|p| p.compute_display_name()) {
        return name;
    }
    game_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn record_unknown(agg: &mut BTreeMap<String, Acc>, nid: &str, library: Option<&str>, game: &str) {
    let acc = agg.entry(nid.to_string()).or_default();
    acc.frequency += 1;
    acc.games.insert(game.to_string());
    if let Some(lib) = library {
        acc.libraries.insert(lib.to_string());
    }
}

fn parse_remu_output(output: &str) -> RemuInventory {
    let mut resolved = HashMap::new();
    let mut all = HashSet::new();
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("plt[") {
            if let Some(idx_end) = rest.find(']') {
                let fields = &rest[idx_end + 1..];
                let nid = fields
                    .split_once(" nid:")
                    .and_then(|(_, s)| s.split_once(" resolved:"))
                    .map(|(nid, _)| nid);
                let resolved_name = fields
                    .split_once(" resolved:")
                    .and_then(|(_, s)| s.split_once(" name:"))
                    .map(|(name, _)| name);
                if let Some(nid) = nid
                    && is_nid(nid)
                {
                    all.insert(nid.to_string());
                    if let Some(name) = resolved_name
                        && name != "unknown"
                    {
                        resolved.insert(nid.to_string(), name.to_string());
                    }
                }
            }
            continue;
        }
        if let Some(symbol) = line.strip_prefix("import=") {
            let symbol = symbol.split_once(" symbol=").map_or(symbol, |(s, _)| s);
            let nid = symbol.split_once('#').map_or(symbol, |(n, _)| n);
            if is_nid(nid) {
                all.insert(nid.to_string());
            }
        }
    }
    RemuInventory { resolved, all }
}

fn resolve_remu_binary(path: &Path) -> PathBuf {
    if path.is_dir() {
        let name = if cfg!(windows) { "remu.exe" } else { "remu" };
        return path.join(name);
    }
    path.to_owned()
}

fn run_remu_imports(remu: &Path, eboot: &Path) -> Option<String> {
    let output = std::process::Command::new(remu)
        .arg("imports")
        .arg(eboot)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    }
}

fn build_entries(
    agg: BTreeMap<String, Acc>,
    games_scanned: usize,
    global_resolved: &HashMap<String, String>,
) -> Vec<UnknownNidEntry> {
    let mut entries: Vec<UnknownNidEntry> = agg
        .into_iter()
        .map(|(nid, acc)| {
            let mut libraries: Vec<String> = acc.libraries.into_iter().collect();
            libraries.sort();
            let mut games: Vec<String> = acc.games.into_iter().collect();
            games.sort();
            let remu_name = global_resolved.get(&nid).cloned();
            let confidence = confidence(
                acc.frequency,
                !libraries.is_empty(),
                remu_name.is_some(),
                games_scanned,
            );
            UnknownNidEntry {
                nid,
                frequency: acc.frequency,
                libraries,
                games,
                remu_name,
                confidence,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.frequency
            .cmp(&a.frequency)
            .then_with(|| a.nid.cmp(&b.nid))
    });
    entries
}

pub(crate) fn cmd_unknown_nids(
    games_dir: &Path,
    remu: &Option<PathBuf>,
    json: bool,
    output: &Option<PathBuf>,
) {
    let catalog = load_catalog(&[]);

    let game_dirs = find_game_dirs(games_dir);
    if game_dirs.is_empty() {
        eprintln!(
            "error: no games found (no eboot.bin) in: {}",
            games_dir.display()
        );
        std::process::exit(1);
    }
    eprintln!(
        "Found {} game(s) in {}",
        game_dirs.len(),
        games_dir.display()
    );

    let remu_binary = remu.as_ref().map(|p| resolve_remu_binary(p));
    if let Some(binary) = &remu_binary {
        eprintln!("Using REmu binary: {}", binary.display());
    }

    let mut games: Vec<GameReport> = Vec::new();
    let mut agg: BTreeMap<String, Acc> = BTreeMap::new();
    let mut native_all: HashMap<String, HashSet<String>> = HashMap::new();
    let mut global_resolved: HashMap<String, String> = HashMap::new();
    let mut extraction_diffs: Vec<ExtractionDiff> = Vec::new();
    let mut imports_total = 0usize;
    let mut known_total = 0usize;

    for (idx, game_dir) in game_dirs.iter().enumerate() {
        let display = display_name(game_dir);
        eprint!("[{}/{}] {} ... ", idx + 1, game_dirs.len(), display);

        let Some(eboot) = select_eboot(game_dir) else {
            eprintln!("SKIPPED (no eboot.bin)");
            games.push(GameReport {
                name: display,
                file: String::new(),
                imports: 0,
                known: 0,
                unknown: 0,
                unknown_nids: Vec::new(),
                error: Some("no eboot.bin found".to_string()),
            });
            continue;
        };

        let data = match std::fs::read(&eboot) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("FAILED (read: {e})");
                games.push(GameReport {
                    name: display,
                    file: eboot.to_string_lossy().to_string(),
                    imports: 0,
                    known: 0,
                    unknown: 0,
                    unknown_nids: Vec::new(),
                    error: Some(format!("read failed: {e}")),
                });
                continue;
            }
        };

        let sha256 = ps5_format::sha256_hex(&data);
        let image = ps5_image::BinaryImageBuilder::build_from_file(&data, &sha256, &catalog);

        let known = image
            .imports
            .iter()
            .filter(|i| i.resolved_name.is_some())
            .count();
        let unknown = image.imports.len() - known;
        imports_total += image.imports.len();
        known_total += known;

        let native = native_all.entry(display.clone()).or_default();
        let mut unknown_nids = HashSet::new();
        for imp in &image.imports {
            native.insert(imp.nid_hash.clone());
            if imp.resolved_name.is_none() {
                unknown_nids.insert(imp.nid_hash.clone());
                let library = image.import_libs.get(&imp.library_id).map(|s| s.as_str());
                record_unknown(&mut agg, &imp.nid_hash, library, &display);
            }
        }
        let mut unknown_list: Vec<String> = unknown_nids.into_iter().collect();
        unknown_list.sort();

        let remu_inventory = remu_binary
            .as_deref()
            .and_then(|b| run_remu_imports(b, &eboot))
            .map(|out| parse_remu_output(&out));
        let mut remu_only = Vec::new();
        if let Some(inventory) = remu_inventory {
            for nid in &inventory.all {
                let in_native = native_all.get(&display).is_some_and(|s| s.contains(nid));
                if !in_native {
                    remu_only.push(nid.clone());
                }
                if let Some(name) = inventory.resolved.get(nid) {
                    global_resolved
                        .entry(nid.clone())
                        .or_insert_with(|| name.clone());
                }
            }
        }
        remu_only.sort();
        if !remu_only.is_empty() {
            extraction_diffs.push(ExtractionDiff {
                game: display.clone(),
                nids_in_remu_not_native: remu_only,
            });
        }

        eprintln!(
            "OK ({}/{} imports, {} unknown)",
            image.imports.len(),
            known,
            unknown
        );

        games.push(GameReport {
            name: display,
            file: eboot.to_string_lossy().to_string(),
            imports: image.imports.len(),
            known,
            unknown,
            unknown_nids: unknown_list,
            error: None,
        });
    }

    let games_scanned = game_dirs.len();
    let entries = build_entries(agg, games_scanned, &global_resolved);

    let mut lib_counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &entries {
        if entry.libraries.is_empty() {
            *lib_counts.entry("unknown".to_string()).or_default() += 1;
        } else {
            for library in &entry.libraries {
                *lib_counts.entry(library.clone()).or_default() += 1;
            }
        }
    }
    let library_breakdown: Vec<LibraryCount> = lib_counts
        .into_iter()
        .map(|(library, unknown_count)| LibraryCount {
            library,
            unknown_count,
        })
        .collect();

    let remu_comparison = remu_binary.as_ref().map(|_| {
        let mut remu_known_not_ours: Vec<RemuNameEntry> = entries
            .iter()
            .filter_map(|e| {
                e.remu_name.as_ref().map(|name| RemuNameEntry {
                    nid: e.nid.clone(),
                    remu_name: name.clone(),
                })
            })
            .collect();
        remu_known_not_ours.sort_by(|a, b| a.nid.cmp(&b.nid));
        let mut neither_known: Vec<String> = entries
            .iter()
            .filter(|e| e.remu_name.is_none())
            .map(|e| e.nid.clone())
            .collect();
        neither_known.sort();
        RemuComparison {
            enabled: true,
            remu_known_not_ours,
            neither_known,
        }
    });

    let games_parsed = games.iter().filter(|g| g.error.is_none()).count();
    let games_failed = games.len() - games_parsed;

    let report = UnknownNidsReport {
        schema_version: SCHEMA_VERSION,
        tool: "ps5rs",
        games_dir: games_dir.to_string_lossy().to_string(),
        generated_at: iso8601_now(),
        summary: Summary {
            games_found: games_scanned,
            games_parsed,
            games_failed,
            imports_total,
            known_total,
            unknown_total: imports_total - known_total,
            unique_unknown_nids: entries.len(),
        },
        games,
        unknown_nids: entries,
        library_breakdown,
        remu_comparison,
        extraction_diffs,
    };

    if json {
        write_to_output_or_stdout(output, &|w| {
            serde_json::to_writer_pretty(w, &report).map_err(std::io::Error::other)
        });
    } else {
        print_report(&report);
    }
}

fn print_report(report: &UnknownNidsReport) {
    println!("Unknown NIDs Report");
    println!("===================");
    println!();
    println!(
        "Games: {} found, {} parsed, {} failed",
        report.summary.games_found, report.summary.games_parsed, report.summary.games_failed
    );
    println!(
        "Imports: {} total ({} known, {} unknown)",
        report.summary.imports_total, report.summary.known_total, report.summary.unknown_total
    );
    println!(
        "Unique unknown NIDs: {}",
        report.summary.unique_unknown_nids
    );
    println!();

    println!("Unknown NIDs by library:");
    for entry in &report.library_breakdown {
        println!("  {:<28} {}", entry.library, entry.unknown_count);
    }
    println!();

    println!("Top unknown NIDs:");
    println!(
        "  {:<11} {:>5} {:>6} {:>9} {:<20} remu name",
        "nid", "freq", "games", "confid.", "library"
    );
    for entry in report.unknown_nids.iter().take(40) {
        let library = if entry.libraries.is_empty() {
            "unknown"
        } else {
            &entry.libraries[0]
        };
        let remu_name = entry.remu_name.as_deref().unwrap_or("-");
        println!(
            "  {:<11} {:>5} {:>6} {:>9} {:<20} {}",
            entry.nid,
            entry.frequency,
            entry.games.len(),
            entry.confidence.as_str(),
            library,
            remu_name
        );
    }
    println!();

    if let Some(comparison) = &report.remu_comparison {
        println!("REMu cross-check: enabled");
        println!(
            "  NIDs REmu can name but we lack: {}",
            comparison.remu_known_not_ours.len()
        );
        println!(
            "  NIDs neither side knows: {}",
            comparison.neither_known.len()
        );
        if !report.extraction_diffs.is_empty() {
            println!("  Extraction divergences:");
            for diff in &report.extraction_diffs {
                println!(
                    "    {}: {} NIDs remu-only",
                    diff.game,
                    diff.nids_in_remu_not_native.len()
                );
            }
        }
        println!();
    }

    let failed: Vec<&GameReport> = report.games.iter().filter(|g| g.error.is_some()).collect();
    if !failed.is_empty() {
        println!("Failed/skipped games:");
        for game in failed {
            let detail = game.error.as_deref().unwrap_or("error");
            println!("  {}: {detail}", game.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_remu_output_extracts_resolved_names() {
        let output = "\
path=x\\eboot.bin
dependencies=2
dependency=libkernel.prx
plt[0]=offset:0x10 symbol:1 nid:bzQExy189ZI resolved:_init_env name:bzQExy189ZI#3#1
plt[1]=offset:0x18 symbol:2 nid:XKRegsFpEpk resolved:unknown name:XKRegsFpEpk#3#1
import=Q3VBxCXhUHs symbol=3 weak=false relocations=1 types=7
import=8G2LB+A3rzg#2#1 symbol=4 weak=true relocations=2 types=1,7
";
        let inventory = parse_remu_output(output);
        assert_eq!(
            inventory.resolved.get("bzQExy189ZI").map(String::as_str),
            Some("_init_env")
        );
        assert!(!inventory.resolved.contains_key("XKRegsFpEpk"));
        assert_eq!(inventory.all.len(), 4);
        assert!(inventory.all.contains("bzQExy189ZI"));
        assert!(inventory.all.contains("XKRegsFpEpk"));
        assert!(inventory.all.contains("Q3VBxCXhUHs"));
        assert!(inventory.all.contains("8G2LB+A3rzg"));
    }

    #[test]
    fn parse_remu_output_ignores_malformed_lines() {
        let output = "\
plt[0]=broken no nid field
import=not-a-nid symbol=1 weak=false relocations=1 types=7
random noise
";
        let inventory = parse_remu_output(output);
        assert!(inventory.resolved.is_empty());
        assert!(inventory.all.is_empty());
    }

    #[test]
    fn select_eboot_prefers_shallow_base_over_decrypted() {
        let tmp = std::env::temp_dir().join("ps5rs-un-nid-test-1");
        std::fs::create_dir_all(tmp.join("decrypted")).unwrap();
        std::fs::write(tmp.join("eboot.bin"), b"x").unwrap();
        std::fs::write(tmp.join("decrypted").join("eboot.bin"), b"y").unwrap();
        let chosen = select_eboot(&tmp).unwrap();
        assert_eq!(chosen, tmp.join("eboot.bin"));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn select_eboot_skips_patch_union_variants() {
        let tmp = std::env::temp_dir().join("ps5rs-un-nid-test-2");
        std::fs::create_dir_all(tmp.join("base")).unwrap();
        std::fs::create_dir_all(tmp.join("base").join("app0-patch0-union")).unwrap();
        std::fs::write(
            tmp.join("base").join("app0-patch0-union").join("eboot.bin"),
            b"x",
        )
        .unwrap();
        std::fs::write(tmp.join("base").join("eboot.bin"), b"y").unwrap();
        let chosen = select_eboot(&tmp).unwrap();
        assert_eq!(chosen, tmp.join("base").join("eboot.bin"));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn select_eboot_returns_none_without_eboot() {
        let tmp = std::env::temp_dir().join("ps5rs-un-nid-test-3");
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(select_eboot(&tmp).is_none());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn confidence_heuristic_ranks_evidence() {
        assert_eq!(confidence(39, true, true, 39), Confidence::High);
        assert_eq!(confidence(1, false, false, 28), Confidence::Low);
        assert_eq!(confidence(2, true, false, 28), Confidence::Medium);
        assert_eq!(confidence(9, true, false, 28), Confidence::High);
        assert_eq!(confidence(1, false, true, 28), Confidence::Medium);
    }

    #[test]
    fn record_unknown_aggregates_across_games() {
        let mut agg: BTreeMap<String, Acc> = BTreeMap::new();
        record_unknown(&mut agg, "abc", Some("libkernel.prx"), "GameA");
        record_unknown(&mut agg, "abc", Some("libkernel.prx"), "GameB");
        record_unknown(&mut agg, "xyz", None, "GameA");
        assert_eq!(agg["abc"].frequency, 2);
        assert_eq!(agg["abc"].games.len(), 2);
        assert_eq!(agg["abc"].libraries.len(), 1);
        assert_eq!(agg["xyz"].frequency, 1);
        assert!(agg["xyz"].libraries.is_empty());
    }

    #[test]
    fn build_entries_sorts_by_frequency_and_attaches_names() {
        let mut agg: BTreeMap<String, Acc> = BTreeMap::new();
        record_unknown(&mut agg, "lowfreq", Some("libScePad.prx"), "GameA");
        record_unknown(&mut agg, "novidence", None, "GameA");
        for i in 0..10 {
            record_unknown(&mut agg, "highfreq", None, &format!("Game{i}"));
        }
        let mut resolved = HashMap::new();
        resolved.insert("highfreq".to_string(), "sceKernelSleep".to_string());
        let entries = build_entries(agg, 28, &resolved);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].nid, "highfreq");
        assert_eq!(entries[0].frequency, 10);
        assert_eq!(entries[0].remu_name.as_deref(), Some("sceKernelSleep"));
        assert_eq!(entries[0].confidence, Confidence::High);
        assert_eq!(entries[1].confidence, Confidence::Medium);
        assert_eq!(entries[2].confidence, Confidence::Low);
    }
}
