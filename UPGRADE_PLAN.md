# ps5rs Upgrade Plan — 10 Proposed Crates

> Gap analysis of the 10 proposed crates against the current 13-crate workspace.
> Generated 2026-08-25 from live source inspection (`Cargo.toml`, `crates/*/src/**/*.rs`).

## 0. Current Workspace Inventory

```
ps5-format        shared types, constants, SHA-256, error enums
ps5-self          SELF container parser (PS4/PS5 → ELF extraction)
ps5-elf           zero-copy ELF64 parser (header/program/dynamic/symbol/reloc/libversion)
ps5-nid           NID algorithm + Catalog (HashMap<String,NidEntry>)
ps5-image         BinaryImage IR + JSON serialization + BinaryImageBuilder
ps5-analysis      scanner, dataset, string fingerprinting, engine/middleware detection
ps5-loader        virtual loader 4-phase pipeline + ModuleGraph + 3-tier resolver
ps5-emu           host-side emulator (native x86-64 + HLE Registry)
ps5-ble-loader    alternative safe-memory loader (stub)
ps5-memory-safe   safe MemoryManager abstraction
ps5-dashboard     static HTML dashboard
ps5-cli           ps5rs binary (18 subcommands)
ps5-tests         ELF fixture generator + manifest
```

Dependency direction: `format → {elf,self,nid} → image → analysis → cli`, `loader → elf`, `ps5-emu → {loader,elf,nid,format}`.

No directory `ps5-prx`, `ps5-sdk-meta`, `ps5-abi`, `ps5-shader`, `ps5-firmware`, `ps5-deps`, `ps5-signatures`, `ps5-fuzz`, `ps5-schema` exists.
`Test-Path fuzz` → `False`. `crates/` glob confirms `0/10` proposed crates exist.

---

## 1. Gap Matrix

| # | Proposed crate | Exists? | Coverage today (scattered) | Verdict |
|---|---|---|---|---|
| 1 | **ps5-prx** `module.rs/metadata.rs/dependencies.rs/exports.rs/imports.rs/versions.rs/error.rs` | No | `ps5-elf/src/dynamic.rs:42` → `DT_SCE_NEEDED_LIB/DT_NEEDED/DT_SONAME`, `ps5-elf/src/libversion.rs`, `ps5-image/src/lib.rs:645` `LibVersionEntry`, `ps5-image/src/builder.rs` (`imports/exports/needed_files/import_libs`), `ps5-loader/src/mapper.rs:25` `ModuleType::Prx/Eboot`, `ModuleState`, `LoadedModule` | **Refactor+promote** — logic exists but no unified `PrxModule` type |
| 2 | **ps5-sdk-meta** `libraries/functions/versions/structures/constants/database` `SdkFunction{nid,name,library,sdk_versions,module,category}` | No | `ps5-nid/src/catalog.rs:24` `Catalog{by_nid: HashMap<String,NidEntry>}` builtins `catalog.rs:171` (~100 names), `ps5-analysis/src/string_patterns.rs:240` SDK hints | **Greenfield DB** — no `SdkFunction`, no version ranges, no SDK structures |
| 3 | **ps5-nid-db** `NidRecord{nid,library,name,versions,source,confidence}` | No | `ps5-nid/src/catalog.rs:5` `NidEntry{names,libraries,tags,sources: BTreeSet}` + `insert()` `catalog.rs:49`, 5-col rich CSV `catalog.rs:107` | **Promote catalog** — needs typed `LibraryId`, `VersionRange`, `Confidence`, `NidSource` |
| 4 | **ps5-abi** `calling_convention.rs/types.rs/functions.rs/structs.rs/callbacks.rs/layouts.rs` `FunctionSignature/StructLayout` | No | `ps5-emu/src/abi/mod.rs:1` + `abi/sysv64.rs` (execution only: `EscapeContext/ImportCallFrame/invoke_guest/escape`) | **Greenfield metadata** — no signature store for HLE |
| 5 | **ps5-shader** `agc/shader_binary.rs/shader_metadata.rs/agsd.rs/disasm.rs/resources.rs/reflection.rs` | No | Zero hits (`*.ags*`, GPU ISA) | **Greenfield** — highest research value |
| 6 | **ps5-firmware** `version.rs/modules.rs/libraries.rs/exports.rs/catalog.rs` | No | `system_modules/` offline exports (gitignored) but no typed model | **Greenfield** — system software metadata |
| 7 | **ps5-deps** `DependencyGraph{modules,edges}` `ps5rs deps --dot` | No crate, logic exists | `ps5-loader/src/graph.rs:15` `ModuleGraph{nodes,edges,aliases,unavailable}` + `topological_sort()` `graph.rs:106`, `ps5-loader/src/resolver.rs` `CrossModuleResolver`, CLI `ps5-cli/src/cli.rs:273` `analyze graph --format dot` | **Extract** — graph already reusable, needs standalone crate + `ps5rs deps` CLI |
| 8 | **ps5-signatures** engine/middleware/SDK/compiler detection | No crate, logic exists | `ps5-analysis/src/engine_fingerprints.rs:33` `UNREAL4/UNREAL5/UNITY/GODOT`, `ps5-analysis/src/string_patterns.rs:51` `detect_third_party()` (PhysX/Oodle/FMOD/Wwise/ICU), `ps5-analysis/src/middleware.rs:172` `CATALOG` + `classify_stem()` `middleware.rs:226` | **Extract** — move reusable patterns to dedicated crate |
| 9 | **ps5-fuzz** `fuzz/elf.rs/self.rs/nid.rs/dynamic.rs/relocations.rs/shader.rs` | No | Inline `#[cfg(test)]` + `ps5-tests` fixtures only | **Greenfield** — `cargo fuzz` targets for parsers |
| 10 | **ps5-schema** `BinaryImage/NidRecord/LibraryRecord/ModuleRecord/SdkFunction/DependencyGraph` stable schemas | No | `ps5-image/src/lib.rs:72` `BinaryImageDocument{schema_version,tool,image}` `BINARY_IMAGE_VERSION`, `ps5-analysis/src/dataset.rs` `DATASET_SCHEMA_VERSION` — versioned but tied to structs | **Greenfield** — decouple JSON interchange from internal structs |

