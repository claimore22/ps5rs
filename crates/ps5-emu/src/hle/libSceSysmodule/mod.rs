//! HLE implementation for `libSceSysmodule` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(SysmoduleModule);
}

/// Stateless host module for `libSceSysmodule` calls.
pub struct SysmoduleModule;

impl HleModule for SysmoduleModule {
    fn name(&self) -> &str {
        "libSceSysmodule"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[("sceSysmoduleLoadModule", HostCall::SceSysmoduleLoadModule)]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        _args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::SceSysmoduleLoadModule => Ok(0),
            _ => Err(EmuError::NoHandler("libSceSysmodule".to_string())),
        }
    }
}
