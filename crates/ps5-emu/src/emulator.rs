use crate::abi::{EscapeContext, arm_escape_ctx, disarm_escape_ctx, invoke_guest};
use crate::core::{Dispatcher, prepare};
use crate::error::EmuError;
use crate::imports::{ImportTable, build_import_table};
use crate::modules::{Host, Registry, kernel::KernelModule, libc::LibcModule, libdbg::DbgModule};
use crate::process::Process;
use crate::trace::{EXECUTION_REPORT_VERSION, ExecutionReport};

use ps5_loader::OfflineExportTable;

/// Lifecycle state of the emulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuState {
    /// Loaded, imports not yet resolved.
    Ready,
    /// Imports resolved; guest execution in progress.
    Running,
    /// Execution finished (or never started this milestone).
    Halted,
}

/// Host-side driver: owns the loaded [`Process`], the HLE [`Registry`], and
/// the emulator state machine.
pub struct Emulator {
    process: Process,
    registry: Registry,
    state: EmuState,
}

impl Emulator {
    /// Wrap a loaded [`Process`] with the default HLE module set.
    pub fn new(process: Process) -> Self {
        let mut registry = Registry::new();
        registry.register(KernelModule);
        registry.register(LibcModule::default());
        registry.register(DbgModule::default());
        Self {
            process,
            registry,
            state: EmuState::Ready,
        }
    }

    /// Load an eboot image (plus optional PRX dependencies) and wrap it with
    /// the default HLE module set.
    ///
    /// The eboot loads at [`ps5_loader::DEFAULT_LOAD_BASE`]; `prx_provider` is
    /// asked for each `DT_NEEDED` module by name.  This is the primary entry
    /// point for consumers that do not need the underlying [`Process`].
    pub fn from_elf(
        eboot_name: &str,
        eboot_bytes: Vec<u8>,
        prx_provider: impl FnMut(&str) -> Option<Vec<u8>>,
        offline: Option<&OfflineExportTable>,
    ) -> Result<Self, EmuError> {
        let process = Process::load(eboot_name, eboot_bytes, prx_provider, offline)?;
        Ok(Self::new(process))
    }

    pub fn process(&self) -> &Process {
        &self.process
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn state(&self) -> EmuState {
        self.state
    }

    /// Build the import table for all loaded modules.
    pub fn resolve_imports(&mut self) -> Result<&ImportTable, EmuError> {
        let table = build_import_table(self.process.context(), self.process.sources(), None)?;
        self.process.set_imports(table);
        Ok(self.process.imports().expect("imports just set"))
    }

    /// Resolve imports using a caller-supplied NID catalog for readable names.
    pub fn resolve_imports_with(
        &mut self,
        catalog: &ps5_nid::Catalog,
    ) -> Result<&ImportTable, EmuError> {
        let table = build_import_table(
            self.process.context(),
            self.process.sources(),
            Some(catalog),
        )?;
        self.process.set_imports(table);
        Ok(self.process.imports().expect("imports just set"))
    }

    /// Execute the guest to completion and return an [`ExecutionReport`]: the
    /// exit code, the entry module, and every import call the guest made.
    ///
    /// Materializes the guest address space, routes every import of the
    /// executable module through HLE stubs, and transfers control to the
    /// entry point.  Returns when the guest calls `exit` (or
    /// `catchReturnFromMain`), or when a naturally-returning entry finishes.
    pub fn run(&mut self) -> Result<ExecutionReport, EmuError> {
        if self.state != EmuState::Ready {
            return Err(EmuError::InvalidState(self.state));
        }
        let imports = self.process.imports().ok_or(EmuError::ImportsNotResolved)?;
        let (name, entry_point) = self.process.entry()?;
        let eboot = self.process.eboot().ok_or(EmuError::NoEntryPoint)?;
        let module_name = eboot.canonical_name().to_string();
        let executable_modules = vec![module_name.clone()];

        let registry = std::mem::take(&mut self.registry);
        let prepared = match prepare(&self.process, imports, &executable_modules, &registry) {
            Ok(prepared) => prepared,
            Err(err) => {
                self.registry = registry;
                return Err(err);
            }
        };
        let stack_top = prepared.stack_top;
        tracing::info!(%name, entry_point, stack_top, "run: start");

        let mut dispatcher =
            Dispatcher::new(registry, prepared.host, prepared.slots, prepared.stubs);

        dispatcher.install();

        let ctx = EscapeContext { rsp: 0, rbp: 0 };
        unsafe { arm_escape_ctx(&ctx) };
        let code = invoke_guest(entry_point, stack_top);
        disarm_escape_ctx();
        dispatcher.uninstall();

        let import_calls = dispatcher.calls().to_vec();
        let total_hits: u64 = dispatcher.hits().iter().sum();
        let slot_count = dispatcher.hits().len();
        self.registry = dispatcher.into_registry();
        self.state = EmuState::Halted;
        tracing::info!(%name, entry_point, code, total_hits, slot_count, "run: complete");
        Ok(ExecutionReport {
            version: EXECUTION_REPORT_VERSION,
            exit_code: code,
            module_name,
            entry_point,
            import_calls,
        })
    }
}

impl Host for Emulator {
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError> {
        self.process.read(addr, len)
    }