**Summary:** `0/10` exist as named crates. `3` have substantial logic ready to extract (`ps5-deps`, `ps5-signatures`, `ps5-nid-db` promotion). `3` are refactors of scattered code (`ps5-prx`, `ps5-abi`, `ps5-schema`). `4` are greenfield (`ps5-sdk-meta`, `ps5-shader`, `ps5-firmware`, `ps5-fuzz`).

---

## 2. Target Architecture

```
                            ps5-schema  (stable JSON schemas, versioned)
                                ▲
                                │
ps5-format ─┬─► ps5-elf ────────┼─► ps5-prx ──► ps5-deps ──┐
            │   ps5-self ───────┘              ▲            │
            │   ps5-nid ──► ps5-nid-db ────────┘            │
            │                │                               │
            │                ▼                               ▼
            │            ps5-sdk-meta ──► ps5-abi ──► ps5-emu (HLE Registry)
            │                │          ▲
            └────────────────┼──────────┘
                             ▼
                     ps5-signatures ──► ps5-analysis ──► ps5-cli / ps5-dashboard
                             ▲
                     ps5-shader ─┘
                     ps5-firmware ─► ps5-image ─► ps5-loader ─┘
                     ps5-fuzz (dev-only, targets all parsers)
```

Dependency rule: `ps5-schema` has **no** dependencies (pure `serde` types). `ps5-nid` stays algorithm-only; `ps5-nid-db` depends on `ps5-nid` + `ps5-schema`. `ps5-prx` depends on `ps5-elf` + `ps5-nid-db`. `ps5-abi` depends on `ps5-nid-db` + `ps5-sdk-meta`. No cycles.

---

## 3. Crate Designs

### 3.1 `ps5-prx` — PRX / Module Intelligence

**Purpose:** First-class model of a PS5 module beyond raw ELF. Feeds `ps5-loader`, `ps5-deps`, `ps5-analysis`, and BLEPS5.

**Proposed layout (matches request):**
```
ps5-prx/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── module.rs       # PrxModule { name, module_type, metadata, tls, init/fini }
│   ├── metadata.rs     # PrxMetadata { soname, needed_files, import_libs, build_id, elf_type/flags }
│   ├── dependencies.rs # dependency extraction (DT_NEEDED + DT_SCE_NEEDED_LIB)
│   ├── exports.rs      # ExportSet { entries: Vec<ExportEntry> } + by-NID index
│   ├── imports.rs      # ImportSet { entries: Vec<ImportEntry> } + unresolved tracking
│   ├── versions.rs     # LibVersionSet (wraps ps5-elf/libversion.rs + ps5-format constants)
│   └── error.rs        # PrxError
```

