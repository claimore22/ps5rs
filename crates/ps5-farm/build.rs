use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Every crate whose source can affect GameLoadReport generation
/// must be included in LOADER_SOURCE_ROOTS.
const LOADER_SOURCE_ROOTS: [&str; 5] =
    ["ps5-loader", "ps5-elf", "ps5-prx", "ps5-nid", "ps5-format"];

fn fail(msg: String) -> ! {
    panic!("fingerprint build input unavailable (failing closed): {msg}");
}

fn push_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_bytes(buf: &mut Vec<u8>, b: &[u8]) {
    push_u64(buf, b.len() as u64);
    buf.extend_from_slice(b);
}

fn hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(root)
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", root.display())));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| fail(format!("cannot read entry: {e}")));
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs")
            || path.file_name().is_some_and(|n| n == "Cargo.toml")
        {
            out.push(path);
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let crates_dir = manifest_dir.join("..");
    let workspace_dir = manifest_dir.join("..").join("..");

    let mut framed: Vec<u8> = Vec::new();
    framed.extend_from_slice(b"LOADER-SOURCES-v1");

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for root in LOADER_SOURCE_ROOTS {
        let dir = crates_dir.join(root);
        if !dir.is_dir() {
            fail(format!("missing source root {}", dir.display()));
        }
        let mut root_files = Vec::new();
        collect_files(&dir, &mut root_files);
        for f in root_files {
            let rel = f
                .strip_prefix(&workspace_dir)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            files.push((rel, f));
        }
    }
    for extra in ["Cargo.toml", "Cargo.lock"] {
        let f = workspace_dir.join(extra);
        if !f.is_file() {
            fail(format!("missing workspace file {}", f.display()));
        }
        files.push((extra.to_string(), f));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);

    push_u64(&mut framed, files.len() as u64);
    for (rel, path) in &files {
        println!("cargo::rerun-if-changed={}", path.display());
        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", path.display())));
        push_bytes(&mut framed, rel.as_bytes());
        push_u64(&mut framed, bytes.len() as u64);
        framed.extend_from_slice(&bytes);
    }
    let catalog = manifest_dir
        .join("..")
        .join("..")
        .join("data")
        .join("nids.csv");
    println!("cargo::rerun-if-changed={}", catalog.display());
    let catalog_bytes = std::fs::read(&catalog)
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", catalog.display())));

    let rustc_vv = Command::new("rustc")
        .arg("-vV")
        .output()
        .unwrap_or_else(|e| fail(format!("cannot run rustc -vV: {e}")));
    if !rustc_vv.status.success() {
        fail("rustc -vV exited non-zero".to_string());
    }

    let build_id = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(&workspace_dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nogit".to_string());

    println!("cargo::rustc-check-cfg=cfg(fingerprint_build)");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=PS5RS_FORCE_BUILD_ID");
    let forced = std::env::var("PS5RS_FORCE_BUILD_ID")
        .ok()
        .filter(|s| !s.is_empty());
    let build_id = forced.unwrap_or(build_id);

    println!(
        "cargo::rustc-env=PS5RS_LOADER_SOURCES_HASH={}",
        hex(&framed)
    );
    println!(
        "cargo::rustc-env=PS5RS_NID_CATALOG_HASH={}",
        hex(&catalog_bytes)
    );
    println!("cargo::rustc-env=PS5RS_RUSTC_TAG={}", hex(&rustc_vv.stdout));
    println!("cargo::rustc-env=PS5RS_BUILD_ID={build_id}");
}
