//! Host implementation of the `libc` subset the samples reach from crt0.

pub mod calls;
pub mod printf;
pub mod random;

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

const ATEXIT_LIMIT: usize = 32;

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(LibcModule);
}

/// Stateless host module for guest `libc` calls; state lives in [`HleContext`].
pub struct LibcModule;

impl HleModule for LibcModule {
    fn name(&self) -> &str {
        "libc"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            ("_init_env", HostCall::InitEnv),
            ("atexit", HostCall::Atexit),
            ("catchReturnFromMain", HostCall::CatchReturnFromMain),
            ("exit", HostCall::Exit),
            ("Need_sceLibc", HostCall::NeedSceLibc),
            ("printf", HostCall::Printf),
            ("puts", HostCall::Puts),
            ("rand", HostCall::Rand),
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
            HostCall::InitEnv | HostCall::NeedSceLibc => calls::init_env(),
            HostCall::Atexit => {
                if ctx.libc.atexit_handlers >= ATEXIT_LIMIT {
                    return Err(EmuError::NoHandler("atexit handler limit".to_string()));
                }
                ctx.libc.atexit_handlers += 1;
                Ok(0)
            }
            HostCall::Exit => calls::exit(args),
            HostCall::CatchReturnFromMain => calls::catch_return_from_main(args),
            HostCall::Puts => calls::puts(host, args),
            HostCall::Printf => calls::printf(host, args),
            HostCall::Rand => Ok(calls::rand(&mut ctx.libc)),
            _ => Err(EmuError::NoHandler("libc".to_string())),
        }
    }
}
