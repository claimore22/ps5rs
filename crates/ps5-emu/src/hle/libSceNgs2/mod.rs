//! HLE implementation for `libSceNgs2` (stub).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(Ngs2Module);
}

/// Stateless host module for guest `libSceNgs2` calls.
pub struct Ngs2Module;

impl HleModule for Ngs2Module {
    fn name(&self) -> &str {
        "libSceNgs2"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[(
            "sceNgs2VoiceRunCommands",
            HostCall::SceNgs2VoiceRunCommands,
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
            HostCall::SceNgs2VoiceRunCommands => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!("WARNING: sceNgs2VoiceRunCommands stub called – not a real implementation");
                    tracing::warn!("sceNgs2VoiceRunCommands stub called – not a real implementation");
                });
                Ok(0)
            }
            _ => Err(EmuError::NoHandler("libSceNgs2".to_string())),
        }
    }
}
