//! Thin handlers for guest `libSceDbg` calls; the severity gate lives in
//! [`DbgState`].  No state lives here.

use crate::error::EmuError;
use crate::hle::libc::printf::format_printf;
use crate::hle::{DbgState, Host};

const SCE_OK: u64 = 0;

// Numeric severity ordering; only the tests reference the names directly.
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_TRACE: u64 = 0;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_DEBUG: u64 = 1;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_ERROR: u64 = 4;

/// `sceDbgSetMinimumLogLevel(level) -> int`: raise the severity gate.
pub fn set_minimum_log_level(state: &mut DbgState, args: &[u64]) -> Result<u64, EmuError> {
    state.minimum_log_level = args.first().copied().unwrap_or(0);
    Ok(SCE_OK)
}

/// `sceDbgLoggingHandler(file, line, severity, component, format, ...)`:
/// format and emit the message when it clears the severity gate.
pub fn logging_handler(
    state: &DbgState,
    host: &mut dyn Host,
    args: &[u64],
) -> Result<u64, EmuError> {
    if args.len() < 5 {
        return Ok(SCE_OK);
    }
    let file = host.read_string(args[0]).unwrap_or_default();
    let line = args[1];
    let severity = args[2];
    let component = host.read_string(args[3]).unwrap_or_default();
    let format = host.read_string(args[4]).unwrap_or_default();

    if severity < state.minimum_log_level {
        return Ok(SCE_OK);
    }

    tracing::debug!(component = %component, file = %file, line, "log message");
    let message = format_printf(host, &format, &args[5..]);
    host.emit(&format!("[{file}:{line}] [{component}] {message}\n"));
    Ok(SCE_OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hle::{HleContext, HleModule};

    struct MemoryHost;

    impl Host for MemoryHost {
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
    fn set_minimum_log_level_returns_ok() {
        let mut ctx = HleContext::default();
        let result = set_minimum_log_level(&mut ctx.libdbg, &[SCE_DBG_LOG_LEVEL_ERROR]).unwrap();
        assert_eq!(result, SCE_OK);
        assert_eq!(ctx.libdbg.minimum_log_level, SCE_DBG_LOG_LEVEL_ERROR);
    }

    #[test]
    fn logging_below_minimum_is_filtered() {
        let state = DbgState {
            minimum_log_level: SCE_DBG_LOG_LEVEL_DEBUG,
        };
        let mut host = MemoryHost;
        let result =
            logging_handler(&state, &mut host, &[0, 0, SCE_DBG_LOG_LEVEL_TRACE, 0, 0, 0]).unwrap();
        assert_eq!(result, SCE_OK);
    }

    #[test]
    fn unknown_call_errors() {
        let mut module = super::super::DbgModule;
        let mut ctx = HleContext::default();
        let mut host = MemoryHost;
        let err = module
            .call(&mut ctx, &mut host, crate::hle::HostCall::KernelSleep, &[])
            .unwrap_err();
        assert!(err.to_string().contains("libSceDbg"));
    }
}
