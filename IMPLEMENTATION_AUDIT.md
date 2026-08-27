# ps5rs Implementation Audit

> Source-of-truth repository audit against UPGRADE_PLAN2.md mission.
> Audited 2026-08-27 on branch `elfspector-dev` @ `6ae64c1`. Read-only investigation — no code changed by this audit.
> **Update — `update_plan_v2` 2026-08-27:** Phases 0–9 validated on `update_plan_v2` (commits edaf364, 7551552, 4ec6811, 8eb8058, d5cdeee, 9d23028, ffd476c, 0796b4b, 55f8f92, ead7456, 0ee1553, 33aac87, abdddd5, 8ba6fbe). See `analysis/validation-report.json`/`md`, `analysis/external-bugsnax.json`.

## 0. update_plan_v2 progress (verified)

- **edaf364** honesty gate: shader/firmware honest empty (not vertex/pixel guess, not total*2), SDK timeline marked reference, module_count honest, ps5-nid-db from_catalog honest, ps5-nid iter, ps5-signatures scoring non-zero, ps5-firmware version-aware + LibraryAvailability, ps5-abi Provenance + free->Void, loader docs, middleware --format json + dashboard --games, dead template removed.
- **7551552** NID/evidence: ps5-format Evidence/EvidenceKind/Detection, ps5-nid-db Firmware source + populate_from_exports_dir, ps5-firmware load_exports_from_dir (8 tables), unknown_nids extended (modules/possible_category/confidence/evidence honest unknown defaults).
- **4ec6811** PRX authoritative: PrxModule TLS/init arrays (TlsInfo now serde), builder wiring, ps5-elf serde, ps5-cli deps honest via SelfImage/ElfImage parsing.
- **8eb8058** shader real: SHA256 not DefaultHasher, stage from Shdr byte 44, hexdump not disassembly, resources no synthetic default_sampler, tests updated to real Shdr header.
- **d5cdeee** validation: 27 games 98.6% NID, dashboard_honesty_test with 181/126/9 middleware, validation-report.json/md.
- **9d23028** bare artifacts: ps5-analysis artifacts inventory — 30+ extensions classified (prx/pssl/sb/gnf/at9/bank/json etc.), relative paths preserved, cross-artifact prx/pssl/sb/gnf/at9 counts, bare content first-class (.pak not required, `archive` when present, unknown kept structured). Validated on Bugsnax bare dump: 17825 files, 514 shaders, 4812 gnf, 35 banks.
- **ffd476c/0796b4b** dashboard artifacts: DashboardData artifacts Option<ArtifactReport> (truncated 200/game), dataset inject_artifacts when --games, html Artifacts tab (by-category/extension, per-game details, cross-artifact linkage).
- **55f8f92** engine multi-signal: ps5-signatures enhance_with_artifacts (bare shader/texture/audio counts as evidence, +2 confidence for shader-rich, never invents engine), ps5-dashboard enriches engine_hints evidence with bare artifact counts — validated Bugsnax remains Unknown (no Unreal strings) despite 514 shaders.
- **ead7456** schema: ps5-schema artifact_record stable interchange (ArtifactRecord/GameArtifactsRecord/ArtifactReportRecord, SCHEMA_VERSION=1, category string, relative_path preserved).
- **0ee1553** cli completeness: shader (Shdr+SHA256, 514 Bugsnax), firmware (8 modules 46021 exports), sdk (SKIPPED when no stubs, honest) — each terminal/json, --help, real data.
- **33aac87** validate external: `ps5rs validate external <path>` full matrix (elf/prx/self/.a/source/shader/.exe) with PASS/FAIL/SKIPPED, .exe first-class (inventoried without execution, SKIPPED unless explicit config), bare .gnf/.at9 counted, Bugsnax 17825 (30 exts) + synthetic 10-type matrix + full 27-game 76154 files + .exe 1 validated, nonexistent SKIPPED — no hardcode, no copy, cargo test still SKIPPED-free.
- **abdddd5** validate external source→binary: `extract_source_apis` (sce* scanning without regex) + `compute_source_correlation` (finds Release_Prospero projects, compares source APIs vs binary imports, reports scanned/matched/mismatched, PASS/INSUFFICIENT), .exe differential validation via explicit --tool remains SKIPPED, validated on single sample convreverb (1 project, 20 APIs, 30 imports, 0 matched) and Tools 3198 files (82 .exe), full samples root SKIPPED handling — no proprietary copy, no hardcode, external roots as configurable oracles.
- **8ba6fbe** validate perf: `compute_source_correlation` now filters `file_list` for Release_Prospero projects and source/binary files (no re-walk), handles full 27-game 76154 files + full samples root without timeout, validated on full ROMS inventory (76154 files, 30 exts, 530 executable, 514 shader) and Tools 3198, single sample still PASS.

