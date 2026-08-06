//! Thin handlers for guest `libkernel` calls: translate the guest ABI into
//! host operations on the shared [`HleContext`](crate::hle::HleContext).  No
//! state lives here.

use crate::error::EmuError;

const SCE_OK: u64 = 0;

/// `sceKernelSleep(seconds) -> int`: the deterministic clock blocks for the
/// requested span, which the frozen clock model reports as instantaneous.
pub fn sleep(args: &[u64]) -> Result<u64, EmuError> {
    let seconds = args.first().copied().unwrap_or(0);
    tracing::debug!(seconds, "sceKernelSleep");
    Ok(SCE_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_returns_ok() {
        assert_eq!(sleep(&[2]).unwrap(), SCE_OK);
    }

    #[test]
    fn sleep_without_args_returns_ok() {
        assert_eq!(sleep(&[]).unwrap(), SCE_OK);
    }
}
