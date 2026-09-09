//! HLE implementation for `libSceAudioOut` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(AudioOutModule);
}

/// Stateless host module for guest `libSceAudioOut` calls.
pub struct AudioOutModule;

impl HleModule for AudioOutModule {
    fn name(&self) -> &str {
        "libSceAudioOut"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[("sceAudioOutOutput", HostCall::SceAudioOutOutput)]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        _args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            // HostCall::SceAudioOutOutput => {
            //     static WARN: std::sync::Once = std::sync::Once::new();
            //     WARN.call_once(|| {
            //         eprintln!("WARNING: sceAudioOutOutput stub called – not a real implementation");
            //         tracing::warn!("sceAudioOutOutput stub called – not a real implementation");
            //     });
            //     Ok(0)
            // }
            _ => Err(EmuError::NoHandler("libSceAudioOut".to_string())),
        }
    }
}
