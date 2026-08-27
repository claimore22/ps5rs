# ps5rs — UPGRADE_PLAN2.md (Rewrite)

> Mission: bring ps5rs from "architecture/crates implemented" to a fully implemented,
> integrated PS5 binary-analysis platform.
>
> The final product should feel like a PS5 binary research/analysis platform, not a
> collection of Rust crates. Optimize for how much useful information ps5rs can actually
> extract from a real PS5 game, and how much of that information the dashboard exposes to
> the researcher.
>
> Generated 2026-08-27 from live source inspection. Supersedes the prior plan-document-only
> revision; this version carries the full implementation mission below.

---

## 0. Hard Rules

Read `AGENTS.md`, `ROADMAP.md`, `UPGRADE_PLAN.md`, and existing crate implementations
before modifying code.

1. **Preserve existing working behavior.** The repo already contains substantial working
   functionality — inspect and extend, never rewrite working code.
2. **Prefer small, idiomatic Rust modules.**
3. **No `unwrap()`**, **no `expect()`**, **no `panic!()`** for normal error handling.
   Use `Result<T, E>` / `Option<T>`.
4. **Do not introduce unnecessary dependencies.**
5. **Do not create compatibility shims that hide incomplete implementation.**
6. **Do not mark a feature complete until it has real behavior and tests.**
7. **Existing data must remain backwards compatible where practical.**
8. **All new analysis information must ultimately be represented in structured data**,
   not only printed to stdout.
9. **The dashboard must consume the structured analysis data** rather than independently
   rediscovering information.
10. **Do NOT create empty crates, placeholder modules, fake implementations, TODO-only
    APIs, or superficial wrappers merely to satisfy the architecture.**
11. **The project is analysis-first.** Do not expand CPU emulation/HLE scope unless
    required to support analysis.

Every checklist item below means: *the feature must actually work against real
PS5 ELF/SELF/PRX data and be exposed through the CLI and/or dashboard where appropriate.*

---

## 1. First: Audit the Actual Implementation

Create **`IMPLEMENTATION_AUDIT.md`**. Do **not** assume `UPGRADE_PLAN.md` is accurate —
verify the code. For each crate, record: current implementation, implemented functionality,
partial functionality, stub/placeholder functionality, dead/unconnected functionality,
missing tests, missing CLI integration, missing dashboard integration, missing real-data
validation.

Crates to audit:
- `ps5-prx`, `ps5-schema`, `ps5-abi`, `ps5-sdk-meta`, `ps5-firmware`, `ps5-shader`,
  `ps5-deps`, `ps5-signatures`, `ps5-nid-db`, `ps5-fuzz`
- plus `ps5-analysis`, `ps5-image`, `ps5-cli`, `ps5-dashboard`, `ps5-loader`, `ps5-emu`,
  `ps5-tests`

### Known audit starting points (from prior inspection, re-verify)

| Area | Finding | Verify at |
|---|---|---|
| Shader dashboard | synthetic: segment-permission → "vertex", `Agc`/`Gnm` string → "pixel", `total_resources = total * 2` | `ps5-dashboard/src/data.rs:737-759`, `html.rs:1232` |
| Shader crate | keyword-sniffing stage detect, `DefaultHasher` "hash", fake resource walk | `ps5-shader/src/shader_binary.rs:55-72`, `resources.rs:31-58` |
| Firmware dashboard | counts `lib_versions` strings from scanned binaries, not a firmware catalog | `ps5-dashboard/src/data.rs:762-779` |
| SDK tab | hardcoded static HTML timeline, no `SdkDatabase` | `ps5-dashboard/src/html.rs:297-321` |
| Graph | game↔library import edges only; `ps5-deps::ModuleGraph` not surfaced | `ps5-dashboard/src/html.rs:1043-1049` |
| Middleware report | `analysis/reports/middleware.json` is terminal text (script lacks `--format json`); dashboard recomputes rather than consumes | `update_data.*`, `ps5-cli/src/dataset.rs:69-73` |
| `ps5-abi` | 50 seed signatures, some fabricated (e.g. `free` returning `U32`) — needs provenance gate | `ps5-abi/src/database.rs` |
| Authored-dataset wiring | `ps5-dashboard` depends on none of `ps5-shader`/`ps5-firmware`/`ps5-sdk-meta`/`ps5-abi`/`ps5-deps` | `crates/*/Cargo.toml` |

---

## 2. `ps5-prx` — Real Module Model

`ps5-prx` must become the authoritative module representation. Implement and verify:
`PrxModule`, module type detection, ELF metadata, SONAME, `DT_NEEDED`, `DT_SCE_NEEDED_LIB`,
imports, exports, NIDs, library versions, TLS metadata, entry point, build ID where
available, init/fini metadata where available, module dependencies, import/export
resolution state.

