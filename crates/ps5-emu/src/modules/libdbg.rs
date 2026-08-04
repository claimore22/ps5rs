use crate::error::EmuError;
use crate::modules::{HleModule, Host};

use super::output::format_printf;

const SCE_OK: u64 = 0;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_TRACE: u64 = 0;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_DEBUG: u64 = 1;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_WARNING: u64 = 3;
#[allow(dead_code)]
const SCE_DBG_LOG_LEVEL_ERROR: u64 = 4;

/// Host implementation of the `libSceDbg` diagnostics API.
#[derive(Default)]
pub struct DbgModule {
    minimum_log_level: u64,
}

impl HleModule for DbgModule {
    fn name(&self) -> &str {
        "libSceDbg"
    }

    fn symbols(&self) -> &'static [&'static str] {
        &["sceDbgSetMinimumLogLevel", "sceDbgLoggingHandler"]
    }

    fn call(&mut self, host: &mut dyn Host, name: &str, args: &[u64]) -> Result<u64, EmuError> {
        match name {
            "sceDbgSetMinimumLogLevel" => {
                self.minimum_log_level = args.first().copied().unwrap_or(0);
                Ok(SCE_OK)
            }
            "sceDbgLoggingHandler" => self.logging_handler(host, args),
            _ => Err(EmuError::NoHandler(name.to_string())),
        }
    }
}

impl DbgModule {
    fn logging_handler(&self, host: &mut dyn Host, args: &[u64]) -> Result<u64, EmuError> {
        if args.len() < 5 {
            return Ok(SCE_OK);
        }
        let file = host.read_string(args[0]).unwrap_or_default();
        let line = args[1];
        let severity = args[2];
        let component = host.read_string(args[3]).unwrap_or_default();
        let format = host.read_string(args[4]).unwrap_or_default();

        if severity < self.minimum_log_level {
            return Ok(SCE_OK);
        }

        tracing::debug!(component = %component, file = %file, line, "log message");
        let message = format_printf(host, &format, &args[5..]);
        host.emit(&format!("[{file}:{line}] [{component}] {message}\n"));
        Ok(SCE_OK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut module = DbgModule::default();
        let mut host = MemoryHost;
        let result = module
            .call(
                &mut host,
                "sceDbgSetMinimumLogLevel",
                &[SCE_DBG_LOG_LEVEL_ERROR],
            )
            .unwrap();
        assert_eq!(result, SCE_OK);
        assert_eq!(module.minimum_log_level, SCE_DBG_LOG_LEVEL_ERROR);
    }

    #[test]
    fn logging_below_minimum_is_filtered() {
        let mut module = DbgModule {
            minimum_log_level: SCE_DBG_LOG_LEVEL_DEBUG,
        };
        let mut host = MemoryHost;
        let result = module
            .call(
                &mut host,
                "sceDbgLoggingHandler",
                &[0, 0, SCE_DBG_LOG_LEVEL_TRACE, 0, 0, 0],
            )
            .unwrap();
        assert_eq!(result, SCE_OK);
    }

    #[test]
    fn unknown_symbol_errors() {
        let mut module = DbgModule::default();
        let mut host = MemoryHost;
        let err = module.call(&mut host, "sceDbgBogus", &[]).unwrap_err();
        assert!(err.to_string().contains("sceDbgBogus"));
    }
}