**Key types:**
```rust
pub enum ModuleType { Eboot, Prx, Sprx, SelfModule, Unknown }
pub struct PrxModule {
    pub name: String,
    pub module_type: ModuleType,
    pub metadata: PrxMetadata,
    pub imports: ImportSet,
    pub exports: ExportSet,
    pub dependencies: Vec<Dependency>, // resolved via dynamic.rs today
    pub lib_versions: LibVersionSet,
    pub tls: Option<TlsInfo>,
    pub entry_point: u64,
}
```

**Migration:** Move `BinaryImageBuilder` import/export/needed logic from `ps5-image/src/builder.rs` + `ps5-elf/src/dynamic.rs:42` + `ps5-elf/src/libversion.rs` into `ps5-prx`. `ps5-image` then **consumes** `ps5-prx::PrxModule` instead of parsing ELF itself. `ps5-loader/src/mapper.rs` `LoadedModule` delegates to `PrxModule`.

**Effort:** ~8h (refactor, no new parsing).

### 3.2 `ps5-sdk-meta` — SDK Knowledge Database

**Purpose:** Metadata *about* the SDK, not the SDK itself. Maps `NID → SdkFunction` with version ranges.

```
ps5-sdk-meta/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── libraries/      # libSce* definitions, ModuleKind
│   ├── functions/      # SdkFunction + category
│   ├── versions/       # VersionRange, SdkVersion (9.00–11.00 etc.)
│   ├── structures/     # known struct layouts (sceKernel* types)
│   ├── constants/      # error codes, flags
│   └── database/       # Database { functions: HashMap<Nid, SdkFunction> } + loaders
```

```rust
pub struct SdkFunction {
    pub nid: String,          // hex or Sony base64
    pub name: String,         // sceSomething
    pub library: String,      // libSceFoo
    pub module: Option<String>, // e.g. libkernel.prx
    pub sdk_versions: VersionRange, // 9.00–11.00
    pub category: String,     // filesystem, audio, net, etc.
    pub signature: Option<FunctionSignature>, // via ps5-abi later
}
```

**Data sources:** `analysis/catalog/nids.csv`, Supabase dumps, `*.a` via `ps5rs catalog import-stubs` `crates/ps5-cli/src/catalog.rs`, leaked SDK headers (metadata only).

**Effort:** ~12h + ongoing curation. Start with schema + import from existing `Catalog`.

### 3.3 `ps5-nid-db` — Richer NID Catalog (promote `ps5-nid`)

**Purpose:** Separate algorithm (`ps5-nid`) from knowledge DB.

**Split:**
- `ps5-nid` stays: `hash()`, `nid_to_u64()`, `algorithm.rs`, `lookup.rs` (no DB)
- `ps5-nid-db` new: `NidRecord`, `LibraryId`, `NidSource`, `Confidence`, persistence

```rust
pub enum NidSource { Builtin, SdkStub, Supabase, Manual, REmuCrossRef }
pub enum Confidence { Verified, High, Medium, Low, Unknown }
pub struct NidRecord {
    pub nid: String,
    pub library: LibraryId,          // typed, not String
    pub name: Option<String>,        // None = unknown
    pub versions: Option<VersionRange>,
    pub source: NidSource,
    pub confidence: Confidence,
    pub aliases: BTreeSet<String>,   // multiple names per NID
}
pub struct NidDatabase { records: HashMap<String, NidRecord> }
```

**Migration:** Replace `ps5-nid/src/catalog.rs:24` `Catalog{by_nid: HashMap<String,NidEntry>}` with `NidDatabase` (or keep `Catalog` as alias). Add `VersionRange` field, `Confidence` enum. Keep CSV compat (`catalog.rs:107` rich 5-col). `ps5-nid` re-exports DB via feature flag or downstream depends on `ps5-nid-db`.

**Effort:** ~6h.

### 3.4 `ps5-abi` — ABI Metadata