Integrate into: `ps5-image`, `ps5-loader`, `ps5-analysis`, `ps5-deps`, CLI.

No duplicated independent PRX parsing logic should remain unnecessarily scattered across
crates.

Tests: real ELF fixtures, real PRX samples if available, malformed-input tests,
import/export regression tests.

---

## 3. NID Database — Beyond Basic Lookup

`ps5-nid` remains the algorithm layer; **`ps5-nid-db` becomes the knowledge layer**.

Implement: NID, library, canonical name, aliases, source, confidence, SDK/version
information, tags, reverse lookup by name, lookup by library, lookup by NID, efficient
indexes, JSON persistence, merge semantics, duplicate/conflict handling.

Then add **evidence-based analysis classification**: system NID, SDK NID, engine NID,
middleware NID, unknown NID.

**Do not classify something as an engine NID merely because a string contains "Unreal."
Classification must be evidence-based** (see §16 Evidence System).

---

## 4. Engine Identification

Current detection is primarily string/fingerprint based. Upgrade to combine: **strings +
NIDs + libraries + module names + source paths + symbol names + known engine fingerprints +
version fingerprints**.

Support at minimum: Unreal Engine 4, Unreal Engine 5, Unity, Godot, native/custom engine,
unknown.

For Unreal specifically: UE4 vs UE5; likely minor version **where evidence exists**;
engine fork/customization evidence; engine source paths; engine plugin paths; engine NID
evidence; confidence score; evidence list.

Structured result (target shape):

```json
{
  "engine": "Unreal Engine",
  "version": "5.3",
  "confidence": 0.91,
  "evidence": [
    "Engine/Source/Runtime",
    "Engine/Plugins",
    "FName",
    "UObject",
    "UE5-specific fingerprint"
  ]
}
```

**Do not invent exact engine versions when the evidence only supports a major version.
Represent uncertainty explicitly.**

---

## 5. SDK Identification

`ps5-sdk-meta` must become useful against real binaries.

Implement: `SdkFunction`, SDK version range, library, module, category, aliases,
confidence, source, structures, constants, function metadata.

Import knowledge from existing project data where legally/technically appropriate
(`analysis/catalog/nids.csv`, `data/nid/catalog.json`, `.a` stubs via
`ps5rs catalog import-stubs` / `dump-stubs`).

Analysis must answer:
- Which SDK functions does this game use? Which libraries? Which SDK generation?
- Which functions are unresolved? Which SDK version range is most likely?

Add: SDK report, SDK statistics, SDK/library heatmap, unresolved SDK NID report.

---

## 6. Firmware Analysis

`ps5-firmware` must not merely contain types. Implement real catalog functionality:
firmware version, modules, libraries, exports, versions, NIDs, source metadata, catalog
loading, catalog comparison.

Source real data: `system_modules/*.exports.json` (8 modules present) + game ROM data.

Analysis must answer:

```
Game requires:  libSceFoo  version X
Firmware provides: libSceFoo version Y
Compatibility:  compatible / unknown / insufficient evidence
```

Add a firmware compatibility report, plus dashboard surfaces: firmware version, required
libraries, available libraries, missing libraries, version mismatches, unresolved exports.

---

## 7. Dependency Graph

`ps5-deps` must represent the complete dependency graph. Include: game, eboot, PRX, SPRX,
system libraries, imported libraries, exported libraries, unresolved dependencies,
dependency edges, aliases, load order.

CLI: `ps5rs deps`, `ps5rs deps --json`, `ps5rs deps --dot`.

Dashboard: interactive dependency graph, module → module, module → system library,
unresolved nodes, click-through to module details.

---

## 8. ABI Database

`ps5-abi` must be more than 50 placeholder signatures. Implement the metadata model:
primitive types, pointers, structs, arrays, callbacks, register classes, calling
conventions, return values, variadic functions, structure layouts.

Then: populate real known signatures **where evidence exists**, attach signatures to
`SdkFunction`, validate HLE registrations, report signature mismatches.

**Unknown signatures must remain explicitly unknown. Never fabricate signatures** — audit
existing `seed_signatures()` for invented entries (e.g. `free` returning `U32`).

---

## 9. Shader Analysis

`ps5-shader` must produce useful information from actual shader artifacts. Implement
incrementally but fully for each implemented format:

- `.ags`, `.agsd`, shader metadata, stage, entry point, shader hash, resources, samplers,
  textures, buffers, render targets, vertex attributes, reflection, debug information
  where available, disassembly where feasible.

