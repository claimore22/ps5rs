use std::io::Write;
use std::path::{Path, PathBuf};

use ps5_elf::{StubSymbol, parse_stub_library, stub_library_name};

pub(crate) const NIDS_CSV: &str = include_str!("../../../data/nids.csv");

const SUPABASE_DEFAULT_URL: &str = "https://krvshlwmvzczpjvuizte.supabase.co";

const SUPABASE_PUBLISHABLE_KEY: &str = "sb_publishable_ClOIpIlMjajaStG_6RmBCA_O_dzcS9l";

const KEY_INSTRUCTIONS: &str = "\
No Supabase key configured.

Sync (read-only, uses default key automatically):
  ps5rs catalog sync

Push unknown NIDs (requires explicit key):
  export PS5RS_SUPABASE_KEY=sb_publishable_xxxxx
  ps5rs catalog push-unknown -i unknown.csv --key $PS5RS_SUPABASE_KEY

Or store it persistently:
  mkdir -p ~/.config/ps5rs
  echo '[catalog]
  supabase_key = \"sb_publishable_xxxxx\"' >> ~/.config/ps5rs/config.toml
";

fn load_config_key() -> Option<String> {
    let path = std::path::PathBuf::from(
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?,
    )
    .join(".config")
    .join("ps5rs")
    .join("config.toml");

    let content = std::fs::read_to_string(path).ok()?;
    let mut in_catalog = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[catalog]" {
            in_catalog = true;
            continue;
        }

        if line.starts_with('[') {
            in_catalog = false;
            continue;
        }

        if in_catalog && line.starts_with("supabase_key") {
            return line
                .split_once('=')
                .map(|(_, value)| value.trim().trim_matches('"').to_string());
        }
    }

    None
}

