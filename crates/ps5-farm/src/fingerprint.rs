use serde::{Deserialize, Serialize};
use std::path::Path;

/// Version of the fingerprint construction protocol. Bumping this
/// intentionally invalidates every stored report exactly once.
pub const FINGERPRINT_FORMAT: u32 = 1;
/// Version of the GameLoadReport shape. Bumped when report fields change.
pub const REPORT_FORMAT: u32 = 1;

const LOADER_SOURCES_HASH: &str = env!("PS5RS_LOADER_SOURCES_HASH");
const NID_CATALOG_HASH: &str = env!("PS5RS_NID_CATALOG_HASH");
const RUSTC_TAG: &str = env!("PS5RS_RUSTC_TAG");
const BUILD_ID: &str = env!("PS5RS_BUILD_ID");

/// Diagnostic components proving a stored report's freshness.
/// Comparison short-circuits on the committed hashes, but every component
/// is retained so a reload can explain itself without reverse-engineering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFreshness {
    pub report_format: u32,
    pub loader_fingerprint: String,
    pub offline_fingerprint: String,
    pub environment_fingerprint: String,
    pub game_fingerprint: String,
}

/// Why a stored report was rejected for reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    Fresh,
    FormatChanged,
    EnvironmentChanged,
    GameInputsChanged,
}

fn push_bytes(buf: &mut Vec<u8>, b: &[u8]) {
    buf.extend_from_slice(&(b.len() as u64).to_le_bytes());
    buf.extend_from_slice(b);
}

fn framed_hash(domain: &str, parts: &[&[u8]]) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(domain.as_bytes());
    buf.extend_from_slice(&FINGERPRINT_FORMAT.to_le_bytes());
    for part in parts {
        push_bytes(&mut buf, part);
    }
    ps5_format::sha256_hex(&buf)
}

/// loader_fingerprint = hash(build identity, sources, catalog, toolchain, format).
pub fn loader_fingerprint() -> String {
    framed_hash(
        "LOADER-FINGERPRINT-v1",
        &[
            BUILD_ID.as_bytes(),
            LOADER_SOURCES_HASH.as_bytes(),
            NID_CATALOG_HASH.as_bytes(),
            RUSTC_TAG.as_bytes(),
            &REPORT_FORMAT.to_le_bytes(),
        ],
    )
}