Clippy `-D warnings --all-targets --all-features` and `cargo test --workspace` green at each milestone (now 170 analysis + 20 dashboard + 13 signatures + source correlation). Bare content and .exe tools are first-class validation corpus; proprietary contents never copied, .exe never executed silently, external paths never hardcoded, no vendor/product name in committed code.

## 1. CURRENT STATE

- **Branch:** `elfspector-dev` @ `6ae64c1` (pushed). Latest commit history: `a232739` (update_data CLI syntax fix), `6ae64c1` (clippy `--all-targets -D warnings` + validate dataset syntax), `d45b574` (multi-page dashboard).
- **Workspace:** resolver 3, v0.4.0, edition 2024, rust-version 1.85, `unsafe_code = "forbid"`, clippy "all warn". **22 crates** in `Cargo.toml` members. `crates/ps5-tests` exists (7 files) but is **not** a workspace member (referenced by consumers only).
- **Root binary** `src/main.rs` is a `Hello, world!` stub — not wired to anything.
- **Fuzzing:** `fuzz/fuzz_targets/` (root) holds all 6 targets (elf, self, nid, dynamic, relocations, shader); `crates/ps5-fuzz` is a separate member with `src/lib.rs` + 3 tests. The root `fuzz/` dir is outside the workspace.
- **Real corpus available:** 27 games under `ROMS/PS5`; 649 shader files under `Content/Shaders` (mostly `.sb`/pssl); 8 firmware export tables in `system_modules/*.exports.json` (libc, libSceFace, libSceFaceTracker, libSceFontGsm, libSceJobManager, libSceJobManager_nosubmission, libSceNpCppWebApi, libScePfs); 165,654 builtin NID entries reported by `scan`.
- **Artifacts present:** `analysis/` scan output (dataset.json, unknown_nids, catalog, images, load, reports, dashboard/index.html 4.46 MB single-file), `analysis_with_modules/`, empty scaffolds `data/{abi,firmware,nid,schemas,sdk,signatures}`.

## 2. Per-crate audit

