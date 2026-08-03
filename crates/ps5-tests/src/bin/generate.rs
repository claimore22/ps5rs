//! Regenerate all fixtures and their manifest into `data/test/generated_elfs/`.
//!
//! Output is byte-exact and deterministic; run it whenever
//! [`ps5_tests::fixtures`] changes and commit the result.

use std::fs;
use std::path::PathBuf;

use ps5_format::hash::sha256_hex;
use ps5_tests::fixtures;
use ps5_tests::manifest::{FixtureExpectation, Manifest};

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/test/generated_elfs");
    fs::create_dir_all(&out_dir).expect("create fixture output directory");

    let mut manifest = Manifest::default();
    for fixture in fixtures::all() {
        let sha = sha256_hex(&fixture.bytes);
        fs::write(out_dir.join(fixture.name), &fixture.bytes).expect("write generated fixture");
        manifest.fixtures.insert(
            fixture.name.to_string(),
            FixtureExpectation {
                sha256: sha.clone(),
                expected_exit: fixture.expected_exit,
                imports: fixture.imports,
                print_string: fixture.print_string.map(str::to_string),
            },
        );
        eprintln!(
            "wrote {} ({:>8} bytes, sha256 {sha})",
            fixture.name,
            fixture.bytes.len()
        );
    }

    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    fs::write(out_dir.join("manifest.json"), format!("{json}\n")).expect("write manifest");
    eprintln!(
        "wrote {} fixtures + manifest.json to {}",
        manifest.fixtures.len(),
        out_dir.display()
    );
}
