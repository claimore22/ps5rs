//! Guest-facing output formatting shared by the `libc` and `libSceDbg`
//! modules.  A minimal printf-style substituter over variadic argument slots.

use crate::modules::Host;

/// Minimal printf-style substitution over the variadic argument slots.
///
/// Reads integer and string arguments from `args` in format-string order;
/// `%s` pointers are resolved through `host`.  Callers slice `args` to the
/// first variadic argument (e.g. `&args[1..]` for `printf`, `&args[5..]` for
/// `sceDbgLoggingHandler`).
pub(crate) fn format_printf(host: &dyn Host, format: &str, args: &[u64]) -> String {
    fn read_arg(args: &[u64], index: &mut usize) -> u64 {
        let v = args.get(*index).copied().unwrap_or(0);
        *index += 1;
        v
    }

    let mut out = String::with_capacity(format.len() + 16);
    let mut index = 0usize;
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let spec = match chars.next() {
            Some('l') | Some('h') | Some('z') | Some('j') | Some('t') => {
                chars.next().unwrap_or('%')
            }
            Some(s) => s,
            None => {
                out.push('%');
                break;
            }
        };
        match spec {
            '%' => out.push('%'),
            'd' | 'i' | 'u' => {
                let v = read_arg(args, &mut index);
                out.push_str(&format!("{v}"));
            }
            'x' => {
                let v = read_arg(args, &mut index);
                out.push_str(&format!("{v:x}"));
            }
            'X' => {
                let v = read_arg(args, &mut index);
                out.push_str(&format!("{v:X}"));
            }
            'p' => {
                let v = read_arg(args, &mut index);
                out.push_str(&format!("{v:#x}"));
            }
            'c' => {
                let v = read_arg(args, &mut index);
                out.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}'));
            }
            's' => {
                let p = read_arg(args, &mut index);
                out.push_str(&host.read_string(p).unwrap_or_default());
            }
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::EmuError;

    struct MemoryHost {
        strings: std::collections::HashMap<u64, String>,
    }

    impl Host for MemoryHost {
        fn read_bytes(&self, _addr: u64, _len: usize) -> Result<Vec<u8>, EmuError> {
            Ok(Vec::new())
        }
        fn read_string(&self, addr: u64) -> Result<String, EmuError> {
            self.strings
                .get(&addr)
                .cloned()
                .ok_or(EmuError::Unmapped(addr))
        }
        fn write(&mut self, _addr: u64, _data: &[u8]) -> Result<(), EmuError> {
            Ok(())
        }
    }

    fn host() -> MemoryHost {
        let mut strings = std::collections::HashMap::new();
        strings.insert(0x100, "world".to_string());
        MemoryHost { strings }
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(format_printf(&host(), "hello", &[]), "hello");
    }

    #[test]
    fn integer_and_string_specs() {
        let h = host();
        assert_eq!(
            format_printf(&h, "%d + %s = %x", &[2, 0x100, 0x1A]),
            "2 + world = 1a"
        );
    }

    #[test]
    fn percent_escapes_and_length_modifiers() {
        let h = host();
        assert_eq!(format_printf(&h, "100%% done", &[]), "100% done");
        assert_eq!(format_printf(&h, "%ld", &[7]), "7");
    }

    #[test]
    fn missing_args_substitute_zero() {
        let h = host();
        assert_eq!(format_printf(&h, "%d", &[]), "0");
    }

    #[test]
    fn unknown_spec_is_preserved() {
        let h = host();
        assert_eq!(format_printf(&h, "%q", &[1]), "%q");
    }
}
