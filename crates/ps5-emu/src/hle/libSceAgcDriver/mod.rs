//! HLE implementation for `libSceAgcDriver` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(AgcDriverModule);
}

/// Stateless host module for guest `libSceAgcDriver` calls.
pub struct AgcDriverModule;

impl HleModule for AgcDriverModule {
    fn name(&self) -> &str {
        "libSceAgcDriver"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            (
                "sceAgcDriverQueryResourceRegistrationUserMemoryRequirements",
                HostCall::SceAgcDriverQueryResourceRegistrationUserMemoryRequirements,
            ),
            (
                "sceAgcDriverAgrSubmitDcb",
                HostCall::SceAgcDriverAgrSubmitDcb,
            ),
            ("sceAgcDriverUnmapIoctl", HostCall::SceAgcDriverUnmapIoctl),
            (
                "sceAgcDriverMapComputeQueue",
                HostCall::SceAgcDriverMapComputeQueue,
            ),
            ("sceAgcDriverInit", HostCall::SceAgcDriverInit),
            (
                "sceAgcDriverUnmapComputeQueue",
                HostCall::SceAgcDriverUnmapComputeQueue,
            ),
            ("sceAgcDriverSubmitDcb", HostCall::SceAgcDriverSubmitDcb),
            ("sceAgcDriverMapIoctl", HostCall::SceAgcDriverMapIoctl),
        ]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        _args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::SceAgcDriverQueryResourceRegistrationUserMemoryRequirements => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!("WARNING: sceAgcDriverQueryResourceRegistrationUserMemoryRequirements stub called – not a real implementation");
                    tracing::warn!(
                        "sceAgcDriverQueryResourceRegistrationUserMemoryRequirements stub called – not a real implementation"
                    );
                });
                Ok(0)
            }
            _ => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!("WARNING: libSceAgcDriver stub called – not a real implementation");
                    tracing::warn!("libSceAgcDriver stub called – not a real implementation");
                });
                Ok(0)
            }
            _ => Err(EmuError::NoHandler("libSceAgcDriver".to_string())),
        }
    }
}
