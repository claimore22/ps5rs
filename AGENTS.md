# ps5rs — Agent Guide

## Overview

PS5 binary analysis, virtual loading, and host-side emulation framework. Parses SELF/ELF/PRX, resolves NID imports, fingerprints games (engine/middleware/SDK detection), generates dashboards. Provides a virtual PS5 loader (maps ELF memory, applies relocations, resolves imports via 3-tier resolution runtime → offline → stubs) and a host-side emulator (`ps5-emu`) that executes guest binaries on the host CPU, routing system-library imports through pure-Rust HLE modules.

## Quick Start

```sh
cargo build --release
cargo test --workspace    # 525+ tests
just check                # fmt + clippy + test (default)
```

## Workspace (11 crates)

| Crate | Purpose |
|---|---|
| `ps5-format` | Shared types, error enums, ELF/SELF constants, SHA-256 |
| `ps5-self` | SELF container parser |
| `ps5-elf` | ELF64 binary parser (zero-copy) |
| `ps5-nid` | NID hash algorithm (SHA1+Sony base64), name catalog |
| `ps5-image` | BinaryImage IR with JSON serialization |
| `ps5-analysis` | Scanner, dataset, string fingerprinting, engine detection, reports |
| `ps5-loader` | Virtual loader: 4-phase pipeline (Map/Relocate/Link/Init) |
| `ps5-emu` | Host-side emulator: native guest execution, HLE modules, `ExecutionReport` |
| `ps5-tests` | Self-authored ELF fixture generator + expected-behavior manifest |
| `ps5-dashboard` | Static HTML dashboard (self-contained, no CDN) |
| `ps5-cli` | Binary `ps5rs` — all subcommands |

Dependency direction: `format → {elf, self, nid} → image → analysis → cli`, with `loader → elf`, `ps5-emu → {loader, elf, nid, format}`, `ps5-tests → {format, nid}`.

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
- Subcommand files in `crates/ps5-cli/src/`: `inspect.rs`, `load.rs`, `strings.rs`, `catalog.rs`, `batch_load.rs`, `export_scan.rs`, `run.rs`, `middleware.rs`, `dataset.rs`, etc.
- `cli.rs` purely declarative (types only, no helpers)

## Loader Pipeline

1. **Map** — `load_elf()`: PT_LOAD → virtual memory, zero-fill `.bss`
2. **Relocate** — RELATIVE (DT_RELACOUNT fast path), ABS64, GLOB_DAT, JUMP_SLOT
3. **Link** — register exports, 3-tier resolution (runtime → offline `./system_modules/` → stubs)
4. **Init** — (planned) `.init_array`/`.preinit_array`/`DT_INIT`

`load_modules_at` loads the first module at a caller-supplied base (default `DEFAULT_LOAD_BASE = 0x810000000`) so several processes can share one host address space.

## Emulator

- `ps5-emu` runs guest entry points as native x86-64; only the ABI boundary is machine code (`abi::sysv64`): stubs forward the six SysV integer registers + guest stack pointer into the `Registry`, which dispatches by computed NID to pure-Rust HLE modules under `hle/` (`hle/libc/`, `hle/libkernel/`, `hle/libSceDbg/`), each with thin handlers in a per-library `calls.rs`.
- API: `Emulator::from_elf(...)` (default modules, `DEFAULT_LOAD_BASE`) → `resolve_imports_with(&catalog)` → `run() -> ExecutionReport` (module name, entry point, exit code, ordered `import_calls`). `ps5-emu/serde` feature gates report serialization.
- Loader/emulator split: `Process` owns memory + loaded modules (`Process::load_at` for custom bases); the emulator wraps a `Process`, a `Registry`, and execution state.
- Library names in traces resolve through the module's `import_libs` table (NID-derived lib id fallback), so masked symbols read as `libkernel::puts`.
- Reference flows: `crates/ps5-cli/src/run.rs` (CLI) and `crates/ps5-emu/examples/boot.rs`.

## Agent Loop (OpenCode + NPU Planner)

```
YOU → Commander (Ollama OSS-20B, local) → Planner (Qwen3-8B on Intel NPU)
       → Worker (cloud LLM) → Validator (deterministic) → Reviewer (cloud)
```

- **Commander** (`ollama/gpt-oss:20b`): understands mission from `UPGRADE_PLAN.md`, controls loop.
- **Planner** (`npu/Qwen3-8B` → `http://localhost:11435/v1` via `npu-test/npu_server.py` on Intel AI Boost NPU): decomposes one ready objective into file-level tasks with deps/risks. One-shot, ~5 tok/s is irrelevant vs Worker's 5–15 min. Start server: `npu-test/run_npu_server.bat` (health: `curl http://localhost:11435/health`).
- **Worker** (cloud LLM, default model): writes code per plan. Must not re-plan.
- **Validator** (deterministic, no LLM): `just check` = `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --workspace` (525+ tests) + `git diff` + `cargo check`. Evidence flows to Reviewer; Worker claims are not trusted.
- **Reviewer** (cloud LLM): PASS→next task, FAIL→correction back to Worker.

`opencode.jsonc` routes `agent.commander.model`→Ollama and `agent.planner`/`agent.plan`→NPU; Worker/Reviewer use the primary cloud model. Mission queue is `UPGRADE_PLAN.md` checklist (ps5-prx/ps5-schema/ps5-abi/…).

## Testing

- Inline `#[cfg(test)] mod tests { use super::*; }` in every source file
- Descriptive snake_case names: `parse_minimal_elf_header`, `loaded_segment_roundtrip`
- Edge cases: truncated data, wrong magic, zero-size TLS, empty sections
- Property tests: `proptest` in `ps5-elf`
- **Generated-fixture regression** (primary emulator suite): `ps5-tests` renders byte-exact ELFs + `manifest.json` to `data/test/generated_elfs/`; `ps5-emu/tests/elf_suite.rs` boots every fixture through the real loader + HLE pipeline and asserts exit code, import trace, and (via `print_string`) guest-string reads. Rebuild fixtures with `cargo run -p ps5-tests --bin generate` when fixtures change, and commit the resulting bytes + manifest.
- **Validator contract**: every Worker change must pass `just check` before Reviewer sees it. Reviewer judges the validator's stdout, not the Worker's self-report.

## Git

- Conventional commits: `feat:`, `fix:`, `docs:`, `chore:`
- Feature branches from `master`
- `system_modules/`, `analysis_old/`, `*.elf` gitignored (large/proprietary) — the tiny self-authored fixtures under `data/test/generated_elfs/` are force-added (`git add -f`) so the regression suite works on fresh clones
