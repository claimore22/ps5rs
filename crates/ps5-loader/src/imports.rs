use std::collections::HashMap;

/// Describes an import symbol that needs address resolution.
///
/// Populated by [`apply_relocations_with`](crate::apply_relocations_with)
/// from the ELF symbol table. At minimum `name` is always set; `library` and
/// `nid` may be `None` if the loader does not have that information.
#[derive(Debug, Clone)]
pub struct ImportRequest {
    /// Index into the ELF symbol table.
    pub symbol_index: u32,
    /// Numeric NID (lower 64 bits of the SHA1+SALT hash). `None` if not computed.
    pub nid: Option<u64>,
    /// Library name (e.g. `"libkernel"`). `None` if unknown.
    pub library: Option<String>,
    /// Import name (e.g. `"sceKernelSleep"`). `None` if unknown.
    pub name: Option<String>,
}

/// The outcome of an import resolution.
///
/// Distinguishes between a real resolved address, a known-but-not-loaded
/// address (system function that exists in the SDK but wasn't linked), and
/// a complete stub (unknown NID).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveResult {
    /// Fully resolved to the real target address (HLE or linked library).
    Resolved(u64),
    /// Known system function, but the module is not loaded — stub address used.
    Known(u64),
    /// Allocated a stub / placeholder address for an unresolved import.
    Stubbed(u64),
}

impl ResolveResult {
    /// The address produced by resolution.
    pub fn address(&self) -> u64 {
        match self {
            ResolveResult::Resolved(addr)
            | ResolveResult::Known(addr)
            | ResolveResult::Stubbed(addr) => *addr,
        }
    }
}

/// Pluggable strategy for resolving import relocations.
///
/// # Implementing a custom resolver
///
/// A real resolver (e.g. backed by an HLE library catalog) would inspect
/// [`ImportRequest::nid`] or [`ImportRequest::name`] and return
/// [`ResolveResult::Resolved`] for known functions.
///
/// ```ignore
/// use ps5_loader::{ImportResolver, ImportRequest, ResolveResult, ImportError};
///
/// struct MyResolver;
///
/// impl ImportResolver for MyResolver {
///     fn resolve(&mut self, request: &ImportRequest) -> Result<ResolveResult, ImportError> {
///         // look up request.nid or request.name in a catalog
///         Err(ImportError("not implemented".into()))
///     }
/// }
/// ```
pub trait ImportResolver {
    fn resolve(&mut self, request: &ImportRequest) -> Result<ResolveResult, ImportError>;
}

/// Error returned when an import cannot be resolved.
#[derive(Debug)]
pub struct ImportError(pub String);

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ImportError {}

/// Default base address for the synthetic stub region.
///
/// Chosen to be clearly synthetic — at the very top of user-addressable space
/// (`0xFFFF_0000_0000_0000`) — so stub addresses are easy to recognise in
/// debugging and never collide with real module mappings.
pub const STUB_REGION_BASE: u64 = 0xFFFF_0000_0000_0000;

/// Stride (in bytes) between consecutive stub addresses.
const STUB_STRIDE: u64 = 16;

/// A default [`ImportResolver`] that assigns sequential stub addresses from a
/// dedicated synthetic region.
///
/// The same import (identified by `(library, name)`) always receives the same
/// stub address.  Different imports receive unique addresses.
///
/// # Example
///
/// ```ignore
/// use ps5_loader::{StubAllocator, ImportResolver, ImportRequest, STUB_REGION_BASE};
///
/// let mut stubber = StubAllocator::new(STUB_REGION_BASE);
/// let request = ImportRequest { symbol_index: 0, nid: None, library: None, name: Some("test".into()) };
/// let result = stubber.resolve(&request).unwrap();
/// assert_eq!(result.address(), STUB_REGION_BASE);
/// ```
#[derive(Debug)]
pub struct StubAllocator {
    base: u64,
    next_offset: u64,
    cache: HashMap<(Option<String>, Option<String>), u64>,
}

impl StubAllocator {
    pub fn new(base: u64) -> Self {
        Self {
            base,
            next_offset: 0,
            cache: HashMap::new(),
        }
    }
}

impl ImportResolver for StubAllocator {
    fn resolve(&mut self, request: &ImportRequest) -> Result<ResolveResult, ImportError> {
        let key = (request.library.clone(), request.name.clone());
        if let Some(&addr) = self.cache.get(&key) {
            return Ok(ResolveResult::Stubbed(addr));
        }
        let addr = self.base + self.next_offset;
        self.next_offset = self
            .next_offset
            .checked_add(STUB_STRIDE)
            .ok_or_else(|| ImportError("stub address overflow".into()))?;
        
        self.cache.insert(key, addr);

        Ok(ResolveResult::Stubbed(addr))
    }
}
