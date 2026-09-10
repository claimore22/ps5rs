//! Generic fallback HLE implementation for imports without a dedicated module.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall};

static WARNED_STUBS: OnceLock<Mutex<HashSet<(String, u64)>>> = OnceLock::new();

pub(crate) fn warn_once(library: &str, nid: u64, name: &str) {
    let warned = WARNED_STUBS.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut warned) = warned.lock() else {
        return;
    };
    
    if warned.insert((library.to_string(), nid)) {
        let nid_b64 = u64_to_nid_str(nid);
        tracing::warn!(
            library,
            name,
            nid = format_args!("{:#x}", nid),
            nid_b64 = %nid_b64,
            "HLE stub called"
        );
    }
}

macro_rules! define_hle_stub_module {
    ($module:ident, $library:literal, [$($symbol:literal),* $(,)?]) => {
        pub struct $module;

        pub fn register(registry: &mut $crate::hle::Registry) {
            registry.register($module);
        }

        impl $crate::hle::HleModule for $module {
            fn name(&self) -> &str {
                $library
            }

            fn symbols(&self) -> &'static [(&'static str, $crate::hle::HostCall)] {
                &[$(($symbol, $crate::hle::HostCall::GenericStub)),*]
            }

            fn call(
                &mut self,
                _ctx: &mut $crate::hle::HleContext,
                _host: &mut dyn $crate::hle::Host,
                call: $crate::hle::HostCall,
                _args: &[u64],
            ) -> Result<u64, $crate::error::EmuError> {
                match call {
                    $crate::hle::HostCall::GenericStub => Ok(0),
                    _ => Err($crate::error::EmuError::NoHandler($library.to_string())),
                }
            }
        }
    };
}

fn u64_to_nid_str(nid: u64) -> String {
    ps5_nid::encode_nid(nid.to_be_bytes())
}

pub(crate) use define_hle_stub_module;

pub struct StubModule;

impl HleModule for StubModule {
    fn name(&self) -> &str {
        "generic HLE stub"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[]
    }

    fn call(
        &mut self,
        _ctx: &mut HleContext,
        _host: &mut dyn Host,
        call: HostCall,
        _args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::GenericStub => Ok(0),
            _ => Err(EmuError::NoHandler("generic HLE stub".to_string())),
        }
    }
}
