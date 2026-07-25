# Changelog

All notable changes to ps5rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- `ps5rs scan` command: produces versioned dataset directory with individual `BinaryImageDocument` JSON files per game
- `ps5rs analyze unknown` command: reports unresolved NID hashes across the dataset, sorted by frequency
- Import inventory report: Library × Games × Imports table via `ps5rs analyze imports`
- `AnalysisDataset` type with `open()` for loading dataset directories
- `Manifest` type with schema versioning for forward compatibility
- `DatasetError` enum for extensible dataset error handling
- Scanner deduplication: only first eboot.bin per game directory (skips patch/DLC subdirs)

### Changed
- `analyze stats`, `analyze heatmap`, `analyze imports` now auto-detect dataset directories (loads from JSON files instead of parsing binaries)
- `AnalysisDataset.images` stores `(String, BinaryImageDocument)` tuples with game names derived from filenames

## [0.1.0-alpha] - 2025-07-25

### Added
- **BinaryImage JSON serialization** (`ps5-image::json`): `export_json` / `import_json` with `BinaryImageDocument` wrapper, hex addresses, ELF-style flags, `JsonError` enum
- **BinaryImage IR** (`ps5-image`): normalized abstraction between parsers and consumers — `BinaryImage`, `LoadedSegment`, `ImportEntry`, `ExportEntry`, `RelocationEntry`, `TlsInfo`
- **BinaryImageBuilder**: `build_from_file()` and `build_from_self()` converting raw parser output to normalized IR
- **ps5-analysis crate**: directory walking, import collection, 5 report types (stats, heatmap, frequency, unresolved, dependency graph), JSON/CSV/DOT export
- **CLI commands**: `inspect`, `imports`, `segments`, `dynamic`, `symbols`, `nid`, `analyze` (with subcommands: collect, stats, heatmap, frequency, unresolved, graph, imports)
- `inspect` and `imports` commands support `--json` and `-o` flags for machine-readable output
- 154K+ NID catalog embedded via `include_str!` from SharpEmu's `nids.csv`
- `sha256_hex()` utility in `ps5-format::hash` using the `sha2` crate
- `lib_id_from_nid()` in `ps5-nid::lookup` with overflow safety (`checked_mul`/`checked_add`)
- ELF e_machine validation: parser rejects non-x86-64 binaries with `ParseError::NotX86_64`
- ELF `DT_FINI`, `DT_FINI_ARRAY`, `DT_FINI_ARRAYSZ`, `DT_PREINIT_ARRAY`, `DT_PREINIT_ARRAYSZ` parsing
- Proptest property tests, malformed-but-valid ELF tests, integration snapshot tests (feature-gated)
- GitHub Actions CI: fmt, clippy, cargo test on ubuntu + windows
- GPL-2.0-only license, README with credits and comprehensive CLI usage docs

### Fixed
- **SELF→ELF offset mapping**: `phdr_file_offsets` array correctly maps ELF phdr indices to SELF data segment file offsets
- **DYNAMIC section resolution**: uses `vaddr_to_offset` through LOAD segments instead of trusting `p_offset` directly in SELF files
- **strtab bounds checking**: `parse_import_libs` and `parse_needed_files` verify `strtab_offset < data.len()` before slicing
- **SHA-256 computation**: replaced non-crypto accumulator with real SHA-256 via `sha2` crate
- **`lib_id_from_nid` deduplication**: single source of truth in `ps5-nid::lookup.rs` with overflow protection

## [0.0.1] - 2025-07-01

### Added
- Initial SELF/ELF/NID parser with 62 synthetic unit tests
- SELF container parsing with correct BE magic + LE fields
- ELF64 parsing: headers, segments, dynamic section, symbols, relocations, TLS
- NID hash algorithm: SHA1 + salt + Sony custom base64 alphabet
- Basic NID catalog with 162 built-in entries
- ElfBuilder test infrastructure for synthetic ELF/SELF construction
