//! HLE implementation for libSceAgc.
use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(AgcModule);
}

/// Stateless host module for guest `libSceAgc` calls.
pub struct AgcModule;

impl HleModule for AgcModule {
    fn name(&self) -> &str {
        "libSceAgc"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            (
                "sceAgcGetDefaultCxStateFlat",
                HostCall::SceAgcGetDefaultCxStateFlat,
            ),
            (
                "sceAgcGetRegisterDefaults2Internal",
                HostCall::SceAgcGetRegisterDefaults2Internal,
            ),
            ("sceAgcCbDispatchGetSize", HostCall::SceAgcCbDispatchGetSize),
            (
                "sceAgcSetShRegIndirectPatchSetAddress",
                HostCall::SceAgcSetShRegIndirectPatchSetAddress,
            ),
            ("sceAgcCbReleaseMem", HostCall::SceAgcCbReleaseMem),
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
            HostCall::SceAgcGetDefaultCxStateFlat
            | HostCall::SceAgcGetRegisterDefaults2Internal
            | HostCall::SceAgcCbDispatchGetSize
            | HostCall::SceAgcSetShRegIndirectPatchSetAddress
            | HostCall::SceAgcCbReleaseMem => Ok(0),
            _ => Err(EmuError::NoHandler("libSceAgc".to_string())),
        }
    }
}
