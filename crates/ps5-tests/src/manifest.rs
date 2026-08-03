//! Fixture manifest: per-fixture expectations serialized to `manifest.json`
//! beside the generated binaries.  The data-driven emulator suite reads this
//! file to know what each fixture must do.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Expected guest behavior of one generated fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureExpectation {
    /// Hex SHA-256 of the fixture bytes, for regeneration drift detection.
    pub sha256: String,
    /// Exit code the guest must produce.
    pub expected_exit: u64,
    /// Exact import-call sequence the guest must produce, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ImportExpectation>,
    /// When set, the first import call must print exactly this string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print_string: Option<String>,
}

/// One expected import call during a fixture run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExpectation {
    /// Library tag from the masked symbol (e.g. `libc`).
    pub library: String,
    /// Readable symbol name (e.g. `puts`).
    pub name: String,
    /// Six SysV register arguments in `rdi..r9`.
    pub args: [u64; 6],
    /// Value the HLE handler returned to the guest.
    pub return_value: u64,
}

/// The whole manifest: fixture filename → expectations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub fixtures: BTreeMap<String, FixtureExpectation>,
}