| Crate (files, verified LOC, tests) | Verdict | Evidence / notes |
|---|---|---|
| `ps5-format` (5, 322, 12) | IMPLEMENTED | Shared constants, error enums, SHA-256. Clean core, no holdup. |
| `ps5-elf` (9, 2655, 74) | IMPLEMENTED | Zero-copy ELF64: header/program/dynamic/symbol/reloc/libversion; proptest. Real parser, no synthetic paths. |
| `ps5-self` (4, 1070, 24) | IMPLEMENTED | SELF→ELF extraction. |
| `ps5-nid` (5, 1002, 54) | IMPLEMENTED | Hash algorithm + `Catalog` (165K builtins, `add_builtins()`); `primary_name` ranking explicitly deferred (`catalog.rs:13-14`). |
| `ps5-image` (3, 2127, 28) | IMPLEMENTED | BinaryImage IR + JSON serialization. Parses ELF itself rather than consuming `ps5-prx` — migration incomplete. |
| `ps5-loader` (13, 3640, 87) | IMPLEMENTED (partial init) | 4-phase pipeline Map/RELATIVE/Export/Resolve (`pipeline.rs:16-22`), real RELATIVE fast path + ABS64/GLOB_DAT/JUMP_SLOT, missing PRX → `graph.mark_unavailable` + continue (`pipeline.rs:68-73, 176-180`), stub allocator base `0x0000_7fff_0000_0000` (`pipeline.rs:163`). **`.init_array`/`.preinit_array`/`DT_INIT` are never executed** — state flips to `Initialized` (`pipeline.rs:117, 225`). `LoadedModule` carries `prx_module: Option<ps5_prx::PrxModule>` (`mapper.rs:109`, built `mapper.rs:157-161`). |
| `ps5-emu` (28, 2944, 36) | IMPLEMENTED | Host-side execution, HLE Registry + `ExecutionReport`, boots fixture ELFs through the real loader via `elf_suite.rs`. ABI boundary only at `abi::sysv64`. |
| `ps5-ble-loader` / `ps5-memory-safe` | STUB / MINIMAL | Alternative safe-memory loader + MemoryManager — flagged stub in plan; not exercised by any consumer. |
| `ps5-deps` (3, 326, 8) | PARTIAL | Real `ModuleGraph` + `topological_sort` + `DepReport`; loaded by loader + `analyze graph`. **No `ps5rs deps` command** (UPGRADE_PLAN.md claimed `ps5rs deps --dot` done — it is not). |
| `ps5-signatures` (4, 587, 11) | PARTIAL | Engine detection real (weighted patterns, confidence = min(score,100), UNREAL4/5/UNITY/GODOT). `patterns.rs` SDK/third-party `Detection{score:0,confidence:0}` always — degenerate; middleware catalog (`classify_stem`) real. |
| `ps5-nid-db` (1, 336, 8) | PARTIAL | Real `NidRecord`/`NidDatabase` with `by_library`/`by_name` indexes + JSON persistence. **`from_catalog()` is a stub** (`let _ = catalog; Self::new()`), so the old 165K catalog cannot flow into the new DB. |
| `ps5-prx` (8, 261, 0 inline + 2 integration) | PARTIAL | Real `PrxModule` model + import/export/dependency/version extraction. `ps5-image` still parses ELF directly (no `PrxModule` consumption); loader stores a `PrxModule` but does not use it for init/fini. |
| `ps5-schema` (8, 68, 0 inline + 2 integration) | PARTIAL | `SCHEMA_VERSION=1` + typed snapshots (NidRecord/ModuleRecord/GameRecord/DependencyGraphSnapshot/ShaderRecord). **`BinaryImageDocument.image` is an untyped `serde_json::Value`** — not a tight schema. Not consumed as the dataset.json interchange format. |
| `ps5-abi` (8, 431, 3 inline + 1 integration) | PARTIAL | Types + `seed_signatures()` (~50 real-ish entries) + emu `default_registry` validates handlers. **Some signatures fabricated** (e.g. `free` → U32) without header verification. |
| `ps5-sdk-meta` (7, 531, 9 inline + 1 integration) | PARTIAL | `SdkDatabase` + loaders (`populate_from_stubs_dir`, `populate_from_nids_csv`), JSON. Data population depends on gitignored/proprietary assets — no committed data. |
| `ps5-firmware` (6, 332, 8) | PARTIAL | `FirmwareCatalog` + `populate_from_roms` (ELF SONAME tally) + JSON roundtrip. **`is_library_available()` always returns `true`** (`catalog.rs:106-115`) — firmware-aware matching is a placeholder. **Never reads `system_modules/*.exports.json`** (the 8 real tables are unused). |
| `ps5-shader` (8, 458, 14 inline + 2 integration) | SYNTHETIC | `shader_binary.rs` stage = byte-keyword sniff (`vertex`/`pixel`); `agc/mod.rs` `AgcShader{data}` is a stub; `disasm.rs` is a hexdump (not a disassembler); `resources.rs` is a text-line scan (`cbuffer`/`Texture`) with `default_sampler` fallback; `agsd.rs` minimal header+names. No real `.sb`/pssl analysis against the 649 real files. |
| `ps5-fuzz` (1, 52, 3) | PARTIAL | lib + 3 tests; actual fuzz targets live in root `fuzz/` — outside the workspace, not run by `just check`. |
| `ps5-analysis` (25, 6310, 165) | IMPLEMENTED | Real scanning/dataset/string fingerprinting; engine + middleware detection delegate to `ps5-signatures` (re-export one-liners). Reports, heatmaps, `export unknown` flows work. |
| `ps5-cli` (18, 5671, 18) | IMPLEMENTED (completeness gaps) | 18+ commands (inspect/imports/segments/dynamic/symbols/nid/scan/analyze/subcommands/extract/batch-extract/validate/dashboard/export-unknown/strings/exports/load/run/…). Gaps: no `deps`, no per-crate command surface for shader/firmware/sdk-meta/abi. `middleware` defaults to terminal output. |
| `ps5-dashboard` (4, 5167, 20) | IMPLEMENTED CORE + SYNTHETIC TABS | 13 tabs; overview/games/engines/libraries/nids/segments/statistics/graph/loader compute real derived data from `ds.images`. **Shader & firmware tabs are synthetic** (see §3). Deps (`Cargo.toml:8-13`) = ps5-analysis, ps5-image, ps5-nid only — the 6 new crates are invisible. Multi-page `generate_dashboard_pages` (index/shader/firmware/deps.html) = same single-file HTML with active-tab swap. |
| `ps5-tests` (7, 1557, 27) | IMPLEMENTED | Fixture generator (`bin/generate.rs`) + manifest + `elf_suite.rs` regression (real loader+HLE boots, byte-exact fixtures). Not a workspace member. |
| root `src/main.rs` | STUB | `Hello, world!` (`src/main.rs:1-3`). |