**Purpose:** Describe calling conventions + signatures *declaratively*, separate from `ps5-emu/src/abi/sysv64.rs` execution trampolines.

```
ps5-abi/
├── calling_convention.rs  # SysV64, Sony custom
├── types.rs               # AbiType { U32, U64, Ptr<T>, Struct, Callback }
├── functions.rs           # FunctionSignature { return_type, params, convention, variadic }
├── structs.rs             # StructLayout { size, align, fields: Vec<Field> }
├── callbacks.rs           # CallbackSignature
└── layouts.rs             # Known PS5 struct layouts (SceKernel* etc.)
```

```rust
pub struct FunctionSignature {
    pub return_type: AbiType,
    pub params: Vec<Param>, // Param { name, ty, reg_class }
    pub calling_convention: CallingConvention, // SysV64
    pub variadic: bool,
}
pub struct StructLayout { pub size: usize, pub align: usize, pub fields: Vec<Field> }
```

**Integration:** `ps5-sdk-meta::SdkFunction.signature: Option<FunctionSignature>`; `ps5-emu` HLE `Registry` validates handlers against stored signatures; generated stubs can be checked at `cargo test`.

**Effort:** ~10h (types + ~50 initial signatures).

### 3.5 `ps5-shader` — Shader Analysis

**Purpose:** First-class shader crate fed by `ps5-elf`/`ps5-self` (shader segments are ELF payloads).

```
ps5-shader/
├── agc/               # AGC (AMD GCN) specifics
├── shader_binary.rs   # ShaderBinary { stage, isa, hash }
├── shader_metadata.rs # ShaderMetadata { stage, entry_point, num_threads }
├── agsd.rs            # .agsd debug container
├── disasm.rs          # GCN disassembler (or wrapper)
├── resources.rs       # ResourceBinding { type, slot, name }
└── reflection.rs      # Reflection { vertex_attributes, render_targets, samplers }
```

**Why now:** User notes existing shader work scattered; ELF/SELF pipeline already extracts shader blobs but has no typed consumer. Start with `.ags`/`.agsd` parsing + resource reflection; defer full ISA disasm.

**Effort:** ~16h initial, then incremental per shader stage.

### 3.6 `ps5-firmware` — System Software Metadata

**Purpose:** Structured view of *which* libraries/modules a firmware version ships, not the firmware itself.

```
ps5-firmware/
├── version.rs    # FirmwareVersion { major, minor, patch } + semver
├── modules.rs    # FirmwareModule { name, path, version, exports_count }
├── libraries.rs  # FirmwareLibrary { name, version, modules: Vec<String> }
├── exports.rs    # FirmwareExportTable (merged system_modules/*.exports.json)
└── catalog.rs    # FirmwareCatalog { version, modules, libraries } + loader
```

Lets `ps5-analysis` answer: “Game requires `libSceFoo` v2.1, SDK 10.xx, but firmware 9.00 only ships v1.4”.

**Effort:** ~6h.

### 3.7 `ps5-deps` — Dependency Analysis Abstraction

**Purpose:** Promote `ps5-loader/src/graph.rs` to reusable crate.

```
ps5-deps/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── graph.rs      # DependencyGraph (from loader/graph.rs:15) + topological_sort
│   ├── resolver.rs   # CrossModuleResolver wrapper
│   └── report.rs     # DepReport { edges, missing, load_order }
```

Already has `ModuleGraph{nodes,edges,aliases,unavailable}` `graph.rs:15`, `DependencyEdge{from,to}` `graph.rs:4`, `topological_sort()` `graph.rs:106`. Add `ps5rs deps <game/>` and `ps5rs deps --dot/--json` (currently `ps5rs analyze graph` `cli.rs:273`).

**Migration:** `ps5-loader` depends on `ps5-deps`; `ps5-analysis` and `ps5-cli` consume it. No behavior change.

**Effort:** ~3h.

### 3.8 `ps5-signatures` — Reusable Signature Definitions

**Purpose:** Extract detection patterns from `ps5-analysis` into a `no_std`-friendly data crate.

```
ps5-signatures/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs      # EngineFingerprint (from engine_fingerprints.rs:33)
│   ├── middleware.rs  # MiddlewareId catalog (from middleware.rs:172 CATALOG)
│   ├── sdk.rs         # SDK generation hints
│   ├── compiler.rs    # toolchain fingerprints
│   └── patterns.rs    # generic Pattern { needle, weight, category }
```

