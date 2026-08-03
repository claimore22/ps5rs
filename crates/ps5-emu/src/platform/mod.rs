//! OS-level memory primitives for the emulator.

pub mod memory;

#[cfg(not(target_os = "windows"))]
pub mod unix;
#[cfg(target_os = "windows")]
pub mod windows;
