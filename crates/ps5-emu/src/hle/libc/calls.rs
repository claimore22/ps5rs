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

use std::sync::Once;

/// Stub implementation for `qsort`. It does **not** perform a real sort –
/// it simply returns 0 (success) so execution can continue.
pub fn qsort(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    static WARN: std::sync::Once = std::sync::Once::new();
    WARN.call_once(|| {
        eprintln!("WARNING: qsort stub called – not a real implementation");
        tracing::warn!("qsort stub called – not a real implementation");
    });
    Ok(0)
}

/// Stub implementation for `ferror`. It does **not** check the stream error
/// indicator; it just returns 0 (no error) so execution can continue.
pub fn ferror(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    static WARN: std::sync::Once = std::sync::Once::new();
    WARN.call_once(|| {
        eprintln!("WARNING: ferror stub called – not a real implementation");
        tracing::warn!("ferror stub called – not a real implementation");
    });
    Ok(0)
}

/// Stub implementation for `time`. It does **not** read the real clock;
/// it just returns 0 (epoch) so execution can continue.
pub fn time(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    static WARN: std::sync::Once = std::sync::Once::new();
    WARN.call_once(|| {
        eprintln!("WARNING: time stub called – not a real implementation");
        tracing::warn!("time stub called – not a real implementation");
    });
    Ok(0)
}

/// Stub implementation for `setjmp`. It does **not** save the call
/// environment; it just returns 0 so execution can continue.
pub fn setjmp(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    static WARN: std::sync::Once = std::sync::Once::new();
    WARN.call_once(|| {
        eprintln!("WARNING: setjmp stub called – not a real implementation");
        tracing::warn!("setjmp stub called – not a real implementation");
    });
    Ok(0)
}

/// Memory allocation stub. Allocates a buffer via the host and returns its address.
pub fn malloc(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let size = args.first().copied().unwrap_or(0) as usize;
    if size == 0 {
        return Ok(0);
    }
    let mut buf = vec![0u8; size];
    let ptr = buf.as_mut_ptr() as u64;
    std::mem::forget(buf);
    static WARN: std::sync::Once = std::sync::Once::new();
    WARN.call_once(|| {
        tracing::debug!("malloc stub: returning host heap pointer");
    });
    Ok(ptr)
}

