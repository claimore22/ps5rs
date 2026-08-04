//! Execution trace: every import call the guest makes through the HLE
//! boundary, collected during one run.

/// Version of the [`ExecutionReport`] shape; bump on breaking field changes.
pub const EXECUTION_REPORT_VERSION: u32 = 1;

/// One import call the guest dispatched to an HLE handler.
///
/// Appended to the trace only after the handler returns successfully; a
/// handler that escapes (`exit` / `catchReturnFromMain`) or errors is never
/// recorded, because from the guest's perspective the call did not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImportCall {
    /// Library tag from the masked symbol (e.g. `libc`).
    pub library: String,
    /// Readable symbol name when the catalog resolves it, else the NID string.
    pub name: String,
    /// The six SysV register arguments in `rdi..r9`.
    pub args: [u64; 6],
    /// The value the HLE handler returned to the guest.
    pub return_value: u64,
}

/// The result of one guest run: its exit code plus the import trace.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionReport {
    /// Shape version; see [`EXECUTION_REPORT_VERSION`].
    pub version: u32,
    /// Exit code the guest produced (`exit` / `catchReturnFromMain` argument).
    pub exit_code: u64,
    /// Canonical name of the module hosting the entry point.
    pub module_name: String,
    /// Guest virtual address of the entry point.
    pub entry_point: u64,
    /// Import calls in the order the guest made them.
    pub import_calls: Vec<ImportCall>,
    /// Chunks the guest wrote to stdout through the HLE modules, in order.
    #[cfg_attr(feature = "serde", serde(default))]
    pub output_lines: Vec<String>,
}
