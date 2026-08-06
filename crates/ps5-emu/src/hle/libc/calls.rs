//! Thin handlers for guest `libc` calls: translate the guest ABI into host
//! operations on the shared [`HleContext`](crate::hle::HleContext).  No state
//! lives here.

use crate::error::EmuError;
use crate::hle::{Host, LibcState};

use super::printf::format_printf;
use super::random::next_rand;

/// `_init_env(module, flags, debug, arg) -> int`: accept crt0 setup.
pub fn init_env() -> Result<u64, EmuError> {
    Ok(0)
}

/// `exit(code)`: unwind to the host caller with `code`.
pub fn exit(args: &[u64]) -> Result<u64, EmuError> {
    let code = args.first().copied().unwrap_or(0);
    tracing::info!(code, "guest exit");
    Err(EmuError::GuestExit(code))
}

/// `catchReturnFromMain(code)`: crt0 reports `main`'s result.
pub fn catch_return_from_main(args: &[u64]) -> Result<u64, EmuError> {
    let code = args.first().copied().unwrap_or(0);
    tracing::info!(code, "guest exited via catchReturnFromMain");
    Err(EmuError::GuestExit(code))
}

/// `puts(s) -> int`: emit `s` plus a newline to guest stdout.
pub fn puts(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let s = host.read_string(
        args.first()
            .copied()
            .ok_or_else(|| EmuError::NoHandler("puts missing argument".to_string()))?,
    )?;
    tracing::debug!(message = %s, "puts");
    let mut line = s;
    line.push('\n');
    host.emit(&line);
    Ok(0)
}

/// `printf(format, ...) -> int`: emit a format-substituted string.
pub fn printf(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let format = host.read_string(
        args.first()
            .copied()
            .ok_or_else(|| EmuError::NoHandler("printf missing format".to_string()))?,
    )?;
    let message = format_printf(host, &format, &args[1..]);
    tracing::debug!(message = %message, "printf");
    host.emit(&message);
    Ok(message.chars().count() as u64)
}

/// `rand() -> int`: next value of the deterministic guest PRNG.
pub fn rand(state: &mut LibcState) -> u64 {
    let value = next_rand(&mut state.rand_state);
    tracing::debug!(value, "rand");
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hle::context::RAND_SEED;

    #[test]
    fn init_env_returns_ok() {
        assert_eq!(init_env().unwrap(), 0);
    }

    #[test]
    fn exit_surfaces_guest_exit() {
        let err = exit(&[7]).unwrap_err();
        assert!(matches!(err, EmuError::GuestExit(7)));
    }

    #[test]
    fn rand_is_deterministic() {
        let mut a = LibcState::default();
        let mut b = LibcState::default();
        let seq_a: Vec<u64> = (0..8).map(|_| rand(&mut a)).collect();
        let seq_b: Vec<u64> = (0..8).map(|_| rand(&mut b)).collect();
        assert_eq!(seq_a, seq_b);
        assert!(seq_a.iter().all(|v| *v <= u32::MAX as u64));
        assert_eq!(a.rand_state, b.rand_state);
    }

    #[test]
    fn rand_state_seeded_from_constant() {
        let state = LibcState::default();
        assert_eq!(state.rand_state, RAND_SEED);
    }

    #[test]
    fn puts_and_printf_emit_through_host() {
        struct EmittingHost {
            output: Vec<String>,
        }
        impl Host for EmittingHost {
            fn read_bytes(&self, _addr: u64, _len: usize) -> Result<Vec<u8>, EmuError> {
                Ok(Vec::new())
            }
            fn read_string(&self, addr: u64) -> Result<String, EmuError> {
                match addr {
                    0x100 => Ok("hi".to_string()),
                    0x200 => Ok("n=%d".to_string()),
                    _ => Err(EmuError::Unmapped(addr)),
                }
            }
            fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<(), EmuError> {
                Ok(())
            }
            fn emit(&mut self, chunk: &str) {
                self.output.push(chunk.to_string());
            }
        }
        let mut host = EmittingHost { output: Vec::new() };
        puts(&mut host, &[0x100]).unwrap();
        printf(&mut host, &[0x200, 7]).unwrap();
        assert_eq!(host.output, vec!["hi\n".to_string(), "n=7".to_string()]);
    }

    struct PrintfHost;

    impl Host for PrintfHost {
        fn read_bytes(&self, _addr: u64, _len: usize) -> Result<Vec<u8>, EmuError> {
            Ok(Vec::new())
        }
        fn read_string(&self, addr: u64) -> Result<String, EmuError> {
            match addr {
                0x100 => Ok("n=%d s=%s x=%x".to_string()),
                0x200 => Ok("world".to_string()),
                _ => Err(EmuError::Unmapped(addr)),
            }
        }
        fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<(), EmuError> {
            Ok(())
        }
    }

    #[test]
    fn printf_formats_variadic_args() {
        let mut host = PrintfHost;
        let result = printf(&mut host, &[0x100, 2, 0x200, 0x1A]).unwrap();
        assert_eq!(result, 16);
    }

    #[test]
    fn printf_missing_format_errors() {
        let mut host = PrintfHost;
        let err = printf(&mut host, &[]).unwrap_err();
        assert!(err.to_string().contains("printf"));
    }
}
