pub mod kernel;
pub mod libc;
pub mod libdbg;
mod output;

use std::collections::HashMap;

use ps5_loader::compute_nid;

use crate::error::EmuError;

/// Services a module handler can use to interact with the guest's memory.
pub trait Host {
    /// Read `len` bytes from guest memory at `addr`.
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError>;
    /// Read a NUL-terminated string from guest memory at `addr`.
    fn read_string(&self, addr: u64) -> Result<String, EmuError>;
    /// Write `data` into guest memory at `addr`.
    fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError>;
}

/// A host-side implementation of one system library's exported functions.
///
/// Modules are pure Rust — no `extern "sysv64"`, no exported function
/// pointers.  The guest ABI boundary lives in [`abi`](crate::abi); a call
/// arrives here as a symbol name plus an argument slice.
pub trait HleModule {
    /// Library identity, e.g. `"libSceDbg"`.
    fn name(&self) -> &str;
    /// Human-readable symbol names this module implements.
    fn symbols(&self) -> &'static [&'static str];
    /// Dispatch a guest call to this module's implementation.
    fn call(&mut self, host: &mut dyn Host, name: &str, args: &[u64]) -> Result<u64, EmuError>;
}

/// NID-indexed collection of registered [`HleModule`]s.
#[derive(Default)]
pub struct Registry {
    modules: Vec<Box<dyn HleModule>>,
    by_nid: HashMap<u64, usize>,
    names: HashMap<u64, &'static str>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module, indexing each of its symbols by computed NID.
    pub fn register(&mut self, module: impl HleModule + 'static) {
        let idx = self.modules.len();
        let symbols = module.symbols().to_vec();
        self.modules.push(Box::new(module));
        for sym in symbols {
            if let Some(nid) = compute_nid(sym) {
                self.by_nid.insert(nid, idx);
                self.names.insert(nid, sym);
            }
        }
    }

    /// The NID for a symbol if it is registered.
    pub fn resolve(&self, name: &str) -> Option<u64> {
        compute_nid(name).filter(|nid| self.by_nid.contains_key(nid))
    }

    /// Whether a handler is registered for `nid`.
    pub fn contains(&self, nid: u64) -> bool {
        self.by_nid.contains_key(&nid)
    }

    /// Number of indexed symbols.
    pub fn len(&self) -> usize {
        self.by_nid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_nid.is_empty()
    }

    /// Dispatch a guest call by NID.
    pub fn call(&mut self, host: &mut dyn Host, nid: u64, args: &[u64]) -> Result<u64, EmuError> {
        let idx = self
            .by_nid
            .get(&nid)
            .copied()
            .ok_or_else(|| EmuError::NoHandler(format!("nid {nid:#x}")))?;
        let name = self.names.get(&nid).copied().unwrap_or("?");
        tracing::debug!(import = name, args = args.len(), "registry: call");
        self.modules[idx].call(host, name, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Probe {
        calls: Vec<String>,
    }

    impl HleModule for Probe {
        fn name(&self) -> &str {
            "probe"
        }
        fn symbols(&self) -> &'static [&'static str] {
            &["probeFn"]
        }
        fn call(
            &mut self,
            _host: &mut dyn Host,
            name: &str,
            args: &[u64],
        ) -> Result<u64, EmuError> {
            self.calls.push(name.to_string());
            Ok(args.iter().sum())
        }
    }

    struct NoHost;

    impl Host for NoHost {
        fn read_bytes(&self, _addr: u64, _len: usize) -> Result<Vec<u8>, EmuError> {
            Ok(Vec::new())
        }
        fn read_string(&self, _addr: u64) -> Result<String, EmuError> {
            Ok(String::new())
        }
        fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<(), EmuError> {
            Ok(())
        }
    }

    #[test]
    fn registry_dispatch_by_computed_nid() {
        let mut registry = Registry::new();
        registry.register(Probe::default());
        let nid = registry.resolve("probeFn").expect("probeFn registered");
        let result = registry.call(&mut NoHost, nid, &[2, 3]).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn registry_missing_nid_errors() {
        let registry = Registry::new();
        assert!(!registry.contains(0));
    }

    #[test]
    fn registry_len_counts_symbols() {
        let mut registry = Registry::new();
        registry.register(Probe::default());
        assert_eq!(registry.len(), 1);
    }
}