fn resolve_supabase_key(cli: Option<&str>) -> Option<String> {
    cli.map(str::to_owned)
        .or_else(|| std::env::var("PS5RS_SUPABASE_KEY").ok())
        .or_else(load_config_key)
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CatalogRow {
    nid: String,
    name: String,
    #[serde(default)]
    library: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    source: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CatalogMetadata {
    sha256: String,
    entries: usize,
    updated: String,
}

#[derive(serde::Deserialize)]
struct UnknownRow {
    nid: String,
    library: Option<String>,
    count: Option<usize>,
    games: Option<String>,
}

pub(crate) fn load_catalog(extra_nids: &[PathBuf]) -> ps5_nid::Catalog {
    let mut cat = ps5_nid::Catalog::new();
    let loaded = cat.load_nids_csv(NIDS_CSV);
    eprintln!("Loaded {} NID mappings from built-in catalog", loaded);

    for path in extra_nids {
        match cat.load_nids_csv_file(path) {
            Ok(n) => eprintln!("Loaded {} NID mappings from {}", n, path.display()),
            Err(e) => {
                eprintln!("error: cannot load {}: {e}", path.display());
                std::process::exit(1);
            }
        }
    }
    cat
}

const PAGE_SIZE: usize = 1000;

pub(crate) fn cmd_sync(key: Option<&str>, catalog_dir: &Path) {
    let key = resolve_supabase_key(key).unwrap_or_else(|| SUPABASE_PUBLISHABLE_KEY.to_string());

    let supabase_url = SUPABASE_DEFAULT_URL;
    let url = format!(
        "{supabase_url}/rest/v1/catalog_export?select=nid,name,library,tag,source&order=nid"
    );

    std::fs::create_dir_all(catalog_dir).expect("failed to create catalog directory");

    let existing_meta = std::fs::read_to_string(catalog_dir.join("metadata.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<CatalogMetadata>(&s).ok());

    eprintln!("Downloading catalog from Supabase...");

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(60))
        .build();

    let fetch_page = |agent: &ureq::Agent, start: usize| -> (Vec<CatalogRow>, bool) {
        let end = start + PAGE_SIZE - 1;

        let response = {
            let mut attempts = 0;
            loop {
                match agent
                    .get(&url)
                    .set("apikey", &key)
                    .set("Range", &format!("{start}-{end}"))
                    .call()
                {
                    Ok(resp) => break resp,
                    Err(ureq::Error::Status(code, _)) if code >= 500 && attempts < 3 => {
                        eprintln!("warning: page {start} returned {code}, retrying...");
                        std::thread::sleep(std::time::Duration::from_millis(1000 << attempts));
                        attempts += 1;
                    }
                    Err(e) => {
                        eprintln!("error: failed to fetch catalog page {start}: {e}");
                        std::process::exit(1);
                    }
                }
            }
        };

        let body = response.into_string().unwrap_or_else(|e| {
            eprintln!("error: failed to read response: {e}");
            std::process::exit(1);
        });

        let page: Vec<CatalogRow> = serde_json::from_str(&body).unwrap_or_else(|e| {
            eprintln!("error: failed to parse catalog JSON: {e}");
            std::process::exit(1);
        });

        let last = page.len() < PAGE_SIZE;
        (page, last)
    };

    let total = agent
        .head(&url)
        .set("apikey", &key)
        .set("Prefer", "count=exact")
        .set("Range", "0-0")
        .call()
        .ok()
        .and_then(|r| r.header("Content-Range").map(str::to_owned))
        .and_then(|cr| cr.split('/').nth(1).and_then(|t| t.parse::<usize>().ok()));

    let (page0, last0) = fetch_page(&agent, 0);
    let mut rows = page0;

    if !last0 {
        match total {
            Some(t) => {
                let pages = t.div_ceil(PAGE_SIZE);
                let next = std::sync::atomic::AtomicUsize::new(1);
                let results = std::sync::Mutex::new(
                    (0..pages)
                        .map(|_| None::<Vec<CatalogRow>>)
                        .collect::<Vec<_>>(),
                );

                std::thread::scope(|s| {
                    for _ in 0..1 {
                        let worker = ureq::AgentBuilder::new()
                            .timeout_connect(std::time::Duration::from_secs(15))
                            .timeout(std::time::Duration::from_secs(60))
                            .build();

                        let next = &next;
                        let results = &results;

                        s.spawn(move || {
                            loop {
                                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if i >= pages {
                                    break;
                                }
                                let (page, _) = fetch_page(&worker, i * PAGE_SIZE);
                                results.lock().unwrap()[i] = Some(page);
                            }
                        });
                    }
                });

                for slot in results.into_inner().unwrap() {
                    rows.extend(slot.unwrap_or_default());
                }
            }
            None => loop {
                let (page, last) = fetch_page(&agent, rows.len());
                rows.extend(page);
                if last {
                    break;
                }
            },
        }
    }

    let sha256 = ps5_format::sha256_hex(
        serde_json::to_string(&rows)
            .expect("failed to serialize catalog rows")
            .as_bytes(),
    );

    if existing_meta.as_ref().is_some_and(|m| m.sha256 == sha256) {
        eprintln!("Catalog unchanged (sha256: {sha256})");
        return;
    }

    let nids_path = catalog_dir.join("nids.csv");
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&nids_path)
        .unwrap_or_else(|e| {
            eprintln!("error: failed to create {}: {e}", nids_path.display());
            std::process::exit(1);
        });

    writer
        .write_record(["nid", "name", "library", "tag", "source"])
        .unwrap();
    for row in &rows {
        writer.serialize(row).unwrap();
    }
    writer.flush().unwrap();

    let updated = iso8601_now();
    let meta = CatalogMetadata {
        sha256: sha256.clone(),
        entries: rows.len(),
        updated,
    };
    let meta_json = serde_json::to_string_pretty(&meta).unwrap();
    std::fs::write(catalog_dir.join("metadata.json"), format!("{meta_json}\n")).unwrap();

    let prev_count = existing_meta.as_ref().map(|m| m.entries).unwrap_or(0);
    let diff = if rows.len() > prev_count {
        format!(" (+{} new)", rows.len() - prev_count)
    } else if rows.len() < prev_count {
        format!(" ({} removed)", prev_count - rows.len())
    } else {
        String::new()
    };

    eprintln!(
        "Downloaded NID catalog: {} entries (sha256: {}){}",
        rows.len(),
        &sha256[..16],
        diff,
    );
}

fn validate_github_username(name: &str) -> bool {
    if name.is_empty() || name.len() > 39 {
        return false;
    }

    let bytes = name.as_bytes();

    if bytes[0] == b'-' || bytes[name.len() - 1] == b'-' {
        return false;
    }

    let mut previous_dash = false;

    for &b in bytes {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => {
                previous_dash = false;
            }
            b'-' if !previous_dash => {
                previous_dash = true;
            }
            _ => return false,
        }
    }

    true
}

fn check_github_user(username: &str) -> Option<bool> {
    let url = format!("https://api.github.com/users/{username}");

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build();

    match agent
        .get(&url)
        .set("User-Agent", "ps5rs")
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(resp) => Some(resp.status() == 200),
        Err(_) => {
            eprintln!("warning: GitHub API unreachable, skipping verification");
            None
        }
    }
}