## 3. SYNTHETIC / INVALID (honesty-gate targets)

- `compute_shader_summary` `data.rs:737-759`: "vertex" = ELF executable segments (`data.rs:743`), "pixel" = libs containing `Agc`/`Gnm` strings (`data.rs:749`), `total_resources: total * 2` literal guess (`data.rs:758`). Rendered by shader tab `/shader.html` (`html.rs:1217-1234`).
- `compute_firmware_summary` `data.rs:762-779`: `total_modules = ds.images.len()` (game count mislabeled as modules, `data.rs:770`), `by_version` = lib-version-string tally (`data.rs:766`). Rendered by firmware tab (`html.rs:1236-1251`).
- SDK Timeline tab `html.rs:866-922`: a **hardcoded static table** (Prospero SDK/UE/EMC), not derived from `SdkDatabase`/`ps5-abi`.
- `sce_library_stats.module_count` always `0` (`data.rs:1641`) yet rendered as "Modules" (`html.rs:823`).
- `ps5-shader`: keyword sniffing + hexdump disasm + text-scan resources + `AgcShader` stub.
- `ps5-firmware::is_library_available` always `true` (`catalog.rs:106-115`).
- `ps5-abi` seed signatures unverified/fabricated (e.g. `free` → U32).
- `ps5-nid-db::from_catalog` ignores its input.
- `ps5-signatures::patterns` SDK/third-party detections always report score/confidence 0.
- `reports/middleware.json` (33,344 B) is **terminal text, not JSON** — update scripts call `ps5rs middleware` without `--format json`.
- `upgrade_plan_complete` computed `true` (`data.rs:731`) **never rendered** in JS.

## 4. UNCONNECTED

- `ps5-dashboard` does not depend on shader/firmware/sdk-meta/abi/deps/prx/nid-db (`Cargo.toml:8-13`); shader+firmware tabs show synthetic values; SDK tab hardcoded.
- `ps5-deps` / `ps5-firmware` / `ps5-sdk-meta` / `ps5-abi` / `ps5-shader` / `ps5-nid-db` have **no CLI surface**.
- `ps5-schema` not used as dataset.json interchange; dataset.json serializes `ps5-image` structs directly.
- `system_modules/*.exports.json` (8 real tables) unused by `ps5-firmware` and by NID resolution.
- `reports/deps.json`, `reports/graph.dot` written by scripts but consumed by nothing (dashboard recomputes its own graph edges from import data).
- `template.rs` — `pub const HTML_TEMPLATE` (`lib.rs:3`, `template.rs:1`) is **dead code**, a second parallel HTML template drifting from the inlined `generate_html` string (`html.rs:42`).
- Middleware: dashboard only injects it when `--games` is passed (`ps5-cli/src/dataset.rs:69-82`; `cli.rs:80-81` has no default). Scripts' dashboard call omits `--games` → middleware tab shows the empty-state message (`html.rs:277`).

