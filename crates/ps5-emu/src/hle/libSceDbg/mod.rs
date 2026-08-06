//! Host implementation of the `libSceDbg` diagnostics API.

pub mod calls;

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(DbgModule);
}

/// Stateless host module for guest `libSceDbg` calls; the minimum severity
/// gate lives in [`HleContext`].
pub struct DbgModule;

impl HleModule for DbgModule {
    fn name(&self) -> &str {
        "libSceDbg"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            (
                "sceDbgSetMinimumLogLevel",
                HostCall::SceDbgSetMinimumLogLevel,
            ),
            ("sceDbgLoggingHandler", HostCall::SceDbgLoggingHandler),
        ]
    }

    fn call(
        &mut self,
        ctx: &mut HleContext,
        host: &mut dyn Host,
        call: HostCall,
        args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::SceDbgSetMinimumLogLevel => {
                calls::set_minimum_log_level(&mut ctx.libdbg, args)
            }
            HostCall::SceDbgLoggingHandler => calls::logging_handler(&ctx.libdbg, host, args),
            _ => Err(EmuError::NoHandler("libSceDbg".to_string())),
        }
    }
}
