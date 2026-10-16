use std::collections::HashSet;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::catalog::load_catalog;

const NID_LEN: usize = 11;
const MATCH_LEN: usize = 15;
const BACKUP_SUFFIXES: &[&str] = &[".esbak", ".bak", ".orig"];
const MAX_WALK_DEPTH: usize = 32;
const PRIMARY_CSV_NAME: &str = "fprx-nids.csv";

const FMOD_CLASS_PREFIXES: &[&str] = &[
    "FMOD::",
    "EventDescription::",
    "EventInstance::",
    "Studio::",
];
const WRAPPER_PREFIX: &str = "fmodstudio_real_";

#[derive(Debug, Error)]
pub(crate) enum ScanError {
    #[error("not a directory: {}", .0.display())]
    RootNotDirectory(PathBuf),
    #[error("no matching module files found under {}", .0.display())]
    NoModules(PathBuf),
    #[error("invalid --min-string-length {0}: must be at least 1")]
    BadMinLength(usize),
    #[error("cannot write report to {}: {source}", .path.display())]
    ReportWrite { path: PathBuf, source: csv::Error },
    #[error("cannot create report file {}: {source}", .path.display())]
    ReportCreate {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StringCategory {
    CppMethod,
    SceKernelApi,
    SceApi,
    FmodApi,
    CompatWrapper,
    Other,
}

impl StringCategory {
    fn as_str(self) -> &'static str {
        match self {
            StringCategory::CppMethod => "cpp_method",
            StringCategory::SceKernelApi => "sce_kernel_api_candidate",
            StringCategory::SceApi => "sce_api_candidate",
            StringCategory::FmodApi => "fmod_api_candidate",
            StringCategory::CompatWrapper => "compat_wrapper_candidate",
            StringCategory::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchType {
    NearbyCandidate,
    SameFileCandidate,
    Unmatched,
    KnownMapping,
}

impl MatchType {
    fn as_str(self) -> &'static str {
        match self {
            MatchType::NearbyCandidate => "nearby_candidate",
            MatchType::SameFileCandidate => "same_file_candidate",
            MatchType::Unmatched => "unmatched",
            MatchType::KnownMapping => "known_mapping",
        }
    }

    fn confidence(self) -> &'static str {
        match self {
            MatchType::NearbyCandidate => "low",
            MatchType::SameFileCandidate => "none",
            MatchType::Unmatched => "",
            MatchType::KnownMapping => "high",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NidRecord {
    pub nid: String,
    pub suffix: String,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct StringRecord {
    pub text: String,
    pub offset: u64,
    pub category: StringCategory,
}

#[derive(Debug, Clone)]
pub(crate) struct ReportRow {
    pub nid: String,
    pub suffix: String,
    pub module_name: String,
    pub relative_path: String,
    pub nid_offset: u64,
    pub matched_ascii_string: String,
    pub string_offset: Option<u64>,
    pub offset_distance: Option<u64>,
    pub string_category: String,
    pub match_type: MatchType,
    pub confidence: String,
    pub mapping_source: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FailedFile {
    pub relative_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ScanStats {
    pub modules_found: usize,
    pub modules_scanned: usize,
    pub modules_failed: usize,
    pub modules_skipped_backup: usize,
    pub traversal_errors: Vec<String>,
    pub failed_files: Vec<FailedFile>,
    pub nid_count: usize,
    pub string_count: usize,
    pub nearby_rows: usize,
    pub same_file_rows: usize,
    pub unmatched_rows: usize,
    pub known_rows: usize,
    pub truncated_pairs: usize,
    pub truncated_nids: usize,
    pub filtered_known: usize,
}

impl ScanStats {
    pub fn row_total(&self) -> usize {
        self.nearby_rows + self.same_file_rows + self.unmatched_rows + self.known_rows
    }

    pub fn incomplete(&self) -> bool {
        self.modules_failed > 0 || !self.traversal_errors.is_empty()
    }
}

#[derive(Debug, Default)]
pub(crate) struct ScanReport {
    pub rows: Vec<ReportRow>,
    pub stats: ScanStats,
}

pub(crate) struct ScanOptions {
    pub nearby_bytes: u64,
    pub min_string_length: usize,
    pub max_candidates_per_nid: usize,
    pub strings_output: Option<PathBuf>,
    pub unknown_only: bool,
    pub include_eboot: bool,
}

fn is_nid_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-')
}

fn is_printable(b: u8) -> bool {
    b.is_ascii_graphic() || b == b' '
}

fn match_at(data: &[u8], start: usize) -> Option<NidRecord> {
    let core = &data[start..start + MATCH_LEN];
    if !core[..NID_LEN].iter().all(|&b| is_nid_byte(b)) {
        return None;
    }
    if core[NID_LEN] != b'#'
        || !core[NID_LEN + 1].is_ascii_uppercase()
        || core[NID_LEN + 2] != b'#'
        || !core[NID_LEN + 3].is_ascii_uppercase()
    {
        return None;
    }
    if start > 0 {
        let prev = data[start - 1];
        if prev == b'#' || is_nid_byte(prev) {
            return None;
        }
    }
    let nid = std::str::from_utf8(&core[..NID_LEN]).ok()?;
    let suffix = std::str::from_utf8(&core[NID_LEN + 1..]).ok()?;
    Some(NidRecord {
        nid: nid.to_string(),
        suffix: suffix.to_string(),
        offset: start as u64,
    })
}

pub(crate) fn scan_nids(data: &[u8]) -> Vec<NidRecord> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    if data.len() < MATCH_LEN {
        return hits;
    }
    let mut i = 0;
    while i + MATCH_LEN <= data.len() {
        if let Some(hit) = match_at(data, i)
            && seen.insert(format!("{}#{}", hit.nid, hit.suffix))
        {
            hits.push(hit);
        }
        i += 1;
    }
    hits
}

fn contains_fmod_folded(text: &str) -> bool {
    text.as_bytes()
        .windows(4)
        .any(|w| w.eq_ignore_ascii_case(b"fmod"))
}

fn looks_like_cpp_method(text: &str) -> bool {
    let Some((head, tail)) = text.split_once("::") else {
        return false;
    };
    !tail.is_empty()
        && head.starts_with(|c: char| c.is_ascii_uppercase())
        && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn classify(text: &str) -> StringCategory {
    if text.starts_with(WRAPPER_PREFIX) {
        return StringCategory::CompatWrapper;
    }
    if text.starts_with("sceKernel") {
        return StringCategory::SceKernelApi;
    }
    if contains_fmod_folded(text) || FMOD_CLASS_PREFIXES.iter().any(|p| text.starts_with(p)) {
        return StringCategory::FmodApi;
    }
    if text.starts_with("sce") {
        return StringCategory::SceApi;
    }
    if looks_like_cpp_method(text) {
        return StringCategory::CppMethod;
    }
    StringCategory::Other
}

pub(crate) fn scan_strings(data: &[u8], min_len: usize) -> Vec<StringRecord> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut flush = |end: usize, start: &mut Option<usize>| {
        if let Some(s) = start.take()
            && end - s >= min_len
            && let Ok(text) = std::str::from_utf8(&data[s..end])
        {
            out.push(StringRecord {
                category: classify(text),
                text: text.to_string(),
                offset: s as u64,
            });
        }
    };
    for (i, &b) in data.iter().enumerate() {
        if is_printable(b) {
            if start.is_none() {
                start = Some(i);
            }
        } else {
            flush(i, &mut start);
        }
    }
    flush(data.len(), &mut start);
    out
}

fn is_prx_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if BACKUP_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
        return false;
    }
    lower.ends_with(".prx") || lower.ends_with(".sprx")
}

fn is_backup_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    BACKUP_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

fn is_eboot_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("eboot.bin")
}

pub(crate) struct Collected {
    pub files: Vec<PathBuf>,
    pub skipped_backups: usize,
    pub traversal_errors: Vec<String>,
}

pub(crate) fn collect_prx_files(
    root: &Path,
    out: &mut Collected,
    depth: usize,
    include_eboot: bool,
) {
    if depth > MAX_WALK_DEPTH {
        out.traversal_errors
            .push(format!("depth limit reached at {}", root.display()));
        return;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) => {
            out.traversal_errors
                .push(format!("cannot list {}: {e}", root.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if is_prx_name(&name) || (include_eboot && is_eboot_name(&name)) {
                out.files.push(path);
            } else if is_backup_name(&name) {
                out.skipped_backups += 1;
            }
        } else if path.is_dir() {
            collect_prx_files(&path, out, depth + 1, include_eboot);
        }
    }
}

fn overlaps_nid(str_offset: u64, str_len: u64, nid_offset: u64) -> bool {
    str_offset < nid_offset + MATCH_LEN as u64 && nid_offset < str_offset + str_len
}

fn is_nid_shaped(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == MATCH_LEN
        && b[..NID_LEN].iter().all(|&c| is_nid_byte(c))
        && b[NID_LEN] == b'#'
        && b[NID_LEN + 1].is_ascii_uppercase()
        && b[NID_LEN + 2] == b'#'
        && b[NID_LEN + 3].is_ascii_uppercase()
}

#[allow(clippy::too_many_arguments)]
fn correlate_file(
    nids: &[NidRecord],
    strings: &[StringRecord],
    module_name: &str,
    relative_path: &str,
    nearby_bytes: u64,
    max_candidates: usize,
    catalog: Option<&ps5_nid::Catalog>,
    unknown_only: bool,
    rows: &mut Vec<ReportRow>,
    stats: &mut ScanStats,
) {
    for nid in nids {
        if unknown_only && catalog.is_some_and(|c| c.resolve(&nid.nid).is_some()) {
            stats.filtered_known += 1;
            continue;
        }
        let lo = nid.offset.saturating_sub(nearby_bytes);
        let hi = nid.offset.saturating_add(nearby_bytes);
        let window_start = strings.partition_point(|s| s.offset < lo);
        // NID-shaped runs are already NID records; letting them fill the
        // capped candidate window would drown out named strings with
        // neighboring symtab entries. They stay in the string inventory.
        let mut candidates: Vec<&StringRecord> = strings[window_start..]
            .iter()
            .take_while(|s| s.offset <= hi)
            .filter(|s| {
                !overlaps_nid(s.offset, s.text.len() as u64, nid.offset) && !is_nid_shaped(&s.text)
            })
            .collect();
        candidates.sort_by_key(|s| nid.offset.abs_diff(s.offset));
        if candidates.len() > max_candidates {
            stats.truncated_pairs += candidates.len() - max_candidates;
            stats.truncated_nids += 1;
            candidates.truncate(max_candidates);
        }
        if candidates.is_empty() {
            if strings.is_empty() {
                rows.push(ReportRow {
                    nid: nid.nid.clone(),
                    suffix: nid.suffix.clone(),
                    module_name: module_name.to_string(),
                    relative_path: relative_path.to_string(),
                    nid_offset: nid.offset,
                    matched_ascii_string: String::new(),
                    string_offset: None,
                    offset_distance: None,
                    string_category: String::new(),
                    match_type: MatchType::Unmatched,
                    confidence: MatchType::Unmatched.confidence().to_string(),
                    mapping_source: String::new(),
                });
                stats.unmatched_rows += 1;
            } else {
                rows.push(ReportRow {
                    nid: nid.nid.clone(),
                    suffix: nid.suffix.clone(),
                    module_name: module_name.to_string(),
                    relative_path: relative_path.to_string(),
                    nid_offset: nid.offset,
                    matched_ascii_string: String::new(),
                    string_offset: None,
                    offset_distance: None,
                    string_category: String::new(),
                    match_type: MatchType::SameFileCandidate,
                    confidence: MatchType::SameFileCandidate.confidence().to_string(),
                    mapping_source: String::new(),
                });
                stats.same_file_rows += 1;
            }
        } else {
            for s in candidates {
                rows.push(ReportRow {
                    nid: nid.nid.clone(),
                    suffix: nid.suffix.clone(),
                    module_name: module_name.to_string(),
                    relative_path: relative_path.to_string(),
                    nid_offset: nid.offset,
                    matched_ascii_string: s.text.clone(),
                    string_offset: Some(s.offset),
                    offset_distance: Some(nid.offset.abs_diff(s.offset)),
                    string_category: s.category.as_str().to_string(),
                    match_type: MatchType::NearbyCandidate,
                    confidence: MatchType::NearbyCandidate.confidence().to_string(),
                    mapping_source: String::new(),
                });
                stats.nearby_rows += 1;
            }
        }
        if let Some(entry) = catalog.and_then(|c| c.resolve(&nid.nid)) {
            let source = if entry.sources.is_empty() {
                "builtin-catalog".to_string()
            } else {
                entry.sources.iter().cloned().collect::<Vec<_>>().join(";")
            };
            for name in &entry.names {
                rows.push(ReportRow {
                    nid: nid.nid.clone(),
                    suffix: nid.suffix.clone(),
                    module_name: module_name.to_string(),
                    relative_path: relative_path.to_string(),
                    nid_offset: nid.offset,
                    matched_ascii_string: name.clone(),
                    string_offset: None,
                    offset_distance: None,
                    string_category: String::new(),
                    match_type: MatchType::KnownMapping,
                    confidence: MatchType::KnownMapping.confidence().to_string(),
                    mapping_source: source.clone(),
                });
                stats.known_rows += 1;
            }
        }
    }
}

fn hex_offset(value: u64) -> String {
    format!("0x{value:x}")
}

fn write_primary_csv(path: &Path, rows: &[ReportRow]) -> Result<(), ScanError> {
    let file = std::fs::File::create(path).map_err(|source| ScanError::ReportCreate {
        path: path.to_path_buf(),
        source,
    })?;
    let mut writer = csv::Writer::from_writer(file);
    writer
        .write_record([
            "nid",
            "suffix",
            "module_name",
            "relative_path",
            "nid_offset",
            "matched_ascii_string",
            "string_offset",
            "offset_distance",
            "string_category",
            "match_type",
            "confidence",
            "mapping_source",
        ])
        .map_err(|source| ScanError::ReportWrite {
            path: path.to_path_buf(),
            source,
        })?;
    for row in rows {
        let nid_offset = hex_offset(row.nid_offset);
        let string_offset = row.string_offset.map(hex_offset).unwrap_or_default();
        let distance = row
            .offset_distance
            .map(|d| d.to_string())
            .unwrap_or_default();
        writer
            .write_record([
                row.nid.as_str(),
                row.suffix.as_str(),
                row.module_name.as_str(),
                row.relative_path.as_str(),
                nid_offset.as_str(),
                row.matched_ascii_string.as_str(),
                string_offset.as_str(),
                distance.as_str(),
                row.string_category.as_str(),
                row.match_type.as_str(),
                row.confidence.as_str(),
                row.mapping_source.as_str(),
            ])
            .map_err(|source| ScanError::ReportWrite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    writer.flush().map_err(|source| ScanError::ReportWrite {
        path: path.to_path_buf(),
        source: source.into(),
    })?;
    Ok(())
}

fn write_strings_header(
    writer: &mut csv::Writer<std::fs::File>,
    path: &Path,
) -> Result<(), ScanError> {
    writer
        .write_record([
            "string",
            "string_offset",
            "string_length",
            "string_category",
            "module_name",
            "relative_path",
        ])
        .map_err(|source| ScanError::ReportWrite {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(())
}

pub(crate) fn run_scan(
    root: &Path,
    opts: &ScanOptions,
    catalog: Option<&ps5_nid::Catalog>,
    read_file: impl for<'a> Fn(&'a Path) -> std::io::Result<Vec<u8>>,
) -> Result<ScanReport, ScanError> {
    if opts.min_string_length == 0 {
        return Err(ScanError::BadMinLength(0));
    }
    if !root.is_dir() {
        return Err(ScanError::RootNotDirectory(root.to_path_buf()));
    }

    let mut collected = Collected {
        files: Vec::new(),
        skipped_backups: 0,
        traversal_errors: Vec::new(),
    };
    collect_prx_files(root, &mut collected, 0, opts.include_eboot);
    collected.files.sort();
    if collected.files.is_empty() {
        return Err(ScanError::NoModules(root.to_path_buf()));
    }

    let mut strings_writer = match &opts.strings_output {
        Some(path) => {
            let file = std::fs::File::create(path).map_err(|source| ScanError::ReportCreate {
                path: path.clone(),
                source,
            })?;
            let mut writer = csv::Writer::from_writer(file);
            write_strings_header(&mut writer, path)?;
            Some(writer)
        }
        None => None,
    };

    let mut report = ScanReport::default();
    report.stats.modules_found = collected.files.len();
    report.stats.modules_skipped_backup = collected.skipped_backups;
    report.stats.traversal_errors = collected.traversal_errors;

    for file in &collected.files {
        let relative_path = file
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file.to_string_lossy().to_string());
        let module_name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.clone());
        let data = match read_file(file) {
            Ok(d) => d,
            Err(e) => {
                report.stats.modules_failed += 1;
                report.stats.failed_files.push(FailedFile {
                    relative_path,
                    reason: e.to_string(),
                });
                continue;
            }
        };
        let nids = scan_nids(&data);
        let strings = scan_strings(&data, opts.min_string_length);
        report.stats.nid_count += nids.len();
        report.stats.string_count += strings.len();
        if let Some(writer) = strings_writer.as_mut() {
            let path = opts
                .strings_output
                .as_deref()
                .unwrap_or_else(|| Path::new(""));
            for s in &strings {
                let offset = hex_offset(s.offset);
                let length = s.text.len().to_string();
                writer
                    .write_record([
                        s.text.as_str(),
                        offset.as_str(),
                        length.as_str(),
                        s.category.as_str(),
                        module_name.as_str(),
                        relative_path.as_str(),
                    ])
                    .map_err(|source| ScanError::ReportWrite {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
        }
        correlate_file(
            &nids,
            &strings,
            &module_name,
            &relative_path,
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            catalog,
            opts.unknown_only,
            &mut report.rows,
            &mut report.stats,
        );
        report.stats.modules_scanned += 1;
    }

    if let Some(writer) = strings_writer.as_mut() {
        let path = opts
            .strings_output
            .as_deref()
            .unwrap_or_else(|| Path::new(""));
        writer.flush().map_err(|source| ScanError::ReportWrite {
            path: path.to_path_buf(),
            source: source.into(),
        })?;
    }

    report.rows.sort_by(|a, b| {
        a.relative_path
            .cmp(&b.relative_path)
            .then(a.nid_offset.cmp(&b.nid_offset))
            .then(a.string_offset.cmp(&b.string_offset))
            .then(a.nid.cmp(&b.nid))
            .then(a.matched_ascii_string.cmp(&b.matched_ascii_string))
    });
    Ok(report)
}

fn read_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_fprx_nids(
    path: &Path,
    output: &Option<PathBuf>,
    nearby_bytes: u64,
    min_string_length: usize,
    max_candidates_per_nid: usize,
    extra_nids: &[PathBuf],
    strings_output: &Option<PathBuf>,
    unknown_only: bool,
    include_eboot: bool,
) {
    let out_path = output
        .clone()
        .unwrap_or_else(|| path.join(PRIMARY_CSV_NAME));
    let opts = ScanOptions {
        nearby_bytes,
        min_string_length,
        max_candidates_per_nid,
        strings_output: strings_output.clone(),
        unknown_only,
        include_eboot,
    };

    eprintln!("Scanning {} for PRX/SPRX modules...", path.display());
    let catalog = load_catalog(extra_nids);
    let report = match run_scan(path, &opts, Some(&catalog), read_bytes) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = write_primary_csv(&out_path, &report.rows) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }

    let stats = &report.stats;
    eprintln!(
        "Modules: {} found, {} scanned, {} failed, {} backup(s) skipped",
        stats.modules_found,
        stats.modules_scanned,
        stats.modules_failed,
        stats.modules_skipped_backup
    );
    for failed in &stats.failed_files {
        eprintln!("  failed: {} ({})", failed.relative_path, failed.reason);
    }
    for err in &stats.traversal_errors {
        eprintln!("  traversal: {err}");
    }
    eprintln!(
        "NID-shaped strings: {}, printable strings: {}",
        stats.nid_count, stats.string_count
    );
    eprintln!(
        "Rows: {} ({} nearby, {} same-file, {} unmatched, {} known)",
        stats.row_total(),
        stats.nearby_rows,
        stats.same_file_rows,
        stats.unmatched_rows,
        stats.known_rows
    );
    if stats.truncated_pairs > 0 {
        eprintln!(
            "  candidate cap: {} pair(s) beyond --max-candidates-per-nid {} across {} nid(s), nearest kept",
            stats.truncated_pairs, opts.max_candidates_per_nid, stats.truncated_nids
        );
    }
    if opts.unknown_only {
        eprintln!(
            "  unknown-only: {} catalog-known nid(s) filtered out",
            stats.filtered_known
        );
    }
    eprintln!("Report written to {}", out_path.display());
    if let Some(sp) = &opts.strings_output {
        eprintln!("String inventory written to {}", sp.display());
    }
    if stats.incomplete() {
        eprintln!("INCOMPLETE: some modules could not be read; results are partial");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ps5rs-fprx2-{}-{tag}", std::process::id()))
    }

    fn scan_opts() -> ScanOptions {
        ScanOptions {
            nearby_bytes: 4096,
            min_string_length: 4,
            max_candidates_per_nid: 25,
            strings_output: None,
            unknown_only: false,
            include_eboot: false,
        }
    }

    #[test]
    fn accepts_task_example_form() {
        let hits = scan_nids(b"\0amVp+qf6SDk#D#A\0");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].nid, "amVp+qf6SDk");
        assert_eq!(hits[0].suffix, "D#A");
        assert_eq!(hits[0].offset, 1);
    }

    #[test]
    fn accepts_slash_in_nid_alphabet() {
        let hits = scan_nids(b"\0ab/cd+ef-gH#Z#Q\0");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].nid, "ab/cd+ef-gH");
    }

    #[test]
    fn rejects_non_uppercase_suffix_letters() {
        for bad in [
            "amVp+qf6SDk#d#A",
            "amVp+qf6SDk#D#a",
            "amVp+qf6SDk#3#1",
            "amVp+qf6SDk#D#1",
            "amVp+qf6SDk##A",
            "amVp+qf6SDk#DA#",
        ] {
            let mut data = vec![0u8];
            data.extend_from_slice(bad.as_bytes());
            data.push(0);
            assert!(scan_nids(&data).is_empty(), "should reject {bad}");
        }
    }

    #[test]
    fn rejects_wrong_nid_length_and_partial_matches() {
        for bad in [
            "short#D#A",
            "amVp+qf6SD#D#A",
            "amVp+qf6SDk",
            "amVp+qf6SDk#",
            "amVp+qf6SDk#D",
            "amVp+qf6SDk#D#",
            "#D#A",
        ] {
            let mut data = vec![0u8];
            data.extend_from_slice(bad.as_bytes());
            data.push(0);
            assert!(scan_nids(&data).is_empty(), "should reject {bad}");
        }
    }

    #[test]
    fn rejects_extended_prefix_but_accepts_underscore_boundary() {
        assert!(scan_nids(b"\0XamVp+qf6SDk#D#A\0").is_empty());
        assert!(scan_nids(b"\0/amVp+qf6SDk#D#A\0").is_empty());
        assert!(scan_nids(b"\0#amVp+qf6SDk#D#A\0").is_empty());
        let hits = scan_nids(b"_amVp+qf6SDk#D#A\0");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].offset, 1);
    }

    #[test]
    fn no_nul_termination_required_and_eof_exact_match() {
        let hits = scan_nids(b"__amVp+qf6SDk#D#A__");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].offset, 2);
        let hits = scan_nids(b"__amVp+qf6SDk#D#A");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].offset, 2);
        assert!(scan_nids(b"short").is_empty());
        assert!(scan_nids(b"").is_empty());
    }

    #[test]
    fn dedups_repeats_keeps_first_offset() {
        let hits = scan_nids(b"\0amVp+qf6SDk#D#A\0junk\0amVp+qf6SDk#D#A\0");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].offset, 1);
    }

    #[test]
    fn same_nid_different_suffix_is_distinct() {
        let hits = scan_nids(b"\0amVp+qf6SDk#D#A\0amVp+qf6SDk#E#B\0");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn strings_keep_exact_offsets() {
        let data = b"\x01hi\x00System::getParameterLabelByName\xffokay\x00";
        let found = scan_strings(data, 4);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].text, "System::getParameterLabelByName");
        assert_eq!(found[0].offset, 4);
        assert_eq!(found[1].text, "okay");
        assert_eq!(found[1].offset, 36);
    }

    #[test]
    fn strings_handle_start_and_eof_runs() {
        let found = scan_strings(b"abcd\x00tail", 4);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].text.as_str(), found[0].offset), ("abcd", 0));
        assert_eq!((found[1].text.as_str(), found[1].offset), ("tail", 5));
    }

    #[test]
    fn strings_respect_min_length() {
        let data = b"\0ab\0abcd\0abcde\0";
        assert_eq!(scan_strings(data, 4).len(), 2);
        assert_eq!(scan_strings(data, 5).len(), 1);
        assert_eq!(scan_strings(data, 1).len(), 3);
    }

    #[test]
    fn strings_split_on_high_bytes_without_panic() {
        let data = b"\x80\xffAB\xC3\x28 lind\xe2\x82";
        let found = scan_strings(data, 4);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "( lind");
        assert_eq!(found[0].offset, 5);
    }

    #[test]
    fn classify_covers_all_categories() {
        assert_eq!(
            classify("EventDescription::getUserData"),
            StringCategory::FmodApi
        );
        assert_eq!(classify("EventInstance::isValid"), StringCategory::FmodApi);
        assert_eq!(
            classify("FMOD Studio update thread"),
            StringCategory::FmodApi
        );
        assert_eq!(
            classify("fmodstudio_real_getEvent"),
            StringCategory::CompatWrapper
        );
        assert_eq!(
            classify("sceKernelAprResolveFilepathsWithPrefixToIds"),
            StringCategory::SceKernelApi
        );
        assert_eq!(classify("scePthreadCreate"), StringCategory::SceApi);
        assert_eq!(
            classify("System::getParameterLabelByName"),
            StringCategory::CppMethod
        );
        assert_eq!(classify("plain text here"), StringCategory::Other);
        assert_eq!(classify("lower::method"), StringCategory::Other);
    }

    #[test]
    fn collect_finds_mixed_case_nested_and_counts_backups() {
        let root = temp_root("collect");
        std::fs::create_dir_all(root.join("a").join("sce_module")).unwrap();
        std::fs::write(root.join("TOP.PRX"), b"x").unwrap();
        std::fs::write(root.join("a").join("sce_module").join("mod.SPRX"), b"y").unwrap();
        std::fs::write(root.join("a").join("eboot.bin"), b"z").unwrap();
        std::fs::write(root.join("old.prx.esbak"), b"z").unwrap();
        std::fs::write(root.join("older.PRX.BAK"), b"z").unwrap();
        let mut out = Collected {
            files: Vec::new(),
            skipped_backups: 0,
            traversal_errors: Vec::new(),
        };
        collect_prx_files(&root, &mut out, 0, false);
        out.files.sort();
        assert_eq!(out.files.len(), 2);
        assert_eq!(out.skipped_backups, 2);
        assert!(out.traversal_errors.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_scan_rejects_bad_inputs() {
        let opts = scan_opts();
        let missing = PathBuf::from("definitely_missing_ps5rs_fprx_dir");
        assert!(matches!(
            run_scan(&missing, &opts, None, read_bytes),
            Err(ScanError::RootNotDirectory(_))
        ));
        let root = temp_root("empty");
        std::fs::create_dir_all(&root).unwrap();
        assert!(matches!(
            run_scan(&root, &opts, None, read_bytes),
            Err(ScanError::NoModules(_))
        ));
        let mut bad = scan_opts();
        bad.min_string_length = 0;
        assert!(matches!(
            run_scan(&root, &bad, None, read_bytes),
            Err(ScanError::BadMinLength(0))
        ));
        std::fs::write(root.join("note.txt"), b"x").unwrap();
        assert!(matches!(
            run_scan(&root.join("note.txt"), &opts, None, read_bytes),
            Err(ScanError::RootNotDirectory(_))
        ));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_scan_records_unreadable_files_and_stays_partial() {
        let root = temp_root("partial");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("good.prx"),
            b"\0amVp+qf6SDk#D#A\0nearby string here\0",
        )
        .unwrap();
        std::fs::write(root.join("bad.prx"), b"x").unwrap();
        let opts = scan_opts();
        let reader = |p: &Path| -> std::io::Result<Vec<u8>> {
            if p.file_name().is_some_and(|n| n == "bad.prx") {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                ))
            } else {
                std::fs::read(p)
            }
        };
        let report = run_scan(&root, &opts, None, reader).unwrap();
        assert_eq!(report.stats.modules_found, 2);
        assert_eq!(report.stats.modules_scanned, 1);
        assert_eq!(report.stats.modules_failed, 1);
        assert_eq!(report.stats.failed_files.len(), 1);
        assert_eq!(report.stats.failed_files[0].relative_path, "bad.prx");
        assert!(report.stats.incomplete());
        assert!(!report.rows.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn nearby_fixture(max_candidates: usize) -> (Vec<NidRecord>, Vec<StringRecord>, ScanOptions) {
        let nids = vec![NidRecord {
            nid: "amVp+qf6SDk".to_string(),
            suffix: "D#A".to_string(),
            offset: 1000,
        }];
        let strings = vec![
            StringRecord {
                text: "near one".to_string(),
                offset: 990,
                category: StringCategory::Other,
            },
            StringRecord {
                text: "near two".to_string(),
                offset: 1200,
                category: StringCategory::CppMethod,
            },
            StringRecord {
                text: "far away string".to_string(),
                offset: 9000,
                category: StringCategory::Other,
            },
        ];
        let mut opts = scan_opts();
        opts.nearby_bytes = 500;
        opts.max_candidates_per_nid = max_candidates;
        (nids, strings, opts)
    }

    #[test]
    fn nearby_and_same_file_rows() {
        let (nids, strings, opts) = nearby_fixture(25);
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            None,
            false,
            &mut rows,
            &mut stats,
        );
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|r| r.match_type == MatchType::NearbyCandidate && r.confidence == "low")
        );
        assert_eq!(stats.nearby_rows, 2);
        let mut far_rows = Vec::new();
        let mut far_stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings[2..],
            "m.prx",
            "m.prx",
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            None,
            false,
            &mut far_rows,
            &mut far_stats,
        );
        assert_eq!(far_rows.len(), 1);
        assert_eq!(far_rows[0].match_type, MatchType::SameFileCandidate);
        assert!(far_rows[0].matched_ascii_string.is_empty());
        assert!(far_rows[0].string_offset.is_none());
    }

    #[test]
    fn unmatched_when_file_has_no_strings() {
        let (nids, _, opts) = nearby_fixture(25);
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &[],
            "m.prx",
            "m.prx",
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            None,
            false,
            &mut rows,
            &mut stats,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].match_type, MatchType::Unmatched);
        assert!(rows[0].confidence.is_empty());
    }

    #[test]
    fn ambiguous_nearby_keeps_every_candidate() {
        let (nids, strings, mut opts) = nearby_fixture(25);
        opts.nearby_bytes = 9000;
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            None,
            false,
            &mut rows,
            &mut stats,
        );
        assert_eq!(rows.len(), 3);
        let distances: Vec<u64> = rows.iter().filter_map(|r| r.offset_distance).collect();
        assert!(distances.contains(&10));
        assert!(distances.contains(&200));
        assert!(distances.contains(&8000));
    }

    #[test]
    fn candidate_cap_keeps_nearest_and_counts_overflow() {
        let (nids, strings, mut opts) = nearby_fixture(1);
        opts.nearby_bytes = 9000;
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            opts.nearby_bytes,
            opts.max_candidates_per_nid,
            None,
            false,
            &mut rows,
            &mut stats,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].offset_distance, Some(10));
        assert_eq!(stats.truncated_pairs, 2);
        assert_eq!(stats.truncated_nids, 1);
    }

    #[test]
    fn repeated_strings_at_different_offsets_are_separate_records() {
        let data = b"\0same text\0longpad\0same text\0";
        let found = scan_strings(data, 4);
        assert_eq!(found.len(), 3);
        let repeats: Vec<&StringRecord> = found.iter().filter(|s| s.text == "same text").collect();
        assert_eq!(repeats.len(), 2);
        assert_ne!(repeats[0].offset, repeats[1].offset);
    }

    #[test]
    fn nid_shaped_strings_are_not_correlation_candidates() {
        assert!(is_nid_shaped("amVp+qf6SDk#D#A"));
        assert!(!is_nid_shaped("amVp+qf6SDk#D#Ax"));
        assert!(!is_nid_shaped("prefix amVp+qf6SDk#D#A"));
        assert!(!is_nid_shaped("sceKernelSleep"));
        let nids = vec![NidRecord {
            nid: "amVp+qf6SDk".to_string(),
            suffix: "D#A".to_string(),
            offset: 1000,
        }];
        let strings = vec![
            StringRecord {
                text: "amVp+qf6SDk#D#A".to_string(),
                offset: 1100,
                category: StringCategory::Other,
            },
            StringRecord {
                text: "sceKernelSleep".to_string(),
                offset: 1200,
                category: StringCategory::SceKernelApi,
            },
        ];
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids, &strings, "m.prx", "m.prx", 4096, 25, None, false, &mut rows, &mut stats,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].matched_ascii_string, "sceKernelSleep");
        assert_eq!(rows[0].string_category, "sce_kernel_api_candidate");
    }

    #[test]
    fn overlapping_string_is_not_its_own_candidate() {
        let mut data = vec![0u8; 500];
        let nid = b"amVp+qf6SDk#D#A";
        data[100..115].copy_from_slice(nid);
        let nids = scan_nids(&data);
        assert_eq!(nids.len(), 1);
        let mut strings = scan_strings(&data, 4);
        assert!(strings.iter().any(|s| s.text.contains("amVp+qf6SDk")));
        strings.push(StringRecord {
            text: "independent neighbour".to_string(),
            offset: 200,
            category: StringCategory::Other,
        });
        strings.sort_by_key(|s| s.offset);
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids, &strings, "m.prx", "m.prx", 500, 25, None, false, &mut rows, &mut stats,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].matched_ascii_string, "independent neighbour");
    }

    #[test]
    fn known_mapping_uses_catalog_names_and_sources() {
        let mut catalog = ps5_nid::Catalog::new();
        catalog.add("sceKernelSleep");
        let nid = ps5_nid::hash("sceKernelSleep");
        let mut data = vec![0u8; 64];
        let full = format!("{nid}#D#A");
        data[8..23].copy_from_slice(full.as_bytes());
        let nids = scan_nids(&data);
        assert_eq!(nids.len(), 1);
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &[],
            "m.prx",
            "m.prx",
            4096,
            25,
            Some(&catalog),
            false,
            &mut rows,
            &mut stats,
        );
        assert_eq!(rows.len(), 2);
        let known = rows
            .iter()
            .find(|r| r.match_type == MatchType::KnownMapping)
            .unwrap();
        assert_eq!(known.matched_ascii_string, "sceKernelSleep");
        assert_eq!(known.confidence, "high");
        assert!(!known.mapping_source.is_empty());
        assert!(rows.iter().any(|r| r.match_type == MatchType::Unmatched));
    }

    #[test]
    fn unknown_only_keeps_only_unresolved_nids() {
        let mut catalog = ps5_nid::Catalog::new();
        catalog.add("sceKernelSleep");
        let known = ps5_nid::hash("sceKernelSleep");
        let nids = vec![
            NidRecord {
                nid: known.clone(),
                suffix: "D#A".to_string(),
                offset: 100,
            },
            NidRecord {
                nid: "ZZZZZZZZZZZ".to_string(),
                suffix: "D#A".to_string(),
                offset: 9000,
            },
        ];
        let strings = vec![StringRecord {
            text: "some distant string".to_string(),
            offset: 5000,
            category: StringCategory::Other,
        }];
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            500,
            25,
            Some(&catalog),
            false,
            &mut rows,
            &mut stats,
        );
        assert!(rows.iter().any(|r| r.nid == known));
        assert!(rows.iter().any(|r| r.nid == "ZZZZZZZZZZZ"));
        assert_eq!(stats.filtered_known, 0);

        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            500,
            25,
            Some(&catalog),
            true,
            &mut rows,
            &mut stats,
        );
        assert!(!rows.iter().any(|r| r.nid == known));
        assert!(rows.iter().any(|r| r.nid == "ZZZZZZZZZZZ"));
        assert_eq!(stats.filtered_known, 1);
    }

    #[test]
    fn unknown_only_compares_against_csv_source() {
        let mut catalog = ps5_nid::Catalog::new();
        assert_eq!(catalog.load_nids_csv("AAAABBBCCCC,myFunc,libFoo,,\n"), 1);
        let nids = vec![
            NidRecord {
                nid: "AAAABBBCCCC".to_string(),
                suffix: "D#A".to_string(),
                offset: 100,
            },
            NidRecord {
                nid: "ZZZZZZZZZZZ".to_string(),
                suffix: "D#A".to_string(),
                offset: 200,
            },
        ];
        let strings = vec![StringRecord {
            text: "some distant string".to_string(),
            offset: 9000,
            category: StringCategory::Other,
        }];
        let mut rows = Vec::new();
        let mut stats = ScanStats::default();
        correlate_file(
            &nids,
            &strings,
            "m.prx",
            "m.prx",
            100,
            25,
            Some(&catalog),
            true,
            &mut rows,
            &mut stats,
        );
        assert!(!rows.iter().any(|r| r.nid == "AAAABBBCCCC"));
        assert!(rows.iter().any(|r| r.nid == "ZZZZZZZZZZZ"));
        assert_eq!(stats.filtered_known, 1);
    }

    #[test]
    fn unknown_only_end_to_end_filters_known_nids() {
        let root = temp_root("unknownonly");
        std::fs::create_dir_all(&root).unwrap();
        let mut catalog = ps5_nid::Catalog::new();
        catalog.add("sceKernelSleep");
        let known = ps5_nid::hash("sceKernelSleep");
        let mut a = vec![0u8; 200];
        let known_text = format!("{known}#D#A");
        a[10..25].copy_from_slice(known_text.as_bytes());
        a[60..75].copy_from_slice(b"QQQQQQQQQQQ#D#A");
        std::fs::write(root.join("a.prx"), &a).unwrap();
        let mut opts = scan_opts();
        opts.unknown_only = true;
        let report = run_scan(&root, &opts, Some(&catalog), read_bytes).unwrap();
        assert_eq!(report.stats.nid_count, 2);
        assert_eq!(report.stats.filtered_known, 1);
        assert!(!report.rows.iter().any(|r| r.nid == known));
        assert!(report.rows.iter().any(|r| r.nid == "QQQQQQQQQQQ"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn collect_includes_eboot_only_with_flag() {
        let root = temp_root("ebootwalk");
        std::fs::create_dir_all(root.join("game")).unwrap();
        std::fs::create_dir_all(root.join("patch")).unwrap();
        std::fs::write(root.join("game").join("eboot.bin"), b"x").unwrap();
        std::fs::write(root.join("patch").join("EBOOT.BIN"), b"x").unwrap();
        std::fs::write(root.join("game").join("eboot.bin.esbak"), b"x").unwrap();
        std::fs::write(root.join("game").join("mod.prx"), b"x").unwrap();
        let mut without = Collected {
            files: Vec::new(),
            skipped_backups: 0,
            traversal_errors: Vec::new(),
        };
        collect_prx_files(&root, &mut without, 0, false);
        assert_eq!(without.files.len(), 1);
        assert_eq!(without.skipped_backups, 1);
        let mut with = Collected {
            files: Vec::new(),
            skipped_backups: 0,
            traversal_errors: Vec::new(),
        };
        collect_prx_files(&root, &mut with, 0, true);
        with.files.sort();
        assert_eq!(with.files.len(), 3);
        assert_eq!(with.skipped_backups, 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn include_eboot_end_to_end_scans_eboot_bin() {
        let root = temp_root("eboote2e");
        std::fs::create_dir_all(&root).unwrap();
        let mut boot = vec![0u8; 200];
        boot[10..25].copy_from_slice(b"QQQQQQQQQQQ#D#A");
        boot[60..90].copy_from_slice(b"nearby eboot string here\0\0\0\0\0\0");
        std::fs::write(root.join("eboot.bin"), &boot).unwrap();
        let mut plain = scan_opts();
        plain.include_eboot = false;
        assert!(matches!(
            run_scan(&root, &plain, None, read_bytes),
            Err(ScanError::NoModules(_))
        ));
        let mut with = scan_opts();
        with.include_eboot = true;
        let report = run_scan(&root, &with, None, read_bytes).unwrap();
        assert_eq!(report.stats.modules_found, 1);
        assert_eq!(report.stats.nid_count, 1);
        assert!(
            report
                .rows
                .iter()
                .any(|r| r.module_name == "eboot.bin" && r.nid == "QQQQQQQQQQQ")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn primary_csv_schema_escapes_and_sorts_deterministically() {
        let rows = vec![
            ReportRow {
                nid: "a".to_string(),
                suffix: "D#A".to_string(),
                module_name: "a.prx".to_string(),
                relative_path: "a.prx".to_string(),
                nid_offset: 10,
                matched_ascii_string: String::new(),
                string_offset: None,
                offset_distance: None,
                string_category: String::new(),
                match_type: MatchType::Unmatched,
                confidence: String::new(),
                mapping_source: String::new(),
            },
            ReportRow {
                nid: "b".to_string(),
                suffix: "D#A".to_string(),
                module_name: "b.prx".to_string(),
                relative_path: "b.prx".to_string(),
                nid_offset: 30,
                matched_ascii_string: "says \"hi\", ok".to_string(),
                string_offset: Some(40),
                offset_distance: Some(10),
                string_category: "other".to_string(),
                match_type: MatchType::NearbyCandidate,
                confidence: "low".to_string(),
                mapping_source: String::new(),
            },
        ];
        let dir = temp_root("csv");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("r.csv");
        write_primary_csv(&path, &rows).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let header = text.lines().next().unwrap();
        assert_eq!(
            header,
            "nid,suffix,module_name,relative_path,nid_offset,\
             matched_ascii_string,string_offset,offset_distance,\
             string_category,match_type,confidence,mapping_source"
        );
        assert!(text.contains("\"says \"\"hi\"\", ok\""));
        let a_pos = text.find("\na,").unwrap();
        let b_pos = text.find("\nb,").unwrap();
        assert!(a_pos < b_pos);
        let parsed: Vec<Vec<String>> = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(text.as_bytes())
            .into_records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .into_iter()
            .map(|r| r.iter().map(str::to_string).collect())
            .collect();
        assert_eq!(parsed[2][5], "says \"hi\", ok");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn end_to_end_scan_writes_both_csvs() {
        let root = temp_root("e2e");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let mut a = vec![0u8; 300];
        a[10..25].copy_from_slice(b"amVp+qf6SDk#D#A");
        a[60..90].copy_from_slice(b"EventInstance::isValid\0\0\0\0\0\0\0\0");
        std::fs::write(root.join("a.prx"), &a).unwrap();
        let mut b = vec![0u8; 300];
        b[20..35].copy_from_slice(b"amVp+qf6SDk#D#A");
        std::fs::write(root.join("sub").join("b.SPRX"), &b).unwrap();
        let mut opts = scan_opts();
        opts.strings_output = Some(root.join("strings.csv"));
        let report = run_scan(&root, &opts, None, read_bytes).unwrap();
        assert_eq!(report.stats.modules_found, 2);
        assert_eq!(report.stats.modules_failed, 0);
        assert!(!report.stats.incomplete());
        assert_eq!(report.stats.nid_count, 2);
        let paths: Vec<&str> = report
            .rows
            .iter()
            .map(|r| r.relative_path.as_str())
            .collect();
        assert!(paths.contains(&"a.prx"));
        let b_rel = Path::new("sub")
            .join("b.SPRX")
            .to_string_lossy()
            .to_string();
        assert!(paths.contains(&b_rel.as_str()));
        let a_nearby: Vec<&ReportRow> = report
            .rows
            .iter()
            .filter(|r| r.relative_path == "a.prx" && r.match_type == MatchType::NearbyCandidate)
            .collect();
        assert_eq!(a_nearby.len(), 1);
        assert_eq!(a_nearby[0].matched_ascii_string, "EventInstance::isValid");
        assert_eq!(a_nearby[0].string_category, "fmod_api_candidate");
        let out = root.join("nids.csv");
        write_primary_csv(&out, &report.rows).unwrap();
        assert!(out.is_file());
        let strings_text = std::fs::read_to_string(root.join("strings.csv")).unwrap();
        assert!(strings_text.starts_with("string,string_offset,"));
        assert!(strings_text.contains("EventInstance::isValid"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