**Do not create a fake disassembler.** If a format cannot yet be completely decoded,
represent the undecoded portions **structurally** and document exactly what remains
undecoded.

Replace the keyword-sniffing stage detection and `DefaultHasher` "hash" with real blob
parsing. Corpus: `.sb`/`.ags`/`.agsd` under `ROMs/**/Content/Shaders/` (649 shader files
present).

Dashboard: shader count, shader stages, shader hashes, resource usage, shader metadata,
`.agsd` debug information, per-game shader inventory.

---

## 10. Schema

`ps5-schema` must become the stable interchange layer. Centralize: schema version, binary
image schema, game schema, module schema, NID schema, dependency schema, shader schema,
SDK schema, firmware schema.

Add: JSON Schema files, validation tests, backward-compatibility tests, migration tests.

**The dashboard should consume stable schema objects rather than internal implementation
structs.**

---

## 11. Dashboard — Major Upgrade

Create a dashboard information architecture:

```
Overview
│
├── Games
│   └── Game Detail
├── Modules
│   └── PRX Detail
├── NIDs
│   ├── Resolved
│   ├── Unknown
│   ├── Engine
│   └── SDK
├── Libraries
│   └── Library Detail
├── Dependencies
│   └── Graph
├── Engines
│   └── Engine + Version
├── Middleware
├── SDK
│   └── SDK usage
├── Firmware
│   └── Compatibility
├── Shaders
│   └── Shader inventory
└── Analysis / Evidence
```

---

## 12. Dashboard Overview

Add real aggregate statistics: number of games, number of binaries, number of PRX/SPRX
modules, total imports, resolved imports, unresolved imports, resolution percentage,
unique NIDs, unknown NIDs, libraries, engines, engine versions, middleware, SDK
generations, firmware versions, shaders, dependency edges.

---

## 13. Game Detail Page

For every game show:

- **Identity**: title, title ID, path, hashes, binary count
- **Engine**: engine, version, confidence, evidence
- **SDK**: likely SDK, SDK evidence, libraries, functions
- **NIDs**: total, resolved, unresolved, engine NIDs, SDK NIDs, system NIDs
- **Modules**: eboot, PRX, SPRX, module dependencies
- **Middleware**: Wwise, FMOD, PhysX, Oodle, etc.
- **Shaders**: shader count, stages, resources
- **Firmware**: required libraries, compatibility, missing/version mismatches

---

## 14. Unknown NID Analysis

Create an Unknown NID page/report. For every unknown NID show: NID, library,
occurrences, games, modules, possible aliases, nearby strings, possible engine, possible
SDK, possible middleware, confidence, evidence.

Allow sorting by: frequency, library, game count, confidence, likely category.

**This should become a major research tool for discovering unknown PS5 NIDs.**

---

## 15. NID Heatmap

Implement the NID/library/game analysis the project is intended to support:

```
              Game A Game B Game C Game D
libSceFoo        ██     ██     ░      ██
libSceBar        ░      ██     ██     ░
libSceBaz        ██     ░      ██     ██
```

Support: game × library, game × NID, library × NID, resolved vs unresolved,
engine-specific NIDs.

---

## 16. Evidence System

Every important detection must answer: **why did ps5rs reach this conclusion?**

For: engine, engine version, SDK, middleware, firmware, NID classification, shader
classification — store structured evidence:

```rust
pub struct Detection {
    pub value: String,
    pub confidence: f64,
    pub evidence: Vec<Evidence>,
}
```

The dashboard must display this evidence. **Do not make the analysis a black box.**

---

## 17. CLI Completeness

Every major subsystem needs a useful command. Expected capabilities:
`scan`, `inspect`, `imports`, `strings`, `nid`, `deps`, `exports`, `catalog`,
`middleware`, `validate`, `analyze`, `dashboard`, `shader`, `firmware`, `sdk`.

For every command: useful real output, JSON output where appropriate, errors are
actionable, no panic, documented in `--help`, tested.

(`shader`, `firmware`, `sdk` subcommands do not yet exist — add them.)

---

## 18. Analysis Pipeline

The complete pipeline:

```
PS5 dump
   ▼
SELF parser
   ▼
ELF parser
   ▼
PRX/module extraction
   ├── NID analysis
   ├── library analysis
   ├── dependency analysis
   ├── strings
   ├── engine detection
   ├── SDK detection
   ├── middleware detection
   ├── firmware requirements
   └── shader discovery
   ▼
Stable schema
   ▼
Dataset
   ├── reports
   ├── JSON
   └── dashboard
```

**There must not be major information discovered by one subsystem that disappears before
reaching the dataset/dashboard.**

---

## 19. Real Data Validation

