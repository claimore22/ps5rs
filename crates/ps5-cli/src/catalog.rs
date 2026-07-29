use std::path::{Path, PathBuf};

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

pub(crate) fn cmd_sync(key: Option<&str>, catalog_dir: &Path) {
    let key = resolve_supabase_key(key).unwrap_or_else(|| SUPABASE_PUBLISHABLE_KEY.to_string());

    let supabase_url = SUPABASE_DEFAULT_URL;
    let url = format!("{supabase_url}/rest/v1/catalog_export?select=nid,name,library,tag,source&order=nid");

    std::fs::create_dir_all(catalog_dir).expect("failed to create catalog directory");

    let existing_meta = std::fs::read_to_string(catalog_dir.join("metadata.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<CatalogMetadata>(&s).ok());

    eprintln!("Downloading catalog from Supabase...");

    let response = ureq::get(&url)
        .set("apikey", &key)
        .call()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to fetch catalog: {e}");
            std::process::exit(1);
        });

    let body = response.into_string().unwrap_or_else(|e| {
        eprintln!("error: failed to read response: {e}");
        std::process::exit(1);
    });

    let sha256 = ps5_format::sha256_hex(body.as_bytes());

    if existing_meta.as_ref().is_some_and(|m| m.sha256 == sha256) {
        eprintln!("Catalog unchanged (sha256: {sha256})");
        return;
    }

    let rows: Vec<CatalogRow> = serde_json::from_str(&body).unwrap_or_else(|e| {
        eprintln!("error: failed to parse catalog JSON: {e}");
        std::process::exit(1);
    });

    let nids_path = catalog_dir.join("nids.csv");
    let mut writer = csv::Writer::from_path(&nids_path).unwrap_or_else(|e| {
        eprintln!("error: failed to create {}: {e}", nids_path.display());
        std::process::exit(1);
    });

    writer.write_record(["nid", "name", "library", "tag", "source"]).unwrap();
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

pub(crate) fn cmd_push_unknown(input: &Path, key: Option<&str>, url: Option<&str>, submitter: Option<&str>) {
    let Some(key) = resolve_supabase_key(key) else {
        eprintln!("{KEY_INSTRUCTIONS}");
        eprintln!("push-unknown requires a Supabase key (reads use the default, writes require explicit configuration).");
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

    eprintln!("Submitting {} unknown NIDs for review...", submissions.len());

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

fn iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    // Approximate year from days since epoch (within a few days)
    let year = 1970 + (days / 365) as u64;
    format!("{year:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", 1, 1)
}
