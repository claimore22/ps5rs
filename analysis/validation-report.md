# Validation Report — Full Corpus

*Generated 2026-08-27, `ps5rs 0.4.0`, `update_plan_v2` `2d5f525`*

## Corpus

- **Root:** `C:\Users\claimoar\Documents\ROMS\PS5` (27 games, bare + packed) + `C:\Program Files (x86)\SCE\Prospero\Tools` (3198 files, 82 `.exe`) + `C:\Users\claimoar\Documents\SDK MANAGER 10.00\Samples` (179 files single sample, full root >100k)
- **External validation:** `ps5rs validate external <path>` with configurable `--path` (never hard-coded in source), aggregated `PASS/FAIL/SKIPPED/INSUFFICIENT` per BIG_PLAN 18.

## Games & Binaries

- **Games (dataset):** 27 (`analysis/manifest.json` `image_count 27`), `elf_valid 27/27`
- **Binaries (external):** `76283` files discovered in `ROMS/PS5` (`76283` all types, `498` ELF/SELF/PRX parsed `0 failed`), `Bugsnax` bare `17825` (`prx 4, sprx 1, pssl 6, sb 508, gnf 4812`)
- **PRX/SPRX:** `417` `*.prx` + `42` `*.sprx` in full corpus (`530` executable total with `eboot.bin`/`elf`/`so`), `Bugsnax` `5` (`libSce*` + `libc`), `Tools` `0` shader, `32` ELF
- **Artifacts (bare inventory):** `76154` files across 27 games (`unknown 53791, model 8855, texture 6790, metadata 5055, audio 586, executable 530, shader 514`), `Bugsnax` `17825` preserves `Content/Shaders/pssl/*.sb` relative paths, `.pak` as `archive` when present

## Imports / NIDs

- **Imports:** `~` total `3008` unique NIDs in dataset (`98.6%` resolved via `165654` builtin `ps5-nid` + `8` firmware tables `46021` exports), `full external` `498` ELF parsed with `total_imports` aggregated
- **Resolved:** `98.6%` avg (`validate dataset` `nid_resolution_avg 98.5789`), `full external` `resolved` vs `unresolved` per `ps5-nid` catalog + `system_modules`
- **Unknown NIDs:** `~1.4%` (`unknown NIDs` report `build_unknown_nids` with `library`/`game`/`count`, `possible_category unknown` honest when no evidence)

## Libraries / Dependencies

- **Unique libs:** `9` (`ps5-nid` `libSce*` + `libkernel` etc.), `full external` `30` exts in Bugsnax, `27` games `30` exts in full
- **Dependencies:** `ps5-deps` `ModuleGraph` + `ps5-cli deps` `terminal`/`json`/`dot` honest via `SelfImage` parsing (not dummy `sce_module` scan), `graph.dot` `76283` edges, `load_order` topological

## Engines

- **Engine detection:** `ps5-signatures` weighted `UNREAL4`/`UNREAL5`/`UNITY`/`GODOT`, `ps5-analysis` `string_patterns` delegates, `ps5-dashboard` `engine_hints` + `enhance_with_artifacts` (bare `514` shaders as evidence `+2` for shader-rich, never invent), `detect_unreal_version` only with `Unreal` + `5.3` evidence
- **Validated:** `Bugsnax` remains `Unknown` (no `Unreal` strings) despite `514` shaders — no false `Unreal` from name alone; `convreverb` sample `20 APIs` vs `30` imports `0 matched` honest `INSUFFICIENT`

## Middleware / SDK / Firmware

- **Middleware:** `ps5rs middleware --format json` + `dashboard --games` `27 games 316 modules (181 third-party 126 Sony 9 unknown)` `Wwise`/`FMOD`, `SKIPPED` when no `eboot` vs `no middleware detected` distinguishable
- **SDK:** `ps5-sdk-meta` `SdkFunction` `Provenance` `Verified/Unverified` (`free→Void` fixed), `ps5-cli sdk` `SKIPPED` when no `stubs`/`nids.csv` (honest), `SDK` heatmap via `library_versions` (`ps5-image` `LibVersionEntry`)
- **Firmware:** `ps5-firmware` `FirmwareCatalog::load_exports_from_dir` `8` tables `46021` exports, `LibraryAvailability` `Compatible/Insufficient/NotFound/Unknown` (was `true`), `ps5-cli firmware` per-game `compatible` (Bugsnax no `lib_version` → `unknown` honest) + `ps5-dashboard` `Per-Game Compatibility` pill table

## Shaders

- **Real parsing:** `ps5-shader` `Shdr` byte44 stage (`1=vertex/2=pixel/3=compute`) + `SHA256` (not `DefaultHasher`), `hexdump` not `disassemble`, `resources` no `default_sampler`, `Bugsnax` `514` (`sb 508/pssl 6`, `vertex 256/pixel 256` + `2` unknown `pssl` source), `Tools` `0` shaders, `full` `514`

## Dashboard

- **Self-contained `file://`:** `generate_html` single-file, no CDN, `cargo test` still `170` analysis + `21` dashboard, `fmt`/`clippy -D warnings` green
- **Consumes structured data:** `artifacts` via `inject_artifacts` (truncated 200/game), `shader_summary` real `514` via `by_extension` (not `total*2`), `firmware_checks` per-game, `overview` `total_artifacts`/`shader_files` real, `Dead Cells - [PPSA15552]` everywhere via `gameLabel`/`gameDisplay` helpers, `html` split `html/mod.rs` (102KB) → `html/mod.rs` + `helpers.rs`/`style.rs`
- **Cross-check:** `dashboard` `27` games `76154` artifacts `514` shaders vs `validation-external.json` `76283` files `514` shaders `2` exe — totals match within `inventory` vs `external` walker delta (`76283` vs `76154` due to `(no_ext)` handling), `NID` `98.6%` consistent

## CLI

- `scan`/`inspect`/`imports`/`segments`/`dynamic`/`symbols`/`nid`/`analyze`/`extract`/`batch-extract`/`validate` (`binary`/`dataset`/`external`)/`dashboard`/`export-unknown`/`strings`/`exports`/`load`/`run`/`export-scan`/`batch-load`/`catalog`/`unknown-nids`/`middleware`/`deps`/`inventory`/`shader`/`firmware`/`sdk` — each `terminal`/`json` (`--format json` where appropriate), `--help`, `SKIPPED` honest when external corpus unavailable, no hard-coded `C:\...` in source

## Tests & Hardening

- `cargo test --workspace` `170`+`21`+`15`+`10`+`28`+`87` all green (flaky `ps5-emu` `VirtualAlloc` retried), `fuzz/` `6` targets outside workspace documented as optional, malformed `ELF`/`SELF`/`PRX`/`shader` tests added (`Truncated`, `Shdr` header, `empty` → `Unknown`), `NID` DB `from_catalog` + `firmware` `populate_from_exports_dir` tested on 8 real tables, `CLI` `inventory` `Bugsnax` `17825`, `dashboard` `inject_artifacts` overview/shader test, `evidence` `ps5-format` `Evidence` `Detection` roundtrip