pub(crate) fn cmd_push_unknown(
    input: &Path,
    key: Option<&str>,
    url: Option<&str>,
    submitter: Option<&str>,
) {
    let Some(key) = resolve_supabase_key(key) else {
        eprintln!("{KEY_INSTRUCTIONS}");
        eprintln!(
            "push-unknown requires a Supabase key (reads use the default, writes require explicit configuration)."
        );
        std::process::exit(1);
    };

    let supabase_url = url.unwrap_or(SUPABASE_DEFAULT_URL);
    let submissions_url = format!("{supabase_url}/rest/v1/submissions");

    let (submitter_type, github_verified) = match submitter {
        Some(s) if !s.is_empty() => {
            if !validate_github_username(s) {
                eprintln!("error: invalid GitHub username \"{s}\"");
                std::process::exit(1);
            }

            let verified = check_github_user(s);
            if verified == Some(false) {
                eprintln!("warning: GitHub user \"{s}\" not found (submitting anyway)");
            }

            ("github", verified)
        }
        _ => ("anonymous", None),
    };

    let mut reader = csv::Reader::from_path(input).unwrap_or_else(|e| {
        eprintln!("error: failed to read {}: {e}", input.display());
        std::process::exit(1);
    });

    let mut seen = std::collections::HashSet::new();
    let mut submissions: Vec<(String, String, String, serde_json::Value)> = Vec::new();

    for result in reader.deserialize::<UnknownRow>() {
        let row = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: skipping malformed row: {e}");
                continue;
            }
        };

        let lib = row.library.unwrap_or_default();
        if !seen.insert((row.nid.clone(), lib.clone())) {
            continue;
        }

        let games: Vec<String> = row
            .games
            .as_deref()
            .map(|g| g.split(';').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let evidence = serde_json::json!({
            "source": "ps5rs",
            "count": row.count.unwrap_or(0),
            "games": games,
            "submitter": submitter.unwrap_or(""),
            "submitter_type": submitter_type,
            "github_verified": github_verified,
        });

        submissions.push((row.nid, lib, String::new(), evidence));
    }

    if submissions.is_empty() {
        eprintln!("No unknown NIDs to submit.");
        return;
    }

    eprintln!(
        "Submitting {} unknown NIDs for review...",
        submissions.len()
    );

    let mut submitted = 0usize;
    let mut errors = 0usize;

    for (nid, library, _name, evidence) in &submissions {
        let mut payload = serde_json::json!({
            "nid": nid,
            "name": nid,
            "library": library,
            "evidence": evidence,
            "source": "ps5rs",
        });

        if let Some(s) = submitter {
            payload["submitter"] = serde_json::Value::String(s.to_string());
            payload["submitter_type"] = serde_json::Value::String(submitter_type.to_string());
            if let Some(v) = github_verified {
                payload["github_verified"] = serde_json::Value::Bool(v);
            }
        }

        match ureq::post(&submissions_url)
            .set("apikey", &key)
            .set("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(_) => submitted += 1,
            Err(e) => {
                eprintln!("warning: failed to submit {nid}: {e}");
                errors += 1;
            }
        }
    }

    eprintln!(
        "Submitted {submitted} / {} unknown NIDs ({errors} errors)",
        submissions.len(),
    );
}

#[derive(Default)]
struct StubAggregate {
    entries: std::collections::BTreeMap<String, StubSymbol>,
    conflicts: Vec<(String, String, String)>,
}

impl StubAggregate {
    fn add(&mut self, symbols: impl Iterator<Item = StubSymbol>) {
        for s in symbols {
            match self.entries.get(&s.nid) {
                Some(existing) if existing.name != s.name => {
                    self.conflicts
                        .push((s.nid.clone(), existing.name.clone(), s.name.clone()));
                }
                Some(_) => {}
                None => {
                    self.entries.insert(s.nid.clone(), s);
                }
            }
        }
    }
}

fn stub_files(sdk_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let candidates = [sdk_dir.to_path_buf(), sdk_dir.join("target").join("lib")];
    for dir in candidates {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_stub = path.is_file()
                    && path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with("_stub_weak.a"));
                if is_stub {
                    files.push(path);
                }
            }
        }
        if !files.is_empty() {
            break;
        }
    }
    files.sort();
    files
}

