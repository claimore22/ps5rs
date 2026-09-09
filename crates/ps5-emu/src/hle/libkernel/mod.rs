//! HLE implementation for `libkernel` (stub – covers many `sceKernel*` calls).

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(KernelModule);
}

/// Stateless host module for guest `libkernel` calls.
pub struct KernelModule;

impl HleModule for KernelModule {
    fn name(&self) -> &str {
        "libkernel"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            ("sceKernelSleep", HostCall::KernelSleep),
            ("sceKernelUsleep", HostCall::KernelUsleep),
            ("sceKernelNanosleep", HostCall::KernelNanosleep),
            ("sceKernelUnlink", HostCall::KernelUnlink),
            ("sceKernelOpen", HostCall::KernelOpen),
            ("sceKernelClose", HostCall::KernelClose),
            ("sceKernelRead", HostCall::KernelRead),
            ("sceKernelWrite", HostCall::KernelWrite),
            ("sceKernelLseek", HostCall::KernelLseek),
            ("sceKernelStat", HostCall::KernelStat),
            ("sceKernelFstat", HostCall::KernelFstat),
            ("sceKernelMkdir", HostCall::KernelMkdir),
            ("sceKernelRmdir", HostCall::KernelRmdir),
            ("sceKernelGetProcessTime", HostCall::KernelGetProcessTime),
            (
                "sceKernelGetProcessTimeCounter",
                HostCall::KernelGetProcessTimeCounter,
            ),
            (
                "sceKernelGetProcessTimeCounterFrequency",
                HostCall::KernelGetProcessTimeCounterFrequency,
            ),
            ("sceKernelReadTsc", HostCall::KernelReadTsc),
            ("sceKernelGetTscFrequency", HostCall::KernelGetTscFrequency),
            ("sceKernelIsTrinityMode", HostCall::KernelIsTrinityMode),
            ("sceKernelGetCurrentCpu", HostCall::KernelGetCurrentCpu),
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
            HostCall::KernelSleep | HostCall::KernelNanosleep => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!("WARNING: sceKernelSleep stub called – not a real implementation");
                    tracing::warn!("sceKernelSleep stub called – not a real implementation");
                });
                Ok(0)
            }
            _ => {
                static WARN: std::sync::Once = std::sync::Once::new();
                WARN.call_once(|| {
                    eprintln!("WARNING: libkernel stub called – not a real implementation");
                    tracing::warn!("libkernel stub called – not a real implementation");
                });
                Ok(0)
            }
        }
    }
}