## 5. STALE PLAN ITEMS (UPGRADE_PLAN.md §5 marked ✓ but not true)

- `ps5rs deps --dot` — only `analyze graph --format dot` exists; no `deps` command.
- `ps5-prx` "migrate BinaryImageBuilder" — `ps5-image` still parses ELF directly.
- `ps5-firmware` "feed from `system_modules/*.exports.json`" — uses `populate_from_roms`, exports tables unused.
- `ps5-shader` "from_roms walker" — keyword-sniffer, not shader analysis.
- Root `fuzz/` "corpus from ps5-tests, CI 60s" — targets exist but root fuzz dir is outside the workspace and not run by `just check`.
- Dashboard shader/firmware/SDK tabs deliver data the underlying crates are not wired to.

## 6. NEW REQUIREMENTS (UPGRADE_PLAN2.md mission, nothing exists yet)

1. **Evidence/provenance system** — every dashboard claim backed by data with source IDs (superset of NID sources, export tables, scan artifacts).
2. **NID knowledge layer** — per-NID pages, REmu cross-ref, `.exports.json` loaders, NID↔hex↔u64, export evidence, unknown-NID triage tool.
3. **Multi-signal detection** — engine/SDK/firmware from strings + build-id + lib_versions + exported symbols + shader data (today: strings only).
4. **Firmware-aware version matching** — game requires `libSceFoo ≥ v2.1` vs firmware's shipped v1.4 (`FirmwareCatalog` real availability check).
5. **Verified ABI signatures** — HLE handler validation against verified signatures, replacing fabricated seeds.
6. **Real shader analysis** — `.sb`/pssl stage + resource binding + hash correlation driving the shader tab (649 real files available).
7. **Dashboard IA** — per-game detail pages, per-library/per-NID drill-down, modularity breakdown, evidence badges, engine forensics; golden-file tests on real data.
8. **CLI completeness** — `ps5rs deps`; per-crate command surfaces; `--format json` correctness (middleware bug); honest empty states.
9. **Real-data arbiter** — `analysis/validation-report.{json,md}` produced per mission; verify 27-game corpus numbers against `index.html` meta.

## 7. RECOMMENDED EXECUTION ORDER (matches UPGRADE_PLAN2.md §23)

- **P0 — Honesty gate (~1d):** write this document (done); fix `middleware --format json` + `dashboard --games` in update scripts; replace synthetic shader/firmware/SDK output with honest "not wired" states or real data; fix `module_count=0`, `from_catalog` stub, zero-score patterns, `is_library_available`, fabricated ABI entries; remove/`#![allow(dead_code)]`-surface dead `template.rs`; wire root `main.rs`; document init-array-not-executed.
- **P1 — Evidence system + NID knowledge layer:** load `system_modules/*.exports.json` into `ps5-nid-db`; per-NID data model in `data/nid|sdk|firmware/`; provenance-tagged imports → dashboard evidence badges.
- **P2 — Real `ps5-shader`:** `.sb` (pssl) stage/entry/hash/resource analysis; drive the shader tab from real data (649-file corpus).
- **P3 — Firmware-aware matching + verified ABI:** real `FirmwareCatalog` availability; verified signatures replacing fabricated seeds.
- **P4 — Dashboard IA + goldens:** per-game detail, drill-down, golden tests over real scan output.
- **P5 — SDK-meta import + NID curation:** populate `ps5-sdk-meta` from stubs/CSV/Supabase with source + confidence.

## 8. Verification gates (uncompromised by this audit)

- `just check` = `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --workspace` + `git diff`.
- `cargo clippy --workspace --all-targets -D warnings` already green (`6ae64c1`).
- Fixture regression `elf_suite.rs` boots every `ps5-tests` fixture through the real loader + HLE.
- All counts in §2 measured from live source (files / verified LOC / inline tests) at branch `6ae64c1`.