//! Host implementation of the `libc` subset the samples reach from crt0.

pub mod calls;
pub mod printf;
pub mod random;

use crate::error::EmuError;
use crate::hle::{HleContext, HleModule, Host, HostCall, Registry};

const ATEXIT_LIMIT: usize = 32;

/// Register this library's host calls.
pub fn register(registry: &mut Registry) {
    registry.register(LibcModule);
}

/// Stateless host module for guest `libc` calls; state lives in [`HleContext`].
pub struct LibcModule;

impl HleModule for LibcModule {
    fn name(&self) -> &str {
        "libc"
    }

    fn symbols(&self) -> &'static [(&'static str, HostCall)] {
        &[
            ("_init_env", HostCall::InitEnv),
            ("atexit", HostCall::Atexit),
            ("catchReturnFromMain", HostCall::CatchReturnFromMain),
            ("exit", HostCall::Exit),
            ("Need_sceLibc", HostCall::NeedSceLibc),
            ("printf", HostCall::Printf),
            ("puts", HostCall::Puts),
            ("qsort", HostCall::Qsort),
            ("rand", HostCall::Rand),
            ("ferror", HostCall::Ferror),
            ("time", HostCall::Time),
            ("setjmp", HostCall::Setjmp),
            ("malloc", HostCall::Malloc),
            ("free", HostCall::Free),
            ("calloc", HostCall::Calloc),
            ("realloc", HostCall::Realloc),
            ("memset", HostCall::Memset),
            ("memcpy", HostCall::Memcpy),
            ("memmove", HostCall::Memmove),
            ("memcmp", HostCall::Memcmp),
            ("strlen", HostCall::Strlen),
            ("strcpy", HostCall::Strcpy),
            ("strncpy", HostCall::Strncpy),
            ("strcmp", HostCall::Strcmp),
            ("strncmp", HostCall::Strncmp),
            ("strchr", HostCall::Strchr),
            ("strrchr", HostCall::Strrchr),
            ("strstr", HostCall::Strstr),
            ("strcat", HostCall::Strcat),
            ("strncat", HostCall::Strncat),
            ("memchr", HostCall::Memchr),
            ("fopen", HostCall::Fopen),
            ("fclose", HostCall::Fclose),
            ("fread", HostCall::Fread),
            ("fwrite", HostCall::Fwrite),
            ("fseek", HostCall::Fseek),
            ("ftell", HostCall::Ftell),
            ("fflush", HostCall::Fflush),
            ("fgetc", HostCall::Fgetc),
            ("fputc", HostCall::Fputc),
            ("getc", HostCall::Getc),
            ("putc", HostCall::Putc),
        ]
    }

    fn call(
        &mut self,
        ctx: &mut HleContext,
        host: &mut dyn Host,
        call: HostCall,
        args: &[u64],
    ) -> Result<u64, EmuError> {
        match call {
            HostCall::InitEnv | HostCall::NeedSceLibc => calls::init_env(),
            HostCall::Atexit => {
                if ctx.libc.atexit_handlers >= ATEXIT_LIMIT {
                    return Err(EmuError::NoHandler("atexit handler limit".to_string()));
                }
                ctx.libc.atexit_handlers += 1;
                Ok(0)
            }
            HostCall::Exit => calls::exit(args),
            HostCall::CatchReturnFromMain => calls::catch_return_from_main(args),
            HostCall::Puts => calls::puts(host, args),
            HostCall::Printf => calls::printf(host, args),
            HostCall::Qsort => calls::qsort(host, args),
            HostCall::Ferror => calls::ferror(host, args),
            HostCall::Time => calls::time(host, args),
            HostCall::Setjmp => calls::setjmp(host, args),
            HostCall::Malloc => calls::malloc(host, args),
            HostCall::Free => calls::free(host, args),
            HostCall::Calloc => calls::calloc(host, args),
            HostCall::Realloc => calls::realloc(host, args),
            HostCall::Memset => calls::memset(host, args),
            HostCall::Memcpy => calls::memcpy(host, args),
            HostCall::Memmove => calls::memmove(host, args),
            HostCall::Memcmp => calls::memcmp(host, args),
            HostCall::Strlen => calls::strlen(host, args),
            HostCall::Strcpy => calls::strcpy(host, args),
            HostCall::Strncpy => calls::strncpy(host, args),
            HostCall::Strcmp => calls::strcmp(host, args),
            HostCall::Strncmp => calls::strncmp(host, args),
            HostCall::Strchr => calls::strchr(host, args),
            HostCall::Strrchr => calls::strrchr(host, args),
            HostCall::Strstr => calls::strstr(host, args),
            HostCall::Strcat => calls::strcat(host, args),
            HostCall::Strncat => calls::strncat(host, args),
            HostCall::Memchr => calls::memchr(host, args),
            HostCall::Fopen => calls::fopen(host, args),
            HostCall::Fclose => calls::fclose(host, args),
            HostCall::Fread => calls::fread(host, args),
            HostCall::Fwrite => calls::fwrite(host, args),
            HostCall::Fseek => calls::fseek(host, args),
            HostCall::Ftell => calls::ftell(host, args),
            HostCall::Fflush => calls::fflush(host, args),
            HostCall::Fgetc => calls::fgetc(host, args),
            HostCall::Fputc => calls::fputc(host, args),
            HostCall::Getc => calls::getc(host, args),
            HostCall::Putc => calls::putc(host, args),
            HostCall::Rand => Ok(calls::rand(&mut ctx.libc)),
            _ => Err(EmuError::NoHandler("libc".to_string())),
        }
    }
}