/// Free stub – no-op (host memory is leaked for now).
pub fn free(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// calloc stub.
pub fn calloc(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let nmemb = args.first().copied().unwrap_or(0) as usize;
    let size = args.get(1).copied().unwrap_or(0) as usize;
    let total = nmemb.saturating_mul(size);
    malloc(host, &[total as u64])
}

/// realloc stub – returns a fresh allocation (old pointer is leaked).
pub fn realloc(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let size = args.get(1).copied().unwrap_or(0);
    malloc(host, &[size])
}

/// memset stub.
pub fn memset(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let dst = args.first().copied().unwrap_or(0);
    let value = args.get(1).copied().unwrap_or(0) as u8;
    let len = args.get(2).copied().unwrap_or(0) as usize;
    let data = vec![value; len];
    host.write(dst, &data)?;
    Ok(dst)
}

/// memcpy stub.
pub fn memcpy(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let dst = args.first().copied().unwrap_or(0);
    let src = args.get(1).copied().unwrap_or(0);
    let len = args.get(2).copied().unwrap_or(0) as usize;
    let data = host.read_bytes(src, len)?;
    host.write(dst, &data)?;
    Ok(dst)
}

/// memmove stub (same as memcpy for now).
pub fn memmove(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    memcpy(host, args)
}

/// memcmp stub – returns 0 (equal) always.
pub fn memcmp(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// strlen stub.
pub fn strlen(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let s = host.read_string(args.first().copied().unwrap_or(0))?;
    Ok(s.len() as u64)
}

/// strcpy stub.
pub fn strcpy(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let dst = args.first().copied().unwrap_or(0);
    let src = args.get(1).copied().unwrap_or(0);
    let s = host.read_string(src)?;
    let bytes = s.as_bytes();
    let mut buf = Vec::with_capacity(bytes.len() + 1);
    buf.extend_from_slice(bytes);
    buf.push(0);
    host.write(dst, &buf)?;
    Ok(dst)
}

/// strncpy stub.
pub fn strncpy(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let dst = args.first().copied().unwrap_or(0);
    let src = args.get(1).copied().unwrap_or(0);
    let n = args.get(2).copied().unwrap_or(0) as usize;
    let s = host.read_string(src)?;
    let bytes = s.as_bytes();
    let mut buf = vec![0u8; n];
    for (i, b) in bytes.iter().take(n).enumerate() {
        buf[i] = *b;
    }
    host.write(dst, &buf)?;
    Ok(dst)
}

/// strcmp stub – always returns 0.
pub fn strcmp(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// strncmp stub – always returns 0.
pub fn strncmp(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// strchr stub – returns the original pointer.
pub fn strchr(_host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    Ok(args.first().copied().unwrap_or(0))
}

/// strrchr stub – returns the original pointer.
pub fn strrchr(_host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    Ok(args.first().copied().unwrap_or(0))
}

/// strstr stub – returns the haystack pointer.
pub fn strstr(_host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    Ok(args.first().copied().unwrap_or(0))
}

/// strcat stub.
pub fn strcat(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let dst = args.first().copied().unwrap_or(0);
    let src = args.get(1).copied().unwrap_or(0);
    let existing = host.read_string(dst).unwrap_or_default();
    let append = host.read_string(src).unwrap_or_default();
    let mut combined = existing.as_bytes().to_vec();
    combined.extend_from_slice(append.as_bytes());
    combined.push(0);
    host.write(dst, &combined)?;
    Ok(dst)
}

/// strncat stub.
pub fn strncat(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    strcat(host, args)
}

/// memchr stub – returns the original pointer.
pub fn memchr(_host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    Ok(args.first().copied().unwrap_or(0))
}

/// fopen stub – returns 0 (null FILE*).
pub fn fopen(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fclose stub – returns 0.
pub fn fclose(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fread stub – returns 0.
pub fn fread(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fwrite stub – returns 0.
pub fn fwrite(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fseek stub – returns 0.
pub fn fseek(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// ftell stub – returns 0.
pub fn ftell(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fflush stub – returns 0.
pub fn fflush(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// fgetc stub – returns EOF (-1).
pub fn fgetc(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(u64::MAX)
}

/// fputc stub – returns 0.
pub fn fputc(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(0)
}

/// getc stub – returns EOF.
pub fn getc(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
    Ok(u64::MAX)
}

/// putc stub – returns 0.
pub fn putc(_host: &mut dyn Host, _args: &[u64]) -> Result<u64, EmuError> {
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

/// `fprintf(stream, format, ...) -> int`: emit formatted text to the host.
pub fn fprintf(host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
    let format = host.read_string(
        args.get(1)
            .copied()
            .ok_or_else(|| EmuError::NoHandler("fprintf missing format".to_string()))?,
    )?;
    let message = format_printf(host, &format, &args[2..]);
    tracing::debug!(message = %message, "fprintf");
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

    struct PrintfHost {
        output: Vec<String>,
    }

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

        fn emit(&mut self, chunk: &str) {
            self.output.push(chunk.to_string());
        }
    }

    #[test]
    fn printf_formats_variadic_args() {
        let mut host = PrintfHost { output: Vec::new() };
        let result = printf(&mut host, &[0x100, 2, 0x200, 0x1A]).unwrap();
        assert_eq!(result, 16);
    }

    #[test]
    fn fprintf_skips_stream_and_emits_formatted_text() {
        let mut host = PrintfHost { output: Vec::new() };
        let result = fprintf(&mut host, &[0xDEAD, 0x100, 7, 0x200, 0xDEAD]).unwrap();
        assert_eq!(result, 18);
        assert_eq!(host.output, vec!["n=7 s=world x=dead".to_string()]);
    }

    #[test]
    fn printf_missing_format_errors() {
        let mut host = PrintfHost { output: Vec::new() };
        let err = printf(&mut host, &[]).unwrap_err();
        assert!(err.to_string().contains("printf"));
    }
}
