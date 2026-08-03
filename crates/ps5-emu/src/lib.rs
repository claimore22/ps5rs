//! PS5 host-side emulator — native guest execution with HLE system library modules.
//!
//! Loads a PS5 eboot (+ PRX dependencies) with the existing `ps5-loader`
//! pipeline, then drives execution on the host CPU.  System-library imports
//! (`libSceDbg`, `libc`, `libkernel`) are handled by idiomatic Rust modules
//! registered in a [`Registry`], so the only ABI-aware code lives in
//! [`abi`].

pub mod abi;
pub mod core;
pub mod emulator;
pub mod error;
pub mod imports;
pub mod modules;
pub mod nid;
pub mod platform;
pub mod process;
pub mod trace;

pub use emulator::{EmuState, Emulator};
pub use error::EmuError;
pub use imports::{ImportBinding, ImportTable};
pub use modules::{HleModule, Host, Registry};
pub use process::{ModuleBytes, Process};
pub use trace::{EXECUTION_REPORT_VERSION, ExecutionReport, ImportCall};
