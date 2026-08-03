use crate::error::EmuError;
use crate::modules::{HleModule, Host};

use super::output::format_printf;

const ATEXIT_LIMIT: usize = 32;

/// Host implementation of the subset of `libc` the samples reach from crt0.
#[derive(Default)]
pub struct LibcModule {
    atexit_handlers: usize,
}

impl HleModule for LibcModule {
    fn name(&self) -> &str {
        "libc"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &[
            "_init_env",
            "atexit",
            "catchReturnFromMain",
            "exit",
            "Need_sceLibc",
            "printf",
            "puts",
            "rand",
        ]
    }

    fn call(&mut self, host: &mut dyn Host, name: &str, args: &[u64]) -> Result<u64, EmuError> {
        match name {
            "_init_env" | "Need_sceLibc" => Ok(0),
            "atexit" => {
                if self.atexit_handlers >= ATEXIT_LIMIT {
                    return Err(EmuError::NoHandler("atexit handler limit".to_string()));
                }
                self.atexit_handlers += 1;
                Ok(0)
            }
            "exit" => {
                let code = args.first().copied().unwrap_or(0);
                tracing::info!(code, "guest exit");
                Err(EmuError::GuestExit(code))
            }
            "puts" => {
                let s =
                    host.read_string(args.first().copied().ok_or_else(|| {
                        EmuError::NoHandler("puts missing argument".to_string())
                    })?)?;
                tracing::debug!(message = %s, "puts");
                println!("{s}");
                Ok(0)
            }
            "printf" => {
                let format =
                    host.read_string(args.first().copied().ok_or_else(|| {
                        EmuError::NoHandler("printf missing format".to_string())
                    })?)?;
                let message = format_printf(host, &format, &args[1..]);
                tracing::debug!(message = %message, "printf");
                print!("{message}");
                Ok(message.chars().count() as u64)
            }
            "rand" => {
                let value = rand_u64();
                tracing::debug!(value, "rand");
                Ok(value)
            }
            "catchReturnFromMain" => {
                let code = args.first().copied().unwrap_or(0);
                tracing::info!(code, "guest exited via catchReturnFromMain");
                Err(EmuError::GuestExit(code))
            }
            _ => Err(EmuError::NoHandler(name.to_string())),
        }
    }
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0x1234_5678)
        | 1;
    let mut state = seed;
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubHost;

    impl Host for StubHost {
        fn read_bytes(&self, _addr: u64, _len: usize) -> Result<Vec<u8>, EmuError> {
            Ok(Vec::new())
        }
        fn read_string(&self, _addr: u64) -> Result<String, EmuError> {
            Ok("hello".to_string())
        }
        fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<(), EmuError> {
            Ok(())
        }
    }

    #[test]
    fn init_env_returns_ok() {
        let mut module = LibcModule::default();
        let mut host = StubHost;
        assert_eq!(module.call(&mut host, "_init_env", &[]).unwrap(), 0);
    }

    #[test]
    fn exit_surfaces_guest_exit() {
        let mut module = LibcModule::default();
        let mut host = StubHost;
        let err = module.call(&mut host, "exit", &[7]).unwrap_err();
        assert!(matches!(err, EmuError::GuestExit(7)));
    }

    #[test]
    fn atexit_tracks_handlers() {
        let mut module = LibcModule::default();
        let mut host = StubHost;
        assert_eq!(module.call(&mut host, "atexit", &[0x4000]).unwrap(), 0);
        assert_eq!(module.atexit_handlers, 1);
    }

    #[test]
    fn rand_produces_variety() {
        let mut module = LibcModule::default();
        let mut host = StubHost;
        let a = module.call(&mut host, "rand", &[]).unwrap();
        let b = module.call(&mut host, "rand", &[]).unwrap();
        assert_ne!(a, b);
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
        let mut module = LibcModule::default();
        let mut host = PrintfHost;
        let result = module
            .call(&mut host, "printf", &[0x100, 2, 0x200, 0x1A])
            .unwrap();
        assert_eq!(result, 16);
    }

    #[test]
    fn printf_missing_format_errors() {
        let mut module = LibcModule::default();
        let mut host = PrintfHost;
        let err = module.call(&mut host, "printf", &[]).unwrap_err();
        assert!(err.to_string().contains("printf"));
    }
}