Current sources:
- `ps5-analysis/src/engine_fingerprints.rs:33` `UNREAL4/UNREAL5/UNITY/GODOT` + `detect_engine()` `engine_fingerprints.rs:86`
- `ps5-analysis/src/string_patterns.rs:51` `detect_third_party()` + `detect_sdk_hints()` `string_patterns.rs:240`
- `ps5-analysis/src/middleware.rs:172` `CATALOG` + `classify_stem()` `middleware.rs:226`
- `ps5-analysis/src/string_patterns.rs:303` `StringAnalysis` aggregator

`ps5-analysis` then becomes orchestrator; `ps5-signatures` holds data. `ps5-cli` can expose `ps5rs signatures list`.

**Effort:** ~5h.

### 3.9 `ps5-fuzz` — Systematic Parser Fuzzing

**Purpose:** Cover `SELF → ELF`, `ELF → BinaryImage`, `ELF → loader` — the attack surface for malformed binaries.

```
fuzz/
├── Cargo.toml
├── fuzz_targets/
│   ├── elf.rs         # parse_elf_bytes() + ps5-elf/src/header.rs
│   ├── self.rs        # SelfImage::parse() + ps5-self/src/lib.rs
│   ├── nid.rs         # hash() + catalog lookup
│   ├── dynamic.rs     # parse_dynamic() + ps5-elf/src/dynamic.rs
│   ├── relocations.rs # apply_relocations() + ps5-loader/src/relocation.rs
│   └── shader.rs      # (once ps5-shader exists)
└── corpus/            # seeded from data/test/generated_elfs/
```

Uses `cargo fuzz` (`libFuzzer`) + `cargo-afl` alternative. Seeds from `ps5-tests` generated ELFs `crates/ps5-tests/src/elf.rs`. CI runs `cargo fuzz run <target> -- -max_total_time=60`.

**Effort:** ~4h setup + ongoing corpus growth.

### 3.10 `ps5-schema` — Stable Interchange Schemas

**Purpose:** Decouple JSON dataset evolution from internal Rust structs. One place for schema versioning.

```
ps5-schema/
├── Cargo.toml
├── src/
│   ├── lib.rs         # re-exports + SCHEMA_VERSION
│   ├── binary_image.rs # BinaryImageDocument (from ps5-image/src/lib.rs:72)
│   ├── nid_record.rs   # NidRecord (from ps5-nid-db)
│   ├── module_record.rs# ModuleRecord (from ps5-prx)
│   ├── dependency_graph.rs # DependencyGraph snapshot
│   ├── shader_record.rs# ShaderRecord (from ps5-shader)
│   └── game_record.rs  # GameRecord { title_id, modules, middleware, engine }
├── schemas/           # JSON Schema files (for external consumers)
│   ├── binary_image.v1.json
│   └── game_record.v1.json
└── migrations/        # v1→v2 converters (serde default + custom)
```

Current versioning: `ps5-image/src/lib.rs:10` `BINARY_IMAGE_VERSION`, `ps5-image/src/lib.rs:72` `BinaryImageDocument{schema_version}`, `ps5-analysis/src/dataset.rs` `DATASET_SCHEMA_VERSION`. Move those constants here; other crates depend on `ps5-schema` for `Serialize/Deserialize` derives. Add `#[serde(default)]` + `skip_serializing_if` already used `ps5-image/src/lib.rs:79` for backward compat.

**Effort:** ~8h.

---

## 4. Phased Rollout

### Phase A — Zero-Cost Extractions (no new data, just moves)

| Step | Crate | Action | Depends on | Effort |
|---|---|---|---|---|
| A1 | `ps5-deps` | Move `ps5-loader/src/graph.rs` + `resolver.rs` to `ps5-deps`, add `ps5rs deps` CLI alias | — | 3h |
| A2 | `ps5-signatures` | Extract `engine_fingerprints.rs` + `string_patterns.rs` patterns + `middleware.rs:172` CATALOG | — | 5h |
| A3 | `ps5-nid-db` | Promote `ps5-nid/src/catalog.rs` to `ps5-nid-db` with `NidRecord`/`Confidence` | — | 6h |
| | **Phase A total** | | | **~14h** |

