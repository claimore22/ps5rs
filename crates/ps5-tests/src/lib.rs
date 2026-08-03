//! `ps5-tests` — deterministic, self-authored ELF fixtures for the emulator.
//!
//! The generator renders fixtures to `data/test/generated_elfs/` as
//! byte-exact binaries plus a `manifest.json` of expected behavior; the
//! emulator's `elf_suite` integration test runs each fixture against the real
//! loader + HLE pipeline.  No private SDK binaries are involved.

pub mod codegen;
pub mod dynelf;
pub mod elf;
pub mod fixtures;
pub mod manifest;