Do not validate only against synthetic fixtures. Run the complete pipeline against the
user's real PS5 dataset (`C:\Users\claimoar\Documents\ROMS\PS5`).

Measure: games scanned, binaries scanned, PRX modules, total imports, resolved imports,
unresolved imports, unique NIDs, unknown NIDs, libraries, engines, engine versions,
middleware, SDK detections, firmware detections, shaders, dependencies.

Produce `analysis/validation-report.json` and `analysis/validation-report.md`.

---

## 20. Testing

Minimum: unit tests, integration tests, malformed-input tests, schema tests, CLI tests,
dashboard generation tests, real-data smoke tests where practical, fuzz targets.

Run:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
cargo fmt --check
```

Fix all regressions.

---

## 21. Definition of Done

**Do NOT mark this project complete because**: crate exists, struct exists, function
exists, CLI command exists, JSON file exists.

A feature is complete only when it is:

- implemented
- **+ integrated**
- **+ tested**
- **+ used by real analysis**
- **+ represented in dataset/schema**
- **+ visible through CLI/dashboard where applicable**
- **+ documented**

---

## 22. Final Checklist

At the end, update `UPGRADE_PLAN.md` so every item accurately reflects reality:

- [ ] `ps5-prx` fully implemented
- [ ] `ps5-nid-db` fully implemented
- [ ] engine NID detection
- [ ] engine version detection
- [ ] SDK metadata fully integrated
- [ ] ABI signatures actually populated/validated
- [ ] firmware compatibility implemented
- [ ] dependency analysis complete
- [ ] shader analysis useful on real artifacts
- [ ] schema validation/migrations
- [ ] fuzzing
- [ ] evidence system
- [ ] CLI complete
- [ ] dashboard complete
- [ ] unknown NID research interface
- [ ] NID heatmaps
- [ ] library heatmaps
- [ ] game detail
- [ ] module detail
- [ ] shader detail
- [ ] firmware detail
- [ ] SDK detail
- [ ] real dataset validation
- [ ] workspace tests passing
- [ ] clippy clean
- [ ] formatting clean

---

## 23. Execution Phases

Work must be incremental, each phase gated on the previous, so working behavior is always
preserved.

| Phase | Scope | Exit criteria |
|---|---|---|
| 0 | **Audit** — write `IMPLEMENTATION_AUDIT.md`; rewire dashboard "honesty gate" (kill synthetic shader/firmware numbers, fix script `--format json` + `--games`) | Audit doc complete; no derived-as-fact renders; `just check` green |
| 1 | **Knowledge plumbing** — `ps5-nid-db` evidence classification, `ps5-sdk-meta` real data, `ps5-firmware` catalog from `system_modules/*.exports.json` + ROMs, `ps5-abi` provenance gate | knowledge crates feed indexed queries; seed data loads from real files |
| 2 | **Binary depth** — `ps5-prx` authoritative module model into `ps5-image`/`ps5-loader`/`ps5-analysis`/`ps5-deps`/CLI; `ps5-deps` graph v2; real shader blob parsing | PRX/ELF/SELF drill-downs live; shader info real; deps typed edges |
| 3 | **Detection upgrade** — multi-signal engine + SDK + middleware detection with `Detection{value,confidence,evidence}`; Unknown NID report on real corpus | evidence chains visible; unknowns sorted by frequency/library/engine |
| 4 | **Dashboard v2** — full IA (§11), overview stats (§12), game/module/NID/library detail pages(§13), heatmaps (§15), shader/firmware/SDK tabs fed by real ingest, evidence display | every tab consumes structured schema objects; no independent rediscovery |
| 5 | **Validation & hardening** — real-dataset validation reports (§19), schema/migration tests, fuzz targets, CLI completeness (§17), regression fixes | `analysis/validation-report.{json,md}`; `cargo test`/`clippy --all-targets --all-features`/`fmt` clean |

---

## 24. Non-Goals (deferred)

- Full GCN/AGC ISA disassembly — research-tier; represent undecoded portions structurally
  and document what remains.
- CPU execution/HLE beyond the `ps5-emu` Registry (TLS block allocation, `.init_array`
  execution, full `libkernel`) — analysis-first per hard rules.
- No SQLite/external DB; JSON-only persistence; self-contained dashboard, `file://`
  compatible, no server, no CDN.
- No Sony SDK name leakage into committed strings (grep gate: `sony.*sdk`).

---

*See also: `AGENTS.md` (conventions), `ROADMAP.md` (loader phases 1–6), `UPGRADE_PLAN.md`
(crate rollout), `IMPLEMENTATION_AUDIT.md` (Phase 0 deliverable), `crates/ps5-*/src/lib.rs`.*