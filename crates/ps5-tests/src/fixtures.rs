//! Declarative fixture definitions: names, byte generators, and expected
//! behavior.  The `generate` binary renders these to
//! `data/test/generated_elfs/`, and the committed bytes are regression-checked
//! below so regeneration never silently changes a fixture.

use ps5_format::elf_constants::ET_EXEC;
use ps5_nid::algorithm::hash;

use crate::codegen;
use crate::dynelf;
use crate::elf::{ElfSpec, build};
use crate::manifest::ImportExpectation;

/// One generated fixture plus its expected guest behavior.
pub struct Fixture {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub expected_exit: u64,
    pub imports: Vec<ImportExpectation>,
    pub print_string: Option<&'static str>,
}

/// Every fixture the generator knows how to produce.
pub fn all() -> Vec<Fixture> {
    vec![hello(), hello_puts()]
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
