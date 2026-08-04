//! Declarative fixture definitions: names, byte generators, and expected
//! behavior.  The `generate` binary renders these to
//! `data/test/generated_elfs/`, and the committed bytes are regression-checked
//! below so regeneration never silently changes a fixture.

use ps5_format::elf_constants::ET_EXEC;
use ps5_nid::algorithm::hash;

use crate::codegen;
use crate::dynelf;
use crate::elf::{ElfSpec, build};
use crate::manifest::{ARG_WILDCARD, ImportExpectation};

/// One generated fixture plus its expected guest behavior.
pub struct Fixture {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub expected_exit: u64,
    pub imports: Vec<ImportExpectation>,
    pub print_string: Option<&'static str>,
    /// Chunks the guest must emit to stdout through the HLE modules, in order.
    pub stdout: Vec<String>,
}

/// Every fixture the generator knows how to produce.
pub fn all() -> Vec<Fixture> {
    vec![hello(), hello_puts(), libdbg_basic()]
}

/// `hello.elf` — a `_start` that just returns.  Boots, maps two segments,
/// hits the entry, and exits with code 0 with no dynamic machinery involved.
fn hello() -> Fixture {
    Fixture {
        name: "hello.elf",
        bytes: build(&ElfSpec {
            e_type: ET_EXEC,
            code: codegen::ret(),
            data: vec![0; 16],
        })
        .bytes,
        expected_exit: 0,
        imports: Vec::new(),
        print_string: None,
        stdout: Vec::new(),
    }
}

/// `hello_puts.elf` — a dynamic `ET_SCE_DYNEXEC` guest that imports
/// `libkernel::puts` through a GOT slot and hands it a string in `rdi`.
/// Boots, relocates the GOT slot, resolves the import to an HLE stub, prints
/// the message, and exits 0 on the natural return.  The printed string is the
/// same one the SDK hello prints, so the generated fixture and the SDK binary
/// behave identically.
fn hello_puts() -> Fixture {
    let puts_nid = hash("puts");
    Fixture {
        name: "hello_puts.elf",
        bytes: dynelf::hello_puts(b"Hello from ps5rs!\0", format!("{puts_nid}#libkernel")),
        expected_exit: 0,
        imports: vec![ImportExpectation {
            library: "libkernel".to_string(),
            name: "puts".to_string(),
            args: [0; 6],
            return_value: 0,
        }],
        print_string: Some("Hello from ps5rs!"),
        stdout: vec!["Hello from ps5rs!\n".to_string()],
    }
}

/// `libdbg_basic.elf` — a deterministic twin of the SDK `api_libdbg/basic`
/// sample's `run()` sequence.  Four imports (min-level, puts, rand, logging
/// handler), 16 GOT calls, and stack varargs pushed right-to-left across the
/// DEBUG/WARNING/ERROR calls.  The compiled-in `__FILE__`/`__LINE__`/format
/// constants mirror the sample so the emitted stdout is byte-predictable.
fn libdbg_basic() -> Fixture {
    let imports = vec![
        format!("{}#libSceDbg", hash("sceDbgSetMinimumLogLevel")),
        format!("{}#libkernel", hash("puts")),
        format!("{}#libc", hash("rand")),
        format!("{}#libSceDbg", hash("sceDbgLoggingHandler")),
    ];
    let messages: Vec<&'static [u8]> = vec![
        b"Hello from ps5rs!\0".as_slice(),
        b"basic.cpp\0".as_slice(),
        b"\0".as_slice(),
        b"One random number: %d\n\0".as_slice(),
        b"Two random numbers: %d, %d\n\0".as_slice(),
        b"Three random numbers: %d, %d, %d %s\n\0".as_slice(),
        b"Four random numbers: %d, %d, %d, %d\n%s\n\0".as_slice(),
        b"My mind is going\0".as_slice(),
        b"Daisy, daisy, give me your answer do\0".as_slice(),
    ];
    let p = dynelf::plan(&messages, &imports);
    let addrs = codegen::LibdbgBasicAddrs {
        min_level: p.got[0],
        puts: p.got[1],
        rand: p.got[2],
        handler: p.got[3],
        msg_hello: p.messages[0],
        file: p.messages[1],
        component: p.messages[2],
        fmt_trace: p.messages[3],
        fmt_debug: p.messages[4],
        fmt_warning: p.messages[5],
        fmt_error: p.messages[6],
        mind: p.messages[7],
        daisy: p.messages[8],
    };
    let spec = dynelf::MultiDynamicSpec {
        code: codegen::libdbg_basic_code(&addrs),
        messages,
        imports: imports.clone(),
    };
    let w = ARG_WILDCARD;
    Fixture {
        name: "libdbg_basic.elf",
        bytes: dynelf::build_multi(&spec),
        expected_exit: 0,
        imports: vec![
            ImportExpectation {
                library: "libSceDbg".to_string(),
                name: "sceDbgSetMinimumLogLevel".to_string(),
                args: [1, 0, 0, 0, 0, 0],
                return_value: 0,
            },
            ImportExpectation {
                library: "libkernel".to_string(),
                name: "puts".to_string(),
                args: [w, 0, 0, 0, 0, 0],
                return_value: 0,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 200_494_509,
            },
            ImportExpectation {
                library: "libSceDbg".to_string(),
                name: "sceDbgLoggingHandler".to_string(),
                args: [w, 33, 0, w, w, 200_494_509],
                return_value: 0,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 40_788_086,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 3_851_444_534,
            },
            ImportExpectation {
                library: "libSceDbg".to_string(),
                name: "sceDbgLoggingHandler".to_string(),
                args: [w, 36, 1, w, w, 40_788_086],
                return_value: 0,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 915_262_580,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 2_714_061_548,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 1_316_748_153,
            },
            ImportExpectation {
                library: "libSceDbg".to_string(),
                name: "sceDbgLoggingHandler".to_string(),
                args: [w, 39, 3, w, w, 915_262_580],
                return_value: 0,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 3_605_590_735,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 452_227_306,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 2_966_872_715,
            },
            ImportExpectation {
                library: "libc".to_string(),
                name: "rand".to_string(),
                args: [0; 6],
                return_value: 1_229_098_382,
            },
            ImportExpectation {
                library: "libSceDbg".to_string(),
                name: "sceDbgLoggingHandler".to_string(),
                args: [w, 42, 4, w, w, 3_605_590_735],
                return_value: 0,
            },
        ],
        print_string: None,
        stdout: vec![
            "Hello from ps5rs!\n".to_string(),
            "[basic.cpp:36] [] Two random numbers: 40788086, 3851444534\n\n".to_string(),
            "[basic.cpp:39] [] Three random numbers: 915262580, 2714061548, 1316748153 My mind is going\n\n".to_string(),
            "[basic.cpp:42] [] Four random numbers: 3605590735, 452227306, 2966872715, 1229098382\nDaisy, daisy, give me your answer do\n\n".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fixture_names_are_unique() {
        let names = all()
            .iter()
            .map(|f| f.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), all().len());
    }

    #[test]
    fn committed_fixtures_match_generation() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/test/generated_elfs");
        for fixture in all() {
            let committed = std::fs::read(dir.join(fixture.name)).unwrap_or_else(|_| {
                panic!(
                    "missing committed fixture {}; run `cargo run -p ps5-tests --bin generate`",
                    fixture.name
                )
            });
            assert_eq!(
                fixture.bytes, committed,
                "fixture {} drifted from generation; run `cargo run -p ps5-tests --bin generate`",
                fixture.name
            );
        }
    }
}
