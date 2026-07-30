use ps5_analysis::dataset::AnalysisDataset;
use ps5_analysis::reports::build_engine_hints;
use ps5_image::{LibVersionEntry, SegmentType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub meta: DashboardMeta,
    pub overview: Overview,
    pub games: Vec<GameRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub game_details: Vec<GameDetail>,
    pub heatmap: HeatmapData,
    pub nid_stats: NidStats,
    pub segments: Vec<SegmentRow>,
    pub library_priority: Vec<LibraryPriority>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_details: Vec<LibraryDetail>,
    pub library_nid_breakdown: Vec<LibraryNidGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<DashboardStatistics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub engine_hints: Vec<DashboardEngineHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub engine_summary: Vec<EngineSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_versions: Vec<DashboardLibraryVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sce_library_stats: Vec<SceLibraryStats>,
    #[serde(default)]
    pub sce_heatmap: HeatmapData,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sce_library_versions: Vec<DashboardLibraryVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMeta {
    pub generated_at: String,
    pub game_count: usize,
    pub tool_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub total_games: usize,
    pub elf_valid: usize,
    pub total_imports: usize,
    pub unique_nids: usize,
    pub unique_libs: usize,
    pub resolution_rate: f64,
    pub avg_imports_per_game: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRow {
    pub name: String,
    pub title_name: Option<String>,
    pub platform: String,
    pub is_self: bool,
    pub engine: String,
    pub engine_confidence: u8,
    pub library_count: usize,
    pub sce_library_count: usize,
    pub unknown_nid_count: usize,
    pub file_size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetail {
    pub name: String,
    pub title_name: Option<String>,
    pub platform: String,
    pub is_self: bool,
    pub file_size_mb: f64,
    pub sha256: String,
    pub entry_point: String,
    pub elf_type: u16,
    pub osabi: u8,
    pub abi_version: u8,
    pub elf_version: u32,
    pub build_id: Option<String>,
    pub segments: Vec<SegmentDetail>,
    pub imports: Vec<ImportDetail>,
    pub unresolved_nids: Vec<UnresolvedNid>,
    pub import_summary: Vec<LibImportCount>,
    pub relocations: usize,
    pub has_tls: bool,
    pub engine: String,
    pub engine_score: u32,
    pub engine_confidence: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub engine_evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sce_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_libs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_forks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depot: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk_hints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib_versions: Vec<LibVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentDetail {
    pub index: usize,
    pub seg_type: String,
    pub vaddr: String,
    pub filesz: u64,
    pub memsz: u64,
    pub flags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDetail {
    pub nid_hash: String,
    pub resolved_name: Option<String>,
    pub library_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedNid {
    pub nid_hash: String,
    pub library_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibImportCount {
    pub library: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeatmapData {
    pub libraries: Vec<String>,
    pub games: Vec<String>,
    pub log_matrix: Vec<Vec<f64>>,
    pub raw_matrix: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NidStats {
    pub top_nids: Vec<TopNid>,
    pub resolved_count: usize,
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopNid {
    pub nid_hash: String,
    pub resolved_name: String,
    pub count: usize,
    pub game_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRow {
    pub game: String,
    pub rx_mb: f64,
    pub r_mb: f64,
    pub rw_mb: f64,
    pub other_mb: f64,
    pub total_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryPriority {
    pub name: String,
    pub game_count: usize,
    pub import_count: usize,
    pub unique_nid_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryNidGroup {
    pub library: String,
    pub game_count: usize,
    pub total_imports: usize,
    pub unique_nid_count: usize,
    pub top_nids: Vec<TopNid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDetail {
    pub name: String,
    pub game_count: usize,
    pub total_imports: usize,
    pub unique_nid_count: usize,
    pub games: Vec<LibGameEntry>,
    pub top_nids: Vec<TopNid>,
    pub unknown_nids: Vec<TopNid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibGameEntry {
    pub game: String,
    pub title_name: Option<String>,
    pub import_count: usize,
    pub unique_nid_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatistics {
    pub top_5_largest: Vec<StatEntry>,
    pub top_5_smallest: Vec<StatEntry>,
    pub top_5_most_imports: Vec<StatEntry>,
    pub top_5_most_libs: Vec<StatEntry>,
    pub top_5_highest_unknown: Vec<StatEntry>,
    pub avg_code_size_mb: f64,
    pub avg_data_size_mb: f64,
    pub avg_rodata_size_mb: f64,
    pub avg_other_size_mb: f64,
    pub total_code_mb: f64,
    pub total_data_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatEntry {
    pub game: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardEngineHint {
    pub name: String,
    pub display_name: String,
    pub engine: String,
    pub score: u32,
    pub confidence: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sce_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_libs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_forks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_depot: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sdk_hints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib_versions: Vec<LibVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSummary {
    pub engine: String,
    pub game_count: usize,
    pub avg_score: f64,
    pub avg_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SceLibraryCategory {
    Graphics,
    Audio,
    Input,
    Network,
    System,
    Storage,
    User,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceLibraryStats {
    pub library: String,
    pub category: SceLibraryCategory,
    pub game_count: usize,
    pub games: Vec<String>,
    pub game_ids: Vec<String>,
    pub module_count: usize,
    pub import_count: usize,
    pub versions: Vec<SceLibVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceLibVersionEntry {
    pub version_string: String,
    pub version_raw: u32,
    pub game_count: usize,
    pub games: Vec<String>,
    pub game_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLibraryVersion {
    pub library: String,
    pub version_raw: u32,
    pub version_string: String,
    pub game_count: usize,
    pub games: Vec<String>,
    pub game_ids: Vec<String>,
}

pub fn compute(ds: &AnalysisDataset) -> DashboardData {
    let meta = DashboardMeta {
        generated_at: now_iso8601(),
        game_count: ds.images.len(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let engine_hint_report = build_engine_hints(ds);
    let engine_hints: Vec<DashboardEngineHint> = engine_hint_report
        .games
        .iter()
        .map(|hint| {
            let img = ds.images.iter().find(|(n, _)| n == &hint.name);
            let sa = img.and_then(|(_, doc)| doc.string_analysis.as_ref());

            let (engine, score, confidence, evidence) =
                if let Some(engine_det) = sa.and_then(|sa| sa.engine.as_ref()) {
                    (
                        engine_det.value.clone(),
                        engine_det.score,
                        engine_det.confidence,
                        engine_det.evidence.clone(),
                    )
                } else if let Some(first) = hint.engines.first() {
                    (first.clone(), 0, 0, vec![])
                } else {
                    ("Unknown".to_string(), 0, 0, vec![])
                };

            DashboardEngineHint {
                name: hint.name.clone(),
                display_name: hint
                    .display_name
                    .clone()
                    .unwrap_or_else(|| hint.name.clone()),
                engine,
                score,
                confidence,
                evidence,
                sce_libraries: hint.sce_libraries.clone(),
                third_party_libs: hint
                    .third_party_libs
                    .iter()
                    .map(|d| d.value.clone())
                    .collect(),
                custom_forks: hint.custom_forks.iter().map(|d| d.value.clone()).collect(),
                build_system: hint.build_system.as_ref().map(|d| d.value.clone()),
                source_depot: hint.source_depot.as_ref().map(|d| d.value.clone()),
                sdk_hints: hint.sdk_hints.iter().map(|d| d.value.clone()).collect(),
                detected_versions: hint
                    .detected_versions
                    .iter()
                    .map(|d| d.value.clone())
                    .collect(),
                lib_versions: img
                    .map(|(_, doc)| doc.image.lib_versions.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    let engine_summary = compute_engine_summary(&engine_hints);

    let overview = compute_overview(ds);
    let games = compute_games(ds, &engine_hints);
    let game_details = compute_game_details(ds, &engine_hints);
    let heatmap = compute_heatmap(ds);
    let nid_stats = compute_nid_stats(ds);
    let segments = compute_segments(ds);
    let library_priority = compute_library_priority(ds);
    let library_details = compute_library_details(ds);
    let library_nid_breakdown = compute_library_nid_breakdown(ds);
    let statistics = compute_statistics(ds, &segments);

    let library_versions = compute_library_versions(ds);
    let sce_library_stats = compute_sce_stats(ds);
    let sce_heatmap = compute_sce_heatmap(ds);
    let sce_library_versions = compute_sce_library_versions(ds);

    DashboardData {
        meta,
        overview,
        games,
        game_details,
        heatmap,
        nid_stats,
        segments,
        library_priority,
        library_details,
        library_nid_breakdown,
        statistics: Some(statistics),
        engine_hints,
        engine_summary,
        library_versions,
        sce_library_stats,
        sce_heatmap,
        sce_library_versions,
    }
}

fn compute_overview(ds: &AnalysisDataset) -> Overview {
    let total_imports: usize = ds.images.iter().map(|(_, d)| d.image.imports.len()).sum();
    let resolved: usize = ds
        .images
        .iter()
        .flat_map(|(_, d)| d.image.imports.iter())
        .filter(|i| i.resolved_name.is_some())
        .count();
    let unique_nids: HashSet<&str> = ds
        .images
        .iter()
        .flat_map(|(_, d)| d.image.imports.iter().map(|i| i.nid_hash.as_str()))
        .collect();
    let unique_libs: HashSet<&str> = ds
        .images
        .iter()
        .flat_map(|(_, d)| d.image.imports.iter().map(|i| i.library_name.as_str()))
        .collect();
    let total_games = ds.images.len();
    let elf_valid = ds
        .images
        .iter()
        .filter(|(_, d)| !d.image.segments.is_empty())
        .count();

    Overview {
        total_games,
        elf_valid,
        total_imports,
        unique_nids: unique_nids.len(),
        unique_libs: unique_libs.len(),
        resolution_rate: if total_imports > 0 {
            resolved as f64 / total_imports as f64 * 100.0
        } else {
            0.0
        },
        avg_imports_per_game: if total_games > 0 {
            total_imports as f64 / total_games as f64
        } else {
            0.0
        },
    }
}

fn compute_games(ds: &AnalysisDataset, engine_hints: &[DashboardEngineHint]) -> Vec<GameRow> {
    let hint_map: HashMap<&str, &DashboardEngineHint> =
        engine_hints.iter().map(|h| (h.name.as_str(), h)).collect();

    ds.images
        .iter()
        .map(|(name, doc)| {
            let img = &doc.image;
            let hint = hint_map.get(name.as_str());
            let engine = hint.map(|h| h.engine.clone()).unwrap_or_default();
            let engine_confidence = hint.map(|h| h.confidence).unwrap_or(0);

            let library_count: HashSet<&str> = img
                .imports
                .iter()
                .map(|i| i.library_name.as_str())
                .collect();
            let unknown_nid_count = img
                .imports
                .iter()
                .filter(|i| i.resolved_name.is_none())
                .count();

            let title_name = Some(ds.display_name_for(name).to_string());

            GameRow {
                name: name.clone(),
                title_name,
                platform: img.platform.to_string(),
                is_self: img.is_self,
                engine,
                engine_confidence,
                library_count: library_count.len(),
                sce_library_count: hint.map(|h| h.sce_libraries.len()).unwrap_or(0),
                unknown_nid_count,
                file_size_mb: img.file_size as f64 / (1024.0 * 1024.0),
            }
        })
        .collect()
}

fn compute_game_details(
    ds: &AnalysisDataset,
    engine_hints: &[DashboardEngineHint],
) -> Vec<GameDetail> {
    let hint_map: HashMap<&str, &DashboardEngineHint> =
        engine_hints.iter().map(|h| (h.name.as_str(), h)).collect();

    ds.images
        .iter()
        .map(|(name, doc)| {
            let img = &doc.image;
            let hint = hint_map.get(name.as_str());

            let engine = hint.map(|h| h.engine.clone()).unwrap_or_default();
            let engine_score = hint.map(|h| h.score.min(100)).unwrap_or(0);
            let engine_confidence = hint.map(|h| h.confidence).unwrap_or(0);
            let engine_evidence = hint.map(|h| h.evidence.clone()).unwrap_or_default();
            let sce_libraries = hint.map(|h| h.sce_libraries.clone()).unwrap_or_default();
            let third_party_libs = hint.map(|h| h.third_party_libs.clone()).unwrap_or_default();
            let custom_forks = hint.map(|h| h.custom_forks.clone()).unwrap_or_default();
            let build_system = hint.and_then(|h| h.build_system.clone());
            let source_depot = hint.and_then(|h| h.source_depot.clone());
            let sdk_hints = hint.map(|h| h.sdk_hints.clone()).unwrap_or_default();
            let detected_versions = hint
                .map(|h| h.detected_versions.clone())
                .unwrap_or_default();

            let segments = img
                .segments
                .iter()
                .enumerate()
                .map(|(i, s)| SegmentDetail {
                    index: i,
                    seg_type: format!("{:?}", s.seg_type),
                    vaddr: format!("0x{:x}", s.vaddr),
                    filesz: s.filesz,
                    memsz: s.memsz,
                    flags: s.flags(),
                })
                .collect();

            let imports = img
                .imports
                .iter()
                .map(|imp| ImportDetail {
                    nid_hash: imp.nid_hash.clone(),
                    resolved_name: imp.resolved_name.clone(),
                    library_name: imp.library_name.clone(),
                })
                .collect();

            let unresolved_nids: Vec<UnresolvedNid> = img
                .imports
                .iter()
                .filter(|i| i.resolved_name.is_none())
                .map(|i| UnresolvedNid {
                    nid_hash: i.nid_hash.clone(),
                    library_name: i.library_name.clone(),
                })
                .collect();

            let mut lib_counts: HashMap<String, usize> = HashMap::new();
            for imp in &img.imports {
                *lib_counts.entry(imp.library_name.clone()).or_insert(0) += 1;
            }
            let mut import_summary: Vec<LibImportCount> = lib_counts
                .into_iter()
                .map(|(library, count)| LibImportCount { library, count })
                .collect();
            import_summary.sort_by_key(|b| std::cmp::Reverse(b.count));

            let meta = &img.metadata;
            GameDetail {
                name: name.clone(),
                title_name: Some(ds.display_name_for(name).to_string()),
                platform: img.platform.to_string(),
                is_self: img.is_self,
                file_size_mb: img.file_size as f64 / (1024.0 * 1024.0),
                sha256: img.sha256.clone(),
                entry_point: format!("0x{:x}", img.entry_point),
                elf_type: meta.elf_type,
                osabi: meta.osabi,
                abi_version: meta.ei_abi_version,
                elf_version: meta.e_version,
                build_id: meta.build_id.clone(),
                segments,
                imports,
                unresolved_nids,
                import_summary,
                relocations: img.relocations.len(),
                has_tls: img.tls.is_some(),
                engine,
                engine_score,
                engine_confidence,
                engine_evidence,
                sce_libraries,
                third_party_libs,
                custom_forks,
                build_system,
                source_depot,
                sdk_hints,
                detected_versions,
                lib_versions: doc.image.lib_versions.clone(),
            }
        })
        .collect()
}

fn compute_heatmap(ds: &AnalysisDataset) -> HeatmapData {
    let mut lib_game_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut all_games: Vec<String> = Vec::new();
    let mut seen_games: HashSet<String> = HashSet::new();

    for (name, doc) in &ds.images {
        if !seen_games.contains(name) {
            all_games.push(name.clone());
            seen_games.insert(name.clone());
        }
        for imp in &doc.image.imports {
            lib_game_counts
                .entry(imp.library_name.clone())
                .or_default()
                .entry(name.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
    }

    let mut lib_names: Vec<String> = lib_game_counts.keys().cloned().collect();
    lib_names.sort();

    let mut raw_matrix = Vec::with_capacity(lib_names.len());
    let mut log_matrix = Vec::with_capacity(lib_names.len());

    for lib in &lib_names {
        let raw_row: Vec<usize> = all_games
            .iter()
            .map(|game| lib_game_counts[lib].get(game).copied().unwrap_or(0))
            .collect();
        let log_row: Vec<f64> = raw_row.iter().map(|&v| ((v as f64) + 1.0).log2()).collect();
        raw_matrix.push(raw_row);
        log_matrix.push(log_row);
    }

    HeatmapData {
        libraries: lib_names,
        games: all_games,
        log_matrix,
        raw_matrix,
    }
}

fn compute_nid_stats(ds: &AnalysisDataset) -> NidStats {
    let mut nid_counts: HashMap<String, (String, usize)> = HashMap::new();
    let mut resolved_total = 0usize;
    let mut unknown_total = 0usize;

    for (_, doc) in &ds.images {
        for imp in &doc.image.imports {
            if imp.resolved_name.is_some() {
                resolved_total += 1;
            } else {
                unknown_total += 1;
            }
            let entry = nid_counts
                .entry(imp.nid_hash.clone())
                .or_insert_with(|| (imp.resolved_name.clone().unwrap_or_default(), 0));
            entry.1 += 1;
        }
    }

    let mut top_nids: Vec<TopNid> = nid_counts
        .into_iter()
        .map(|(hash, (name, count))| TopNid {
            nid_hash: hash,
            resolved_name: name,
            count,
            game_count: 0,
        })
        .collect();

    top_nids.sort_by_key(|b| std::cmp::Reverse(b.count));
    top_nids.truncate(25);

    NidStats {
        top_nids,
        resolved_count: resolved_total,
        unknown_count: unknown_total,
    }
}

fn compute_segments(ds: &AnalysisDataset) -> Vec<SegmentRow> {
    ds.images
        .iter()
        .map(|(name, doc)| {
            let (rx, r, rw, other) = sum_segment_sizes(&doc.image);
            let total = rx + r + rw + other;
            SegmentRow {
                game: name.clone(),
                rx_mb: rx as f64 / (1024.0 * 1024.0),
                r_mb: r as f64 / (1024.0 * 1024.0),
                rw_mb: rw as f64 / (1024.0 * 1024.0),
                other_mb: other as f64 / (1024.0 * 1024.0),
                total_mb: total as f64 / (1024.0 * 1024.0),
            }
        })
        .collect()
}

fn compute_library_priority(ds: &AnalysisDataset) -> Vec<LibraryPriority> {
    let mut lib_data: HashMap<String, (usize, usize, HashSet<String>)> = HashMap::new();

    for (_, doc) in &ds.images {
        let mut seen_libs: HashSet<&str> = HashSet::new();
        for imp in &doc.image.imports {
            let e = lib_data
                .entry(imp.library_name.clone())
                .or_insert_with(|| (0, 0, HashSet::new()));
            e.1 += 1;
            e.2.insert(imp.nid_hash.clone());
            seen_libs.insert(&imp.library_name);
        }
        for lib in &seen_libs {
            lib_data
                .entry(lib.to_string())
                .or_insert_with(|| (0, 0, HashSet::new()))
                .0 += 1;
        }
    }

    let mut result: Vec<LibraryPriority> = lib_data
        .into_iter()
        .map(|(name, (gc, ic, nids))| LibraryPriority {
            name,
            game_count: gc,
            import_count: ic,
            unique_nid_count: nids.len(),
        })
        .collect();

    result.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(b.import_count.cmp(&a.import_count))
    });
    result
}

fn compute_library_details(ds: &AnalysisDataset) -> Vec<LibraryDetail> {
    let mut lib_games: HashMap<String, HashMap<String, (usize, HashSet<String>)>> = HashMap::new();

    for (game, doc) in &ds.images {
        for imp in &doc.image.imports {
            let game_entry = lib_games
                .entry(imp.library_name.clone())
                .or_default()
                .entry(game.clone())
                .or_insert_with(|| (0, HashSet::new()));
            game_entry.0 += 1;
            game_entry.1.insert(imp.nid_hash.clone());
        }
    }

    let mut result: Vec<LibraryDetail> = Vec::new();

    for (lib, games_map) in &lib_games {
        let total_imports: usize = games_map.values().map(|(c, _)| c).sum();
        let all_nids: HashSet<&str> = games_map
            .values()
            .flat_map(|(_, nids)| nids.iter().map(|s| s.as_str()))
            .collect();
        let unique_nid_count = all_nids.len();

        let mut games: Vec<LibGameEntry> = games_map
            .iter()
            .map(|(game, (count, nids))| {
                let title = Some(ds.display_name_for(game).to_string());
                LibGameEntry {
                    game: game.clone(),
                    title_name: title,
                    import_count: *count,
                    unique_nid_count: nids.len(),
                }
            })
            .collect();
        games.sort_by_key(|b| std::cmp::Reverse(b.import_count));

        let mut nid_counts: HashMap<String, (String, usize)> = HashMap::new();
        let mut unknown_counts: HashMap<String, (String, usize)> = HashMap::new();
        for (_, doc) in &ds.images {
            for imp in &doc.image.imports {
                if imp.library_name != *lib {
                    continue;
                }
                if imp.resolved_name.is_some() {
                    let e = nid_counts
                        .entry(imp.nid_hash.clone())
                        .or_insert_with(|| (imp.resolved_name.clone().unwrap_or_default(), 0));
                    e.1 += 1;
                } else {
                    let e = unknown_counts
                        .entry(imp.nid_hash.clone())
                        .or_insert_with(|| (String::new(), 0));
                    e.1 += 1;
                }
            }
        }

        let mut top_nids: Vec<TopNid> = nid_counts
            .into_iter()
            .map(|(hash, (name, count))| TopNid {
                nid_hash: hash,
                resolved_name: name,
                count,
                game_count: 0,
            })
            .collect();
        top_nids.sort_by_key(|n| std::cmp::Reverse(n.count));
        top_nids.truncate(15);

        let mut unknown_nids: Vec<TopNid> = unknown_counts
            .into_iter()
            .map(|(hash, (_, count))| TopNid {
                nid_hash: hash,
                resolved_name: String::new(),
                count,
                game_count: 0,
            })
            .collect();
        unknown_nids.sort_by_key(|n| std::cmp::Reverse(n.count));
        unknown_nids.truncate(15);

        result.push(LibraryDetail {
            name: lib.clone(),
            game_count: games.len(),
            total_imports,
            unique_nid_count,
            games,
            top_nids,
            unknown_nids,
        });
    }

    result.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(b.total_imports.cmp(&a.total_imports))
    });
    result
}

fn compute_library_nid_breakdown(ds: &AnalysisDataset) -> Vec<LibraryNidGroup> {
    let mut lib_games: HashMap<String, HashSet<String>> = HashMap::new();
    let mut lib_nids: HashMap<String, HashMap<String, (String, usize)>> = HashMap::new();

    for (game, doc) in &ds.images {
        for imp in &doc.image.imports {
            lib_games
                .entry(imp.library_name.clone())
                .or_default()
                .insert(game.clone());
            let nid_entry = lib_nids
                .entry(imp.library_name.clone())
                .or_default()
                .entry(imp.nid_hash.clone())
                .or_insert_with(|| (imp.resolved_name.clone().unwrap_or_default(), 0));
            nid_entry.1 += 1;
        }
    }

    let mut groups: Vec<LibraryNidGroup> = lib_games
        .into_iter()
        .map(|(lib, games)| {
            let total_imports: usize = lib_nids
                .get(&lib)
                .map(|nids| nids.values().map(|(_, c)| c).sum())
                .unwrap_or(0);
            let unique_nid_count = lib_nids.get(&lib).map_or(0, |nids| nids.len());
            let mut top_nids: Vec<TopNid> = lib_nids
                .get(&lib)
                .map(|nids| {
                    nids.iter()
                        .map(|(hash, (name, count))| TopNid {
                            nid_hash: hash.clone(),
                            resolved_name: name.clone(),
                            count: *count,
                            game_count: 0,
                        })
                        .collect()
                })
                .unwrap_or_default();
            top_nids.sort_by_key(|n| std::cmp::Reverse(n.count));
            top_nids.truncate(10);

            LibraryNidGroup {
                library: lib,
                game_count: games.len(),
                total_imports,
                unique_nid_count,
                top_nids,
            }
        })
        .collect();

    groups.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(b.total_imports.cmp(&a.total_imports))
    });
    groups
}

fn compute_statistics(ds: &AnalysisDataset, segments: &[SegmentRow]) -> DashboardStatistics {
    if segments.is_empty() {
        return DashboardStatistics {
            top_5_largest: vec![],
            top_5_smallest: vec![],
            top_5_most_imports: vec![],
            top_5_most_libs: vec![],
            top_5_highest_unknown: vec![],
            avg_code_size_mb: 0.0,
            avg_data_size_mb: 0.0,
            avg_rodata_size_mb: 0.0,
            avg_other_size_mb: 0.0,
            total_code_mb: 0.0,
            total_data_mb: 0.0,
        };
    }

    let mut by_size: Vec<&SegmentRow> = segments.iter().collect();
    by_size.sort_by(|a, b| {
        b.total_mb
            .partial_cmp(&a.total_mb)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_5_largest: Vec<StatEntry> = by_size
        .iter()
        .take(5)
        .map(|s| StatEntry {
            game: s.game.clone(),
            value: s.total_mb,
        })
        .collect();
    let top_5_smallest: Vec<StatEntry> = by_size
        .iter()
        .rev()
        .take(5)
        .map(|s| StatEntry {
            game: s.game.clone(),
            value: s.total_mb,
        })
        .collect();

    let mut by_imports: Vec<(&String, &ps5_image::BinaryImageDocument)> =
        ds.images.iter().map(|(n, d)| (n, d)).collect();
    by_imports.sort_by_key(|b| std::cmp::Reverse(b.1.image.imports.len()));

    let top_5_most_imports: Vec<StatEntry> = by_imports
        .iter()
        .take(5)
        .map(|(name, doc)| StatEntry {
            game: name.to_string(),
            value: doc.image.imports.len() as f64,
        })
        .collect();

    let mut lib_counts_per_game: Vec<(String, usize)> = ds
        .images
        .iter()
        .map(|(name, doc)| {
            let libs: HashSet<&str> = doc
                .image
                .imports
                .iter()
                .map(|i| i.library_name.as_str())
                .collect();
            (name.clone(), libs.len())
        })
        .collect();
    lib_counts_per_game.sort_by_key(|b| std::cmp::Reverse(b.1));

    let top_5_most_libs: Vec<StatEntry> = lib_counts_per_game
        .iter()
        .take(5)
        .map(|(name, count)| StatEntry {
            game: name.clone(),
            value: *count as f64,
        })
        .collect();

    let mut unknown_pct: Vec<(String, f64)> = ds
        .images
        .iter()
        .filter(|(_, doc)| !doc.image.imports.is_empty())
        .map(|(name, doc)| {
            let total = doc.image.imports.len();
            let unknown = doc
                .image
                .imports
                .iter()
                .filter(|i| i.resolved_name.is_none())
                .count();
            (name.clone(), unknown as f64 / total as f64 * 100.0)
        })
        .collect();
    unknown_pct.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_5_highest_unknown: Vec<StatEntry> = unknown_pct
        .iter()
        .take(5)
        .map(|(name, pct)| StatEntry {
            game: name.clone(),
            value: *pct,
        })
        .collect();

    let n = segments.len() as f64;
    let avg_code = segments.iter().map(|s| s.rx_mb).sum::<f64>() / n;
    let avg_data = segments.iter().map(|s| s.rw_mb).sum::<f64>() / n;
    let avg_rodata = segments.iter().map(|s| s.r_mb).sum::<f64>() / n;
    let avg_other = segments.iter().map(|s| s.other_mb).sum::<f64>() / n;
    let total_code = segments.iter().map(|s| s.rx_mb).sum::<f64>();
    let total_data = segments.iter().map(|s| s.rw_mb).sum::<f64>();

    DashboardStatistics {
        top_5_largest,
        top_5_smallest,
        top_5_most_imports,
        top_5_most_libs,
        top_5_highest_unknown,
        avg_code_size_mb: avg_code,
        avg_data_size_mb: avg_data,
        avg_rodata_size_mb: avg_rodata,
        avg_other_size_mb: avg_other,
        total_code_mb: total_code,
        total_data_mb: total_data,
    }
}

fn compute_engine_summary(hints: &[DashboardEngineHint]) -> Vec<EngineSummary> {
    let mut by_engine: HashMap<String, (usize, u64, u64)> = HashMap::new();
    for h in hints {
        let e = by_engine.entry(h.engine.clone()).or_insert((0, 0, 0));
        e.0 += 1;
        e.1 += h.score as u64;
        e.2 += h.confidence as u64;
    }
    let mut result: Vec<EngineSummary> = by_engine
        .into_iter()
        .map(|(engine, (count, total_score, total_conf))| EngineSummary {
            engine,
            game_count: count,
            avg_score: if count > 0 {
                total_score as f64 / count as f64
            } else {
                0.0
            },
            avg_confidence: if count > 0 {
                total_conf as f64 / count as f64
            } else {
                0.0
            },
        })
        .collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.game_count));
    result
}

fn compute_library_versions(ds: &AnalysisDataset) -> Vec<DashboardLibraryVersion> {
    let mut map: HashMap<(String, u32), DashboardLibraryVersion> = HashMap::new();
    for (name, doc) in &ds.images {
        let display = ds.display_name_for(name).to_string();
        for lv in &doc.image.lib_versions {
            let key = (lv.name.clone(), lv.version_raw);
            let entry = map.entry(key).or_insert_with(|| DashboardLibraryVersion {
                library: lv.name.clone(),
                version_raw: lv.version_raw,
                version_string: lv.version_string.clone(),
                game_count: 0,
                games: Vec::new(),
                game_ids: Vec::new(),
            });
            if !entry.games.contains(&display) {
                entry.games.push(display.clone());
                entry.game_ids.push(name.clone());
                entry.game_count = entry.games.len();
            }
        }
    }
    let mut entries: Vec<DashboardLibraryVersion> = map.into_values().collect();
    entries.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(a.library.cmp(&b.library))
    });
    entries
}

fn sum_segment_sizes(img: &ps5_image::BinaryImage) -> (u64, u64, u64, u64) {
    let mut rx = 0u64;
    let mut r = 0u64;
    let mut rw = 0u64;
    let mut other = 0u64;

    for seg in &img.segments {
        if seg.seg_type != SegmentType::Load {
            other += seg.filesz;
            continue;
        }
        if seg.is_executable {
            rx += seg.filesz;
        } else if seg.is_writable {
            rw += seg.filesz;
        } else {
            r += seg.filesz;
        }
    }

    (rx, r, rw, other)
}

fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

fn collect_sce_libraries(doc: &ps5_image::BinaryImageDocument) -> BTreeSet<String> {
    let mut libs = BTreeSet::new();
    for imp in &doc.image.imports {
        if imp.library_name.starts_with("libSce") {
            libs.insert(imp.library_name.clone());
        }
    }
    for lib in doc.image.import_libs.values() {
        if lib.starts_with("libSce") {
            libs.insert(lib.clone());
        }
    }
    if let Some(sa) = &doc.string_analysis {
        for lib in &sa.sce_libraries {
            if lib.starts_with("libSce") {
                libs.insert(lib.clone());
            }
        }
    }
    libs
}

fn categorize_sce_library(name: &str) -> SceLibraryCategory {
    if name.contains("Gnm")
        || name.contains("VideoOut")
        || name.contains("Gpu")
        || name.contains("Display")
        || name.contains("Gnmf")
    {
        SceLibraryCategory::Graphics
    } else if name.contains("Audio") || name.contains("Sound") {
        SceLibraryCategory::Audio
    } else if name.contains("Pad")
        || name.contains("Mouse")
        || name.contains("Keyboard")
        || name.contains("Touch")
        || name.contains("Move")
        || name.contains("Trigger")
    {
        SceLibraryCategory::Input
    } else if name.contains("Net")
        || name.contains("Http")
        || name.contains("Ssl")
    {
        SceLibraryCategory::Network
    } else if name.contains("SaveData")
        || name.contains("Storage")
        || name.contains("Disc")
        || name.contains("Ngs2")
    {
        SceLibraryCategory::Storage
    } else if name.contains("Np")
        || name.contains("User")
        || name.contains("NpMatching")
    {
        SceLibraryCategory::User
    } else if name.contains("System")
        || name.contains("AppContent")
        || name.contains("Kernel")
        || name.contains("Thread")
    {
        SceLibraryCategory::System
    } else {
        SceLibraryCategory::Unknown
    }
}

fn compute_sce_stats(ds: &AnalysisDataset) -> Vec<SceLibraryStats> {
    let mut lib_data: HashMap<
        String,
        (Vec<String>, Vec<String>, usize, HashMap<String, SceLibVersionEntry>),
    > = HashMap::new();

    for (name, doc) in &ds.images {
        let sce_libs = collect_sce_libraries(doc);
        let display = ds.display_name_for(name).to_string();

        for lib in &sce_libs {
            let (game_ids, game_displays, import_count, versions) =
                lib_data.entry(lib.clone()).or_default();

            if !game_ids.contains(name) {
                game_ids.push(name.clone());
                game_displays.push(display.clone());
            }

            let count = doc
                .image
                .imports
                .iter()
                .filter(|i| i.library_name == *lib)
                .count();
            *import_count += count;

            for lv in &doc.image.lib_versions {
                if lv.name == *lib {
                    let v_entry =
                        versions.entry(lv.version_string.clone()).or_insert_with(|| {
                            SceLibVersionEntry {
                                version_string: lv.version_string.clone(),
                                version_raw: lv.version_raw,
                                game_count: 0,
                                games: Vec::new(),
                                game_ids: Vec::new(),
                            }
                        });
                    if !v_entry.game_ids.contains(name) {
                        v_entry.game_ids.push(name.clone());
                        v_entry.games.push(display.clone());
                        v_entry.game_count = v_entry.game_ids.len();
                    }
                }
            }
        }
    }

    let mut result: Vec<SceLibraryStats> = lib_data
        .into_iter()
        .map(|(lib, (game_ids, game_displays, import_count, versions))| {
            let category = categorize_sce_library(&lib);
            let mut version_list: Vec<SceLibVersionEntry> = versions.into_values().collect();
            version_list.sort_by_key(|v| std::cmp::Reverse(v.version_raw));

            SceLibraryStats {
                library: lib,
                category,
                game_count: game_ids.len(),
                games: game_displays,
                game_ids,
                module_count: 0,
                import_count,
                versions: version_list,
            }
        })
        .collect();

    result.sort_by(|a, b| {
        b.game_count
            .cmp(&a.game_count)
            .then(b.import_count.cmp(&a.import_count))
    });
    result
}

fn compute_sce_heatmap(ds: &AnalysisDataset) -> HeatmapData {
    let mut lib_game_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut all_games: Vec<String> = Vec::new();
    let mut seen_games: HashSet<String> = HashSet::new();

    for (name, doc) in &ds.images {
        let sce_libs = collect_sce_libraries(doc);
        if !seen_games.contains(name) {
            all_games.push(name.clone());
            seen_games.insert(name.clone());
        }
        for lib in &sce_libs {
            lib_game_counts
                .entry(lib.clone())
                .or_default()
                .entry(name.clone())
                .or_insert(1);
        }
    }

    let mut lib_names: Vec<String> = lib_game_counts.keys().cloned().collect();
    lib_names.sort();

    let mut raw_matrix = Vec::with_capacity(lib_names.len());
    let mut log_matrix = Vec::with_capacity(lib_names.len());

    for lib in &lib_names {
        let raw_row: Vec<usize> = all_games
            .iter()
            .map(|game| lib_game_counts[lib].get(game).copied().unwrap_or(0))
            .collect();
        let log_row: Vec<f64> = raw_row.iter().map(|&v| ((v as f64) + 1.0).log2()).collect();
        raw_matrix.push(raw_row);
        log_matrix.push(log_row);
    }

    HeatmapData {
        libraries: lib_names,
        games: all_games,
        log_matrix,
        raw_matrix,
    }
}

fn compute_sce_library_versions(ds: &AnalysisDataset) -> Vec<DashboardLibraryVersion> {
    let all = compute_library_versions(ds);
    all.into_iter()
        .filter(|v| v.library.starts_with("libSce"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps5_image::{
        BinaryImage, BinaryImageDocument, ImportEntry, LoadedSegment, Platform, SegmentType,
        SymbolBinding, SymbolType, SymbolVisibility,
    };

    fn make_doc(
        sha: &str,
        imports: Vec<ImportEntry>,
        segments: Vec<LoadedSegment>,
    ) -> BinaryImageDocument {
        use ps5_image::BinaryMetadata;
        BinaryImageDocument {
            schema_version: 1,
            tool: "test".to_string(),
            image_type: ps5_image::ImageType::Eboot,
            parent_image: None,
            string_analysis: None,
            image: BinaryImage {
                sha256: sha.to_string(),
                platform: Platform::Ps5,
                is_self: true,
                file_size: 1024 * 1024,
                entry_point: 0x80000000,
                metadata: BinaryMetadata {
                    build_id: None,
                    elf_type: 3,
                    elf_flags: 0,
                    osabi: 0x9,
                    ei_abi_version: 2,
                    e_version: 1,
                    self_key_type: None,
                    self_attr: None,
                    self_mode: None,
                    self_endian: None,
                    self_version: None,
                    self_flags: None,
                    sections: vec![],
                },
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
                import_libs: std::collections::HashMap::new(),
                needed_files: vec![],
                dynamic_entries: vec![],
                version_defs: vec![],
                lib_versions: vec![],
            },
        }
    }

    fn make_imp(nid: &str, resolved: Option<&str>, lib: &str) -> ImportEntry {
        ImportEntry {
            nid_hash: nid.to_string(),
            resolved_name: resolved.map(|s| s.to_string()),
            library_id: 1,
            library_name: lib.to_string(),
            value: 0,
            size: 0,
            shndx: 0,
            binding: SymbolBinding::Global,
            sym_type: SymbolType::Func,
            visibility: SymbolVisibility::Default,
            ordinal: 0,
        }
    }

    fn make_seg(flags: &str, filesz: u64) -> LoadedSegment {
        let (exec, write) = match flags {
            "RX" => (true, false),
            "R" => (false, false),
            "RW" => (false, true),
            "RWX" => (true, true),
            _ => (false, false),
        };
        LoadedSegment {
            vaddr: 0,
            file_offset: 0,
            filesz,
            memsz: filesz,
            is_executable: exec,
            is_writable: write,
            seg_type: SegmentType::Load,
            p_paddr: 0,
            p_align: 0x1000,
            is_encrypted: false,
            is_compressed: false,
            phdr_index: None,
        }
    }

    fn make_dataset(docs: Vec<(&str, BinaryImageDocument)>) -> AnalysisDataset {
        use ps5_analysis::dataset::{DATASET_SCHEMA_VERSION, Manifest};
        let mut images = Vec::new();
        for (name, doc) in docs {
            images.push((name.to_string(), doc));
        }
        images.sort_by_key(|(n, _)| n.clone());
        AnalysisDataset {
            manifest: Manifest {
                schema_version: DATASET_SCHEMA_VERSION,
                tool: "test".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                image_count: images.len(),
                module_count: 0,
                games: vec![],
            },
            images,
            display_names: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn compute_overview_basic() {
        let ds = make_dataset(vec![
            (
                "game1",
                make_doc(
                    &"a".repeat(64),
                    vec![make_imp("n1", Some("f1"), "libA")],
                    vec![],
                ),
            ),
            (
                "game2",
                make_doc(
                    &"b".repeat(64),
                    vec![
                        make_imp("n1", Some("f1"), "libA"),
                        make_imp("n2", None, "libA"),
                    ],
                    vec![],
                ),
            ),
        ]);
        let ov = compute_overview(&ds);
        assert_eq!(ov.total_games, 2);
        assert_eq!(ov.total_imports, 3);
        assert_eq!(ov.unique_nids, 2);
        assert_eq!(ov.unique_libs, 1);
        assert!((ov.resolution_rate - 66.66).abs() < 0.1);
    }

    #[test]
    fn compute_segments_load_only() {
        let ds = make_dataset(vec![(
            "game1",
            make_doc(
                &"a".repeat(64),
                vec![],
                vec![
                    make_seg("RX", 10 * 1024 * 1024),
                    make_seg("R", 3 * 1024 * 1024),
                    make_seg("RW", 2 * 1024 * 1024),
                ],
            ),
        )]);
        let segs = compute_segments(&ds);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].rx_mb - 10.0).abs() < 0.01);
        assert!((segs[0].r_mb - 3.0).abs() < 0.01);
        assert!((segs[0].rw_mb - 2.0).abs() < 0.01);
    }

    #[test]
    fn heatmap_log_scaling() {
        let ds = make_dataset(vec![(
            "g1",
            make_doc(
                &"a".repeat(64),
                vec![make_imp("n", Some("f"), "lib")],
                vec![],
            ),
        )]);
        let hm = compute_heatmap(&ds);
        assert_eq!(hm.libraries, vec!["lib"]);
        assert_eq!(hm.raw_matrix, vec![vec![1]]);
        let expected_log = ((1.0_f64) + 1.0).log2();
        assert!((hm.log_matrix[0][0] - expected_log).abs() < 0.001);
    }

    #[test]
    fn library_priority_sorted() {
        let ds = make_dataset(vec![
            (
                "g1",
                make_doc(
                    &"a".repeat(64),
                    vec![
                        make_imp("n1", Some("f"), "libA"),
                        make_imp("n2", Some("f"), "libA"),
                    ],
                    vec![],
                ),
            ),
            (
                "g2",
                make_doc(
                    &"b".repeat(64),
                    vec![
                        make_imp("n1", Some("f"), "libA"),
                        make_imp("n3", Some("f"), "libB"),
                    ],
                    vec![],
                ),
            ),
        ]);
        let lp = compute_library_priority(&ds);
        assert_eq!(lp[0].name, "libA");
        assert_eq!(lp[0].game_count, 2);
        assert_eq!(lp[0].import_count, 3);
        assert_eq!(lp[0].unique_nid_count, 2);
    }

    #[test]
    fn game_details_have_segments_and_imports() {
        let ds = make_dataset(vec![(
            "game1",
            make_doc(
                &"a".repeat(64),
                vec![
                    make_imp("n1", Some("f1"), "libA"),
                    make_imp("n2", None, "libA"),
                ],
                vec![make_seg("RX", 1024 * 1024), make_seg("RW", 512 * 1024)],
            ),
        )]);
        let details = compute_game_details(&ds, &[]);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].segments.len(), 2);
        assert_eq!(details[0].imports.len(), 2);
        assert_eq!(details[0].unresolved_nids.len(), 1);
        assert_eq!(details[0].import_summary.len(), 1);
        assert_eq!(details[0].import_summary[0].count, 2);
        assert_eq!(details[0].engine, "");
        assert_eq!(details[0].engine_confidence, 0);
    }

    #[test]
    fn library_details_group_by_game() {
        let ds = make_dataset(vec![
            (
                "g1",
                make_doc(
                    &"a".repeat(64),
                    vec![
                        make_imp("n1", Some("f1"), "libA"),
                        make_imp("n2", Some("f2"), "libA"),
                    ],
                    vec![],
                ),
            ),
            (
                "g2",
                make_doc(
                    &"b".repeat(64),
                    vec![
                        make_imp("n1", Some("f1"), "libA"),
                        make_imp("n3", Some("f3"), "libB"),
                    ],
                    vec![],
                ),
            ),
        ]);
        let details = compute_library_details(&ds);
        let lib_a = details.iter().find(|d| d.name == "libA").unwrap();
        assert_eq!(lib_a.game_count, 2);
        assert_eq!(lib_a.total_imports, 3);
        assert_eq!(lib_a.games.len(), 2);
    }

    #[test]
    fn statistics_computed() {
        let ds = make_dataset(vec![
            (
                "game1",
                make_doc(
                    &"a".repeat(64),
                    vec![make_imp("n1", Some("f1"), "libA")],
                    vec![make_seg("RX", 10 * 1024 * 1024)],
                ),
            ),
            (
                "game2",
                make_doc(
                    &"b".repeat(64),
                    vec![make_imp("n2", None, "libA")],
                    vec![make_seg("RX", 1024)],
                ),
            ),
        ]);
        let segments = compute_segments(&ds);
        let stats = compute_statistics(&ds, &segments);
        assert_eq!(stats.top_5_largest.len(), 2);
        assert_eq!(stats.top_5_most_imports.len(), 2);
        assert!(stats.avg_code_size_mb > 0.0);
    }

    use ps5_analysis::param_json::GameParam;

    #[test]
    fn display_name_for_resolves_from_manifest() {
        let mut ds = make_dataset(vec![(
            "Bugsnax-PPSA01502-USA-PS5",
            make_doc(&"a".repeat(64), vec![], vec![]),
        )]);
        ds.manifest.games = vec![GameParam {
            title_id: Some("PPSA01502".to_string()),
            title_name: Some("Bugsnax".to_string()),
            display_name: Some("Bugsnax - [PPSA01502]".to_string()),
            ..Default::default()
        }];
        ds.display_names.insert(
            "Bugsnax-PPSA01502-USA-PS5".to_string(),
            "Bugsnax - [PPSA01502]".to_string(),
        );
        assert_eq!(
            ds.display_name_for("Bugsnax-PPSA01502-USA-PS5"),
            "Bugsnax - [PPSA01502]"
        );
    }

    #[test]
    fn display_name_for_falls_back_to_name() {
        let ds = make_dataset(vec![(
            "SomeGame-ABC",
            make_doc(&"a".repeat(64), vec![], vec![]),
        )]);
        assert_eq!(ds.display_name_for("SomeGame-ABC"), "SomeGame-ABC");
    }

    #[test]
    fn display_name_for_case_insensitive() {
        let mut ds = make_dataset(vec![(
            "trek-to-yomi-ppsa02629",
            make_doc(&"a".repeat(64), vec![], vec![]),
        )]);
        ds.display_names.insert(
            "trek-to-yomi-ppsa02629".to_string(),
            "Trek To Yomi - [PPSA02629]".to_string(),
        );
        assert_eq!(
            ds.display_name_for("trek-to-yomi-ppsa02629"),
            "Trek To Yomi - [PPSA02629]"
        );
    }
}
