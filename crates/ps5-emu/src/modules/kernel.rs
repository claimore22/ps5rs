use crate::error::EmuError;
use crate::modules::{HleModule, Host};

/// Host placeholder for `libkernel` — symbols are added as execution needs them.
#[derive(Default)]
pub struct KernelModule;

impl HleModule for KernelModule {
    fn name(&self) -> &str {
        "libkernel"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &[]
    }

    fn call(&mut self, _host: &mut dyn Host, _name: &str, _args: &[u64]) -> Result<u64, EmuError> {
        Err(EmuError::NoHandler(
            "libkernel has no handlers yet".to_string(),
        ))
    }
}
