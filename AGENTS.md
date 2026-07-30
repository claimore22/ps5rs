# ps5rs — Agent Guide

## Overview

PS5 binary analysis and virtual loading framework. Parses SELF/ELF/PRX, resolves NID imports, fingerprints games (engine/middleware/SDK detection), generates dashboards. Also provides a virtual PS5 loader: maps ELF memory, applies relocations, resolves imports via 3-tier resolution (runtime → offline → stubs).

## Quick Start

```sh
cargo build --release
cargo test --workspace    # 430+ tests
just check                # fmt + clippy + test (default)
```

## Workspace (9 crates)

| Crate | Purpose |
|---|---|
| `ps5-format` | Shared types, error enums, ELF/SELF constants, SHA-256 |
| `ps5-self` | SELF container parser |
| `ps5-elf` | ELF64 binary parser (zero-copy) |
| `ps5-nid` | NID hash algorithm (SHA1+Sony base64), name catalog |
| `ps5-image` | BinaryImage IR with JSON serialization |
| `ps5-analysis` | Scanner, dataset, string fingerprinting, engine detection, reports |
| `ps5-loader` | Virtual loader: 4-phase pipeline (Map/Relocate/Link/Init) |
| `ps5-dashboard` | Static HTML dashboard (self-contained, no CDN) |
| `ps5-cli` | Binary `ps5rs` — all subcommands |

Dependency direction: `format → {elf, self, nid} → image → analysis → cli`, with `loader → elf`.

## Code Conventions

- **Edition 2024**, MSRV 1.85, `unsafe_code = "forbid"` (deny across workspace)
- **No comments** in source code unless explaining *why*, not *what*
- **Snake_case** for functions/variables, **PascalCase** for types
- **Standard ELF terminology** — `ElfHeader`, `ProgramHeader`, `Relocation` (not renamed)
- **Sony-specific** gets new names — `SelfImage`, `SceDynamicTag`
- **Error handling**: `Result<T, ps5_format::ParseError>` in parsers; loader has `LoaderError`, `MemoryError`, `RelocationError`
- **Zero-copy parsers**: all types borrow `&'a [u8]`
- **Serde**: hex addresses as `"0x..."`, `#[serde(default)]` for backward compat, `skip_serializing_if`
- **Imports**: `std` → blank → extern → blank → internal → blank → `mod`
- **No regex** — use `contains()` for string matching
- **Case-insensitive** matching for title IDs

## CLI

- Binary: `ps5rs` (from `ps5-cli` crate, `clap` derive API)
- Subcommand files in `crates/ps5-cli/src/`: `inspect.rs`, `load.rs`, `strings.rs`, `catalog.rs`, etc.
- `cli.rs` purely declarative (types only, no helpers)

## Loader Pipeline

1. **Map** — `load_elf()`: PT_LOAD → virtual memory, zero-fill `.bss`
2. **Relocate** — RELATIVE (DT_RELACOUNT fast path), ABS64, GLOB_DAT, JUMP_SLOT
3. **Link** — register exports, 3-tier resolution (runtime → offline `./system_modules/` → stubs)
4. **Init** — (planned) `.init_array`/`.preinit_array`/`DT_INIT`

## Testing

- Inline `#[cfg(test)] mod tests { use super::*; }` in every source file
- Descriptive snake_case names: `parse_minimal_elf_header`, `loaded_segment_roundtrip`
- Edge cases: truncated data, wrong magic, zero-size TLS, empty sections
- Property tests: `proptest` in `ps5-elf`

## Git

- Conventional commits: `feat:`, `fix:`, `docs:`, `chore:`
- Feature branches from `master`
- `system_modules/` and `analysis_old/` gitignored (large/proprietary)
