//! Data-driven end-to-end suite over the generated fixtures.
//!
//! Reads `data/test/generated_elfs/manifest.json` (written by the
//! `ps5-tests` generator), boots each fixture through the real loader + HLE
//! pipeline, and asserts its exit code and import trace match the manifest.
//! This is the public, SDK-free regression suite: no private PS5 binaries.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use ps5_emu::{EXECUTION_REPORT_VERSION, Emulator, ImportCall, Process};
use ps5_tests::manifest::Manifest;

static GUEST_LOCK: Mutex<()> = Mutex::new(());
static CATALOG: OnceLock<ps5_nid::Catalog> = OnceLock::new();

/// This binary runs as its own process, so it must not share the default
/// load base with the other test binaries (their identity-mapped
/// reservations would collide).
const LOAD_BASE: u64 = 0x830000000;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/test/generated_elfs")
}

fn catalog() -> &'static ps5_nid::Catalog {
    CATALOG.get_or_init(ps5_emu::nid::catalog)
}

fn read_manifest() -> Manifest {
    let raw = std::fs::read_to_string(fixture_dir().join("manifest.json"))
        .expect("read fixture manifest.json");
    serde_json::from_str(&raw).expect("parse fixture manifest.json")
}

fn expected_calls(manifest: &Manifest, fixture: &str) -> Vec<ImportCall> {
    manifest
        .fixtures
        .get(fixture)
        .map(|expectation| {
            expectation
                .imports
                .iter()
                .map(|import| ImportCall {
                    library: import.library.clone(),
                    name: import.name.clone(),
                    args: import.args,
                    return_value: import.return_value,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_manifest_sha256_matches_its_file() {
    let manifest = read_manifest();
    for (name, expectation) in &manifest.fixtures {
        let bytes = std::fs::read(fixture_dir().join(name))
            .unwrap_or_else(|_| panic!("missing fixture {name}"));
        assert_eq!(
            ps5_format::hash::sha256_hex(&bytes),
            expectation.sha256,
            "fixture {name} drifted from manifest; regenerate with `cargo run -p ps5-tests --bin generate`"
        );
    }
}

#[test]
fn every_fixture_boots_with_expected_report() {
    let _guard = GUEST_LOCK.lock().unwrap();
    let manifest = read_manifest();
    assert!(
        !manifest.fixtures.is_empty(),
        "no fixtures in manifest; generate them with `cargo run -p ps5-tests --bin generate`"
    );

    for (name, expectation) in &manifest.fixtures {
        let bytes = std::fs::read(fixture_dir().join(name))
            .unwrap_or_else(|_| panic!("missing fixture {name}"));
        let process = Process::load_at(name, bytes, |_| None, None, LOAD_BASE)
            .unwrap_or_else(|err| panic!("{name}: load failed: {err}"));
        let mut emulator = Emulator::new(process);
        emulator
            .resolve_imports_with(catalog())
            .unwrap_or_else(|err| panic!("{name}: resolve imports failed: {err}"));

        let report = emulator
            .run()
            .unwrap_or_else(|err| panic!("{name}: run failed: {err}"));
        assert_eq!(
            report.version, EXECUTION_REPORT_VERSION,
            "{name}: unexpected report version"
        );
        assert_eq!(
            report.exit_code, expectation.expected_exit,
            "{name}: unexpected exit code"
        );

        let expected = expected_calls(&manifest, name);
        assert_eq!(
            report.import_calls.len(),
            expected.len(),
            "{name}: unexpected import trace length"
        );
        for (got, want) in report.import_calls.iter().zip(expected.iter()) {
            assert_eq!(
                got.library, want.library,
                "{name}: unexpected import library"
            );
            assert_eq!(got.name, want.name, "{name}: unexpected import name");
            if want.args != [0; 6] {
                assert_eq!(got.args, want.args, "{name}: unexpected import args");
            }
            assert_eq!(
                got.return_value, want.return_value,
                "{name}: unexpected import return value"
            );
        }

        if let Some(text) = &expectation.print_string {
            let puts = report
                .import_calls
                .iter()
                .find(|call| call.library == "libkernel" && call.name == "puts")
                .unwrap_or_else(|| panic!("{name}: expected a libkernel::puts import call"));
            let printed = emulator
                .process()
                .read_string(puts.args[0])
                .unwrap_or_else(|err| panic!("{name}: read guest string failed: {err}"));
            assert_eq!(&printed, text, "{name}: guest printed the wrong string");
        }
    }
}
