//! Shared state every HLE handler reaches through the [`HleModule`] boundary.
//!
//! Handlers stay thin and stateless; the managers below own the whole HLE
//! process.  Manager structs (threads, memory, files, sync, modules) join this
//! context as their subsystems land.

/// Fixed seed so guest `rand` sequences are reproducible across runs and
/// machines (matches the approved deterministic-fixture plan).
pub const RAND_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Guest-visible `libc` state: crt0 bookkeeping and the deterministic PRNG.
#[derive(Debug)]
pub struct LibcState {
    pub atexit_handlers: usize,
    pub rand_state: u64,
}

impl Default for LibcState {
    fn default() -> Self {
        Self {
            atexit_handlers: 0,
            rand_state: RAND_SEED,
        }
    }
}

/// Guest-visible `libSceDbg` state: the minimum severity gate.
#[derive(Debug, Default)]
pub struct DbgState {
    pub minimum_log_level: u64,
}

/// Shared mutable state for the duration of one guest run.  Every library
/// module receives a mutable reference alongside the [`Host`](crate::hle::Host).
#[derive(Debug, Default)]
pub struct HleContext {
    pub libc: LibcState,
    pub libdbg: DbgState,
}