    fn read_string(&self, addr: u64) -> Result<String, EmuError> {
        self.process.read_string(addr)
    }

    fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError> {
        self.process.write(addr, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static GUEST_LOCK: Mutex<()> = Mutex::new(());

    fn minimal_elf(entry: u64) -> Vec<u8> {
        let mut buf = vec![0u8; 0x1100];
        buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        buf[4] = 2;
        buf[5] = 1;
        buf[6] = 1;
        buf[16..18].copy_from_slice(&0xFE10u16.to_le_bytes());
        buf[18..20].copy_from_slice(&62u16.to_le_bytes());
        buf[20..24].copy_from_slice(&1u32.to_le_bytes());
        buf[24..32].copy_from_slice(&entry.to_le_bytes());
        buf[32..40].copy_from_slice(&64u64.to_le_bytes());
        buf[52..54].copy_from_slice(&64u16.to_le_bytes());
        buf[54..56].copy_from_slice(&56u16.to_le_bytes());
        buf[56..58].copy_from_slice(&1u16.to_le_bytes());

        let phoff = 64usize;
        buf[phoff..phoff + 4].copy_from_slice(&1u32.to_le_bytes());
        buf[phoff + 4..phoff + 8].copy_from_slice(&5u32.to_le_bytes());
        buf[phoff + 8..phoff + 16].copy_from_slice(&0x1000u64.to_le_bytes());
        buf[phoff + 16..phoff + 24].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 24..phoff + 32].copy_from_slice(&0u64.to_le_bytes());
        buf[phoff + 32..phoff + 40].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 40..phoff + 48].copy_from_slice(&0x100u64.to_le_bytes());
        buf[phoff + 48..phoff + 56].copy_from_slice(&0x1000u64.to_le_bytes());

        buf[0x1000..0x1100].fill(0xCC);
        buf[0x1000] = 0xC3;
        buf[0x1020] = 0xC3;
        buf
    }

    #[test]
    fn run_without_imports_errors() {
        let process = Process::load("eboot.elf", minimal_elf(0), |_| None, None).unwrap();
        let mut emulator = Emulator::new(process);
        let err = emulator.run().unwrap_err();
        assert!(matches!(err, EmuError::ImportsNotResolved));
    }

    #[test]
    fn default_registry_has_sample_symbols() {
        let process = Process::load("eboot.elf", minimal_elf(0), |_| None, None).unwrap();
        let emulator = Emulator::new(process);
        assert!(
            emulator
                .registry()
                .resolve("sceDbgLoggingHandler")
                .is_some()
        );
        assert!(emulator.registry().resolve("puts").is_some());
        assert!(emulator.registry().resolve("printf").is_some());
        assert!(emulator.registry().resolve("rand").is_some());
    }

    #[test]
    fn run_returns_report_with_exit_code_and_empty_trace() {
        let _guard = GUEST_LOCK.lock().unwrap();
        let process =
            Process::load_at("eboot.elf", minimal_elf(0x20), |_| None, None, 0x820000000).unwrap();
        let mut emulator = Emulator::new(process);
        emulator.resolve_imports().unwrap();
        let report = emulator.run().unwrap();
        assert_eq!(report.version, EXECUTION_REPORT_VERSION);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.module_name, "eboot.elf");
        assert!(report.import_calls.is_empty());
    }

    #[test]
    fn from_elf_loads_and_runs() {
        let _guard = GUEST_LOCK.lock().unwrap();
        let mut emulator =
            Emulator::from_elf("eboot.elf", minimal_elf(0x20), |_| None, None).unwrap();
        emulator.resolve_imports().unwrap();
        let report = emulator.run().unwrap();
        assert_eq!(report.version, EXECUTION_REPORT_VERSION);
        assert_eq!(report.exit_code, 0);
        assert_eq!(report.module_name, "eboot.elf");
        assert!(report.import_calls.is_empty());
    }
}
