//! Host implementation of the `libkernel` subset currently exercised.

pub mod calls;

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(KernelModule);
}

/// Stateless host module for guest `libkernel` calls.
pub struct KernelModule;

impl HleModule for KernelModule {
    fn name(&self) -> &str {
        "libkernel"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[("sceKernelSleep", HostCall::KernelSleep)]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::KernelSleep => calls::sleep(args),
            _ => Err(EmuError::NoHandler("libkernel".to_string())),
        }
    }
}
