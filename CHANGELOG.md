# Changelog

All notable changes to ps5rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **NID Catalog v2** (`ps5-nid`): `NidEntry` struct with `names`, `libraries`, `tags`, `sources` as `BTreeSet<String>`; `resolve()` returns `Option<&NidEntry>` instead of `Option<&str>`; `primary_name()` convenience method; `insert()` for rich metadata with merge semantics
- **Rich CSV catalog format**: auto-detects legacy space-separated (`NID name`) vs new comma-separated (`nid,name,library,tag,source`) with optional header row; later files override earlier ones via merge
- **String-based fingerprinting** (`ps5-analysis::string_patterns`): `extract_strings()`, `detect_sce_libraries()`, `detect_engine()`, `detect_third_party()`, `detect_build_system()`, `detect_depot()`, `detect_project_paths()`, `detect_custom_forks()`, `detect_sdk_hints()`, `detect_versions()`, `detect_source_paths()`, `analyze_strings()` orchestrator
- **Weighted engine fingerprints** (`ps5-analysis::engine_fingerprints`): `EngineFingerprint` struct with `score()` method; const definitions for `UNREAL4`, `UNREAL5`, `UNITY`, `GODOT`; tie-breaking prefers newer engine; `detect_engine()` returns `Detection` with computed confidence; `detect_custom_forks()` for P4Damascus, HK_Project_Delivery, HK_EngineSources
- **Detection confidence and evidence**: `Detection` struct gains `confidence: u8` and `evidence: Vec<String>` fields (backward compatible with `#[serde(default)]`)
- **`StringAnalysis` on BinaryImageDocument**: `sce_libs`, `engine`, `build_system`, `depot`, `third_party`, `sdk_hints`, `versions`, `source_paths`, `custom_forks` fields
- **`custom_forks` on `EngineHint`**: merged from ELF-based and string-based detection
- **`strings` subcommand**: `ps5rs strings <FILE> [-n MIN] [--offsets] [--detect] [-o OUTPUT]` for byte-level string extraction from any binary; `--detect` prints structured summary (SCE libs, engine, build system, depot, third-party, SDK, versions, source paths, custom forks)
- **`export-unknown` command**: `ps5rs export-unknown <path> --group-by frequency|library` outputs unknown NIDs as CSV (`library,nid,count,games`) with semicolon-separated games
- **`--nids` flag**: external NID catalog files on `scan`, `analyze`, and `batch-extract`; later files override earlier ones
- **`display_name` field**: cached derived display name on manifest games; `display_name_for()` helper with case-insensitive lookup; `--display-name` flag on `inspect`
- **Schema v5**: bumped from v3 for `confidence` field on `Detection`, `custom_forks` on `StringAnalysis`
- **Workspace infrastructure**: `unsafe_code = "forbid"`, MSRV 1.85, `rustfmt.toml`, `.editorconfig`, CI lint→test pipeline, `CONTRIBUTING.md`, PR/issue templates, `CODEOWNERS`, `justfile`

### Changed
- **`BinaryImageBuilder::build_from_file`** now takes `&[u8]` instead of `Vec<u8>` — borrows instead of cloning hundred-MB binaries; all callers updated
- **`build_engine_hints()`** merges ELF-based + string-based engine detection; `EngineHint` struct gains `engines`, `third_party_libs`, `build_system`, `source_depot`, `sdk_hints`, `detected_versions`, `source_paths`, `project_paths`, `custom_forks` fields
- **Engine detection**: UE5 detection uses strong patterns only (`UnrealEngine5Runtime`, `UE5Runtime`, `Nanite`, `Lumen`); UE4 detection uses `UnrealEngine4Runtime`, `Engine/Source/Runtime`, `Engine/Plugins`, `PhysXCooking`, `QuickHullConvexHullLib`; Unity confidence: `UnityEngine` = 90, `UnityPlayer` = +5, `il2cpp` = +5, `global-metadata.dat` = +5
- **SCE library detection**: `contains("libSce")` not `starts_with("libSce")` for embedded paths
- **Source depot filtering**: first path component must be >=2 chars, alphanumeric/underscore/dash only (filters binary noise like `F:\!`, `A:\5`)
- **`extract_strings()`**: capped at `MAX_STRING_LENGTH = 4096`
- **Source depot split**: handles both `/` and `\` path separators
- **28 games re-scanned**: 197 unique unknown NIDs (2.2% of 29,977 imports) — down from 3,008 before string-based fingerprinting

### Fixed
- **P4Damascus custom fork detection**: "Unreal Engine 4 custom fork (P4Damascus depot)" at 90% confidence on 8 UE4 games
- **UE5 false positive prevention**: weak patterns no longer trigger UE5 detection
- **Source depot noise**: binary garbage in `F:\!`-style paths no longer detected as source depots

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
- **SELF->ELF offset mapping**: `phdr_file_offsets` array correctly maps ELF phdr indices to SELF data segment file offsets
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
