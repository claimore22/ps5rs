//! HLE implementation for `libSceSaveData_native` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(SaveDataNativeModule);
}

/// Stateless host module for guest `libSceSaveData_native` calls.
pub struct SaveDataNativeModule;

impl HleModule for SaveDataNativeModule {
    fn name(&self) -> &str {
        "libSceSaveData_native"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[(
            "sceSaveDataCreateTransactionResource",
            HostCall::SceSaveDataCreateTransactionResource,
        )]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        _args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::SceSaveDataCreateTransactionResource => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!(
                        "WARNING: sceSaveDataCreateTransactionResource stub called - not a real implementation"
                    );
                    tracing::warn!(
                        "sceSaveDataCreateTransactionResource stub called - not a real implementation"
                    );
                });
                Ok(0)
            }
            _ => Err(EmuError::NoHandler("libSceSaveData_native".to_string())),
        }
    }
}