pub(crate) fn cmd_import_stubs(sdk_dir: &Path, output: Option<&Path>, verify: bool) {
    let mut catalog = ps5_nid::Catalog::new();
    let loaded = catalog.load_nids_csv(NIDS_CSV);

    let files = stub_files(sdk_dir);
    if files.is_empty() {
        eprintln!(
            "error: no *_stub_weak.a files found under {} (also checked target/lib)",
            sdk_dir.display()
        );
        std::process::exit(1);
    }

    let mut total_symbols = 0usize;
    let mut aggregate = StubAggregate::default();
    let mut verify_mismatches = Vec::new();

    for path in &files {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read {}: {e}", path.display());
                std::process::exit(1);
            }
        };
        let library = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(stub_library_name)
            .unwrap_or("unknown")
            .to_string();
        let symbols = match parse_stub_library(&bytes, &library) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to parse {}: {e}", path.display());
                std::process::exit(1);
            }
        };
        if verify {
            for s in &symbols {
                let derived = ps5_nid::hash(&s.name);
                if derived != s.nid {
                    verify_mismatches.push((
                        s.nid.clone(),
                        s.name.clone(),
                        s.library.clone(),
                        derived,
                    ));
                }
            }
        }
        total_symbols += symbols.len();
        aggregate.add(symbols.into_iter());
    }

    let unique = aggregate.entries.len();
    let conflicts = aggregate.conflicts.len();
    let mut net_new: Vec<&StubSymbol> = aggregate
        .entries
        .values()
        .filter(|s| catalog.resolve(&s.nid).is_none())
        .collect();
    net_new.sort_by(|a, b| a.nid.cmp(&b.nid));

    if output.is_none() {
        for s in &net_new {
            println!("{} {}", s.nid, s.name);
        }
    }

    eprintln!("Loaded {loaded} NID mappings from built-in catalog");
    eprintln!(
        "Scanned {} stub libraries: {total_symbols} symbols, {unique} unique NIDs, {} net-new vs catalog, {conflicts} conflicts",
        files.len(),
        net_new.len(),
    );
    for (nid, first, second) in &aggregate.conflicts {
        eprintln!("conflict: {nid} => {first} vs {second}");
    }
    if verify {
        eprintln!(
            "verify: {} of {total_symbols} scenid NIDs differ from hash(name)",
            verify_mismatches.len()
        );
        for (nid, name, library, derived) in verify_mismatches.iter().take(20) {
            eprintln!("  {name} ({library}): scenid {nid} vs hash {derived}");
        }
    }

    if let Some(out) = output {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out)
            .unwrap_or_else(|e| {
                eprintln!("error: cannot open {}: {e}", out.display());
                std::process::exit(1);
            });
        for s in &net_new {
            writeln!(file, "{} {}", s.nid, s.name).unwrap_or_else(|e| {
                eprintln!("error: failed to write {}: {e}", out.display());
                std::process::exit(1);
            });
        }
        eprintln!(
            "Appended {} net-new entries to {}",
            net_new.len(),
            out.display()
        );
    }
}

pub(crate) fn iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    iso8601(secs)
}

fn iso8601(secs: u64) -> String {
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_epoch_is_unix_epoch() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn iso8601_handles_leap_day() {
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn iso8601_tracks_month_rollover() {
        assert_eq!(iso8601(951_868_800), "2000-03-01T00:00:00Z");
    }

    #[test]
    fn iso8601_rounds_time_of_day() {
        assert_eq!(iso8601(86399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn aggregate_keeps_first_and_reports_conflict() {
        let mut agg = StubAggregate::default();
        agg.add(
            [
                StubSymbol {
                    nid: "23LRUSvYu1M".into(),
                    name: "sceAgcInit".into(),
                    library: "libSceAgc".into(),
                },
                StubSymbol {
                    nid: "23LRUSvYu1M".into(),
                    name: "sceAgcInitAlias".into(),
                    library: "libSceAgc".into(),
                },
                StubSymbol {
                    nid: "23LRUSvYu1M".into(),
                    name: "sceAgcInit".into(),
                    library: "libSceAgc".into(),
                },
            ]
            .into_iter(),
        );
        assert_eq!(agg.entries.len(), 1);
        assert_eq!(agg.entries["23LRUSvYu1M"].name, "sceAgcInit");
        assert_eq!(agg.conflicts.len(), 1);
        assert_eq!(
            agg.conflicts[0],
            (
                "23LRUSvYu1M".into(),
                "sceAgcInit".into(),
                "sceAgcInitAlias".into()
            )
        );
    }

    #[test]
    fn stub_files_finds_only_stub_archives() {
        let dir = std::env::temp_dir().join("ps5rs_test_stub_files");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("libSceAgc_stub_weak.a"), b"!<arch>\n").unwrap();
        std::fs::write(dir.join("libSceAgc.a"), b"!<arch>\n").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hello").unwrap();
        let files = stub_files(&dir);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].file_name().unwrap().to_str().unwrap(),
            "libSceAgc_stub_weak.a"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stub_files_falls_back_to_target_lib() {
        let dir = std::env::temp_dir().join("ps5rs_test_stub_fallback");
        let lib = dir.join("target").join("lib");
        let _ = std::fs::create_dir_all(&lib);
        std::fs::write(lib.join("libkernel_stub_weak.a"), b"!<arch>\n").unwrap();
        let files = stub_files(&dir);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].file_name().unwrap().to_str().unwrap(),
            "libkernel_stub_weak.a"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stub_files_returns_empty_for_missing_dir() {
        let dir = std::env::temp_dir().join("ps5rs_test_stub_missing");
        assert!(stub_files(&dir).is_empty());
    }
}
