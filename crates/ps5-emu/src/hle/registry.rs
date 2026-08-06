//! NID-indexed collection of registered [`HleModule`]s.

use std::collections::HashMap;

use ps5_loader::compute_nid;

use crate::error::EmuError;

use super::{HleContext, HleModule, Host, HostCall};

/// NID-indexed collection of registered [`HleModule`]s.
#[derive(Default)]
pub struct Registry {
    modules: Vec<Box<dyn HleModule>>,
    by_nid: HashMap<u64, (usize, HostCall)>,
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
        for (sym, call) in symbols {
            if let Some(nid) = compute_nid(sym) {
                self.by_nid.insert(nid, (idx, call));
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
    pub fn call(
        &mut self,
        ctx: &mut HleContext,
        host: &mut dyn Host,
        nid: u64,
        args: &[u64],
    ) -> Result<u64, EmuError> {
        let (idx, call) = self
            .by_nid
            .get(&nid)
            .copied()
            .ok_or_else(|| EmuError::NoHandler(format!("nid {nid:#x}")))?;
        let name = self.names.get(&nid).copied().unwrap_or("?");
        tracing::debug!(import = name, args = args.len(), "registry: call");
        self.modules[idx]
            .call(ctx, host, call, args)
            .map_err(|err| {
                if matches!(err, EmuError::NoHandler(_)) {
                    EmuError::NoHandler(format!("{}::{name}", self.modules[idx].name()))
                } else {
                    err
                }
            })
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
        fn symbols(&self) -> &'static [(&'static str, HostCall)] {
            &[("probeFn", HostCall::Rand)]
        }
        fn call(
            &mut self,
            _ctx: &mut HleContext,
            _host: &mut dyn Host,
            call: HostCall,
            args: &[u64],
        ) -> Result<u64, EmuError> {
            self.calls.push(format!("{call:?}"));
            Ok(match call {
                HostCall::Rand => args.iter().sum(),
                _ => 0,
            })
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
        let mut ctx = HleContext::default();
        let mut host = NoHost;
        let nid = registry.resolve("probeFn").expect("probeFn registered");
        let result = registry.call(&mut ctx, &mut host, nid, &[2, 3]).unwrap();
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