/// Hash of the resolved offline/system-module directory: sorted relative
/// paths plus file bytes. A missing directory is a valid "no offline data"
/// configuration and hashes as the empty set; any I/O problem while
/// walking an existing directory is an error, never a silent "unchanged".
pub fn offline_fingerprint(dir: &Path) -> Result<String, String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    if dir.is_dir() {
        collect_offline(dir, dir, &mut files)?;
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut buf = Vec::new();
    buf.extend_from_slice(b"OFFLINE-v1");
    push_bytes(&mut buf, &(files.len() as u64).to_le_bytes());
    for (rel, bytes) in &files {
        push_bytes(&mut buf, rel.as_bytes());
        push_bytes(&mut buf, &(bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(bytes);
    }
    Ok(ps5_format::sha256_hex(&buf))
}

fn collect_offline(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_offline(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("cannot relativize {}: {e}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes =
                std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            out.push((rel, bytes));
        }
    }
    Ok(())
}

pub fn environment_fingerprint(loader_fp: &str, offline_fp: &str) -> String {
    framed_hash(
        "ENVIRONMENT-FINGERPRINT-v1",
        &[loader_fp.as_bytes(), offline_fp.as_bytes()],
    )
}

/// game_fingerprint = hash(env_fp, eboot bytes, sorted PRX name+bytes).
/// `prxs` must be sorted by name; this function sorts a copy defensively.
pub fn game_fingerprint(env_fp: &str, eboot: &[u8], prxs: &[(String, Vec<u8>)]) -> String {
    let mut sorted: Vec<&(String, Vec<u8>)> = prxs.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut buf = Vec::new();
    buf.extend_from_slice(b"GAME-FINGERPRINT-v1");
    push_bytes(&mut buf, env_fp.as_bytes());
    push_bytes(&mut buf, eboot);
    push_bytes(&mut buf, &(sorted.len() as u64).to_le_bytes());
    buf.extend_from_slice(b"PRX-COUNT");
    for (name, bytes) in sorted {
        push_bytes(&mut buf, name.as_bytes());
        push_bytes(&mut buf, bytes);
    }
    ps5_format::sha256_hex(&buf)
}

pub fn check_freshness(stored: &ReportFreshness, env_fp: &str, game_fp: &str) -> Staleness {
    if stored.report_format != REPORT_FORMAT {
        return Staleness::FormatChanged;
    }
    if stored.environment_fingerprint != env_fp {
        return Staleness::EnvironmentChanged;
    }
    if stored.game_fingerprint != game_fp {
        return Staleness::GameInputsChanged;
    }
    Staleness::Fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prxs() -> Vec<(String, Vec<u8>)> {
        vec![
            ("b.prx".to_string(), vec![4, 5]),
            ("a.prx".to_string(), vec![1, 2, 3]),
        ]
    }

    #[test]
    fn game_fingerprint_is_order_insensitive_for_prxs() {
        let a = game_fingerprint("env", b"eboot", &sample_prxs());
        let mut rev = sample_prxs();
        rev.reverse();
        let b = game_fingerprint("env", b"eboot", &rev);
        assert_eq!(a, b);
    }

    #[test]
    fn game_fingerprint_changes_with_any_input() {
        let base = game_fingerprint("env", b"eboot", &sample_prxs());
        assert_ne!(base, game_fingerprint("env2", b"eboot", &sample_prxs()));
        assert_ne!(base, game_fingerprint("env", b"eboot!", &sample_prxs()));
        assert_ne!(
            base,
            game_fingerprint("env", b"eboot", &[("a.prx".to_string(), vec![9])])
        );
    }

    #[test]
    fn check_freshness_reports_each_cause() {
        let stored = ReportFreshness {
            report_format: REPORT_FORMAT,
            loader_fingerprint: "l".to_string(),
            offline_fingerprint: "o".to_string(),
            environment_fingerprint: "env".to_string(),
            game_fingerprint: "game".to_string(),
        };
        assert_eq!(check_freshness(&stored, "env", "game"), Staleness::Fresh);
        assert_eq!(
            check_freshness(&stored, "env", "other"),
            Staleness::GameInputsChanged
        );
        assert_eq!(
            check_freshness(&stored, "other", "game"),
            Staleness::EnvironmentChanged
        );
        let mut old = stored.clone();
        old.report_format = REPORT_FORMAT + 1;
        assert_eq!(
            check_freshness(&old, "env", "game"),
            Staleness::FormatChanged
        );
    }

    #[test]
    fn offline_fingerprint_missing_dir_hashes_as_empty() {
        let empty_dir = std::env::temp_dir().join("ps5rs_fp_offline_empty");
        let _ = std::fs::remove_dir_all(&empty_dir);
        std::fs::create_dir_all(&empty_dir).unwrap();
        let missing = offline_fingerprint(Path::new("/definitely/not/here")).unwrap();
        let empty = offline_fingerprint(&empty_dir).unwrap();
        assert_eq!(missing, empty);
        let _ = std::fs::remove_dir_all(&empty_dir);
    }

    #[test]
    fn offline_fingerprint_distinguishes_content() {
        let dir = std::env::temp_dir().join("ps5rs_fp_offline");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.json"), b"{}").unwrap();
        let one = offline_fingerprint(&dir).unwrap();
        std::fs::write(dir.join("b.json"), b"{}").unwrap();
        let two = offline_fingerprint(&dir).unwrap();
        assert_ne!(one, two);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loader_fingerprint_is_stable_within_build() {
        assert_eq!(loader_fingerprint(), loader_fingerprint());
        assert!(!BUILD_ID.is_empty());
    }
}
