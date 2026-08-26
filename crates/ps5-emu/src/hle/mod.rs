//! Host-side implementation of guest system libraries (HLE).
//!
//! Layering: guest code → import dispatcher → typed [`HostCall`] → thin
//! per-library handlers → shared [`HleContext`] managers → host platform.
//! Dispatch never matches on symbol strings at runtime: the [`Registry`] maps
//! imported NIDs to [`HostCall`] identities at registration time, and each
//! library module keeps a small exhaustive match over the calls it owns.

// Library names keep their SDK spelling; the lints are intentional.
#[allow(non_snake_case)]
pub mod libSceDbg;
pub mod libc;
pub mod libkernel;
pub mod registry;

mod context;

pub use context::{DbgState, HleContext, LibcState};
pub use registry::Registry;

use ps5_abi::types::AbiType;

use crate::error::EmuError;

/// Stable identity of one guest host call, shared by every library module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostCall {
    InitEnv,
    NeedSceLibc,
    Atexit,
    Exit,
    CatchReturnFromMain,
    Printf,
    Puts,
    Rand,
    SceDbgSetMinimumLogLevel,
    SceDbgLoggingHandler,
    KernelSleep,
    KernelUsleep,
    KernelGetProcessTime,
    KernelGetProcessTimeCounter,
    KernelGetProcessTimeCounterFrequency,
    KernelReadTsc,
    KernelGetTscFrequency,
    KernelIsTrinityMode,
    KernelGetCurrentCpu,
}

/// Services a handler can use to interact with the guest's memory.
pub trait Host {
    /// Read `len` bytes from guest memory at `addr`.
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError>;
    /// Read a NUL-terminated string from guest memory at `addr`.
    fn read_string(&self, addr: u64) -> Result<String, EmuError>;
    /// Write `data` into guest memory at `addr`.
    fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError>;
    /// Forward a chunk of guest stdout to the host sink.  Defaults to a no-op;
    /// the emulator's guest memory captures these for the execution report.
    fn emit(&mut self, _chunk: &str) {}
}

/// A host-side implementation of one system library's exported functions.
///
/// Modules are stateless — any state a handler needs lives in the shared
/// [`HleContext`].  The guest ABI boundary lives in [`abi`](crate::abi); a
/// call arrives here as a [`HostCall`] plus an argument slice.
pub trait HleModule {
    /// Library identity, e.g. `"libSceDbg"`.
    fn name(&self) -> &str;
    /// Symbols this module implements, each bound to its [`HostCall`] identity.
    fn symbols(&self) -> &'static [(&'static str, HostCall)];
    /// Dispatch a guest call to this module's implementation.
    fn call(
        &mut self,
        ctx: &mut HleContext,
        host: &mut dyn Host,
        call: HostCall,
        args: &[u64],
    ) -> Result<u64, EmuError>;
}

/// The default HLE module set every emulator starts with.
pub fn default_registry() -> Registry {
    let _abi_check = AbiType::U64;
    let mut registry = Registry::new();
    libc::register(&mut registry);
    libkernel::register(&mut registry);
    libSceDbg::register(&mut registry);
    registry
}
