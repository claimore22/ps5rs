//! HLE implementation for `libSceNpUniversalDataSystem` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(NpUniversalDataSystemModule);
}

/// Stateless host module for guest `libSceNpUniversalDataSystem` calls.
pub struct NpUniversalDataSystemModule;

impl HleModule for NpUniversalDataSystemModule {
    fn name(&self) -> &str {
        "libSceNpUniversalDataSystem"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[(
            "sceNpUniversalDataSystemDestroyEvent",
            HostCall::SceNpUniversalDataSystemDestroyEvent,
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
            // HostCall::SceNpUniversalDataSystemDestroyEvent => {
            //     static WARN: std::sync::Once = std::sync::Once::new();
            //     WARN.call_once(|| {
            //         eprintln!("WARNING: sceNpUniversalDataSystemDestroyEvent stub called – not a real implementation");
            //         tracing::warn!(
            //             "sceNpUniversalDataSystemDestroyEvent stub called – not a real implementation"
            //         );
            //     });
            //     Ok(0)
            // }
            _ => Err(EmuError::NoHandler(
                "libSceNpUniversalDataSystem".to_string(),
            )),
        }
    }
}