`ps5-loader`, `ps5-analysis`, `ps5-emu` update `Cargo.toml` dependencies. No schema break.

### Phase B — Structural Refactors (wrap existing parsing)

| Step | Crate | Action | Depends on | Effort |
|---|---|---|---|---|
| B1 | `ps5-prx` | Create `PrxModule` wrapping `ps5-elf` + `ps5-nid-db`, migrate `ps5-image/src/builder.rs` | A3 | 8h |
| B2 | `ps5-schema` | Extract `BinaryImageDocument`/`BinaryMetadata` + dataset schemas, add `schemas/*.json` | B1, A3 | 8h |
| B3 | `ps5-abi` | Define `FunctionSignature`/`StructLayout`, seed 50 SDK funcs, wire to `ps5-sdk-meta` + `ps5-emu` Registry | A3 | 10h |
| | **Phase B total** | | | **~26h** |

`ps5-image` now depends on `ps5-prx` + `ps5-schema`; `ps5-emu` validates HLE handlers against `ps5-abi`.

### Phase C — New Knowledge Bases

| Step | Crate | Action | Depends on | Effort |
|---|---|---|---|---|
| C1 | `ps5-sdk-meta` | Create `SdkFunction` DB, import `analysis/catalog/nids.csv` + `catalog import-stubs` | A3, B2 | 12h |
| C2 | `ps5-firmware` | Model firmware versions + module catalog, feed `system_modules/` | B1, B2 | 6h |
| | **Phase C total** | | | **~18h** |

Populate `ps5-sdk-meta` from `data/test/generated_elfs` + Supabase; `ps5-firmware` from `system_modules/*.exports.json`.

### Phase D — New Domains + Tooling

| Step | Crate | Action | Depends on | Effort |
|---|---|---|---|---|
| D1 | `ps5-shader` | `.ags`/`.agsd` + resource reflection + stub disasm | B1, B2 | 16h |
| D2 | `ps5-fuzz` | `cargo fuzz` targets for `elf/self/nid/dynamic/reloc/shader`, seed from `ps5-tests` | — | 4h |
| | **Phase D total** | | | **~20h** |

`ps5-fuzz` can land early (no deps) — schedule with Phase A if desired.

### Overall

| Phase | Scope | Effort | Cumulative |
|---|---|---|---|
| A | Extractions | 14h | 14h |
| B | Refactors | 26h | 40h |
| C | Knowledge bases | 18h | 58h |
| D | New domains | 20h | 78h |

Phases are incremental — each crate can ship independently. Recommended order: `A1 → A2 → D2 → A3 → B1 → B2 → B3 → C1 → C2 → D1`.

---

## 5. Checklist

- [ ] `ps5-deps` — new crate, `ps5-loader` re-exports, `ps5rs deps` + `--dot` CLI
- [ ] `ps5-signatures` — data crate, `ps5-analysis` becomes orchestrator
- [ ] `ps5-nid-db` — typed `NidRecord`, keep `ps5-nid` algorithm-only, CSV compat
- [ ] `ps5-prx` — `PrxModule` type, migrate `BinaryImageBuilder`/`dynamic.rs`/`libversion.rs`
- [ ] `ps5-schema` — `SCHEMA_VERSION` central, JSON Schema files, migrations
- [ ] `ps5-abi` — `FunctionSignature` store, 50 initial funcs, Registry validation
- [ ] `ps5-sdk-meta` — `SdkFunction` DB, import from `nids.csv` + `*.a`
- [ ] `ps5-firmware` — `FirmwareCatalog`, firmware-aware analysis (`libSceFoo v2.1 vs v1.4`)
- [ ] `ps5-shader` — `.ags`/`.agsd`/`disasm`/`reflection`
- [ ] `ps5-fuzz` — `fuzz/fuzz_targets/*.rs`, corpus from `ps5-tests`, CI 60s per target

## 6. Non-Goals (deferred per ROADMAP.md)

CPU execution/HLE beyond `ps5-emu` Registry, TLS block allocation, `.init_array` execution, full `libkernel` impl — all remain analysis-first.

---

*See also: `ROADMAP.md` (loader phases 1–6), `AGENTS.md` (crate conventions), `crates/ps5-*/src/lib.rs` for current APIs.*
