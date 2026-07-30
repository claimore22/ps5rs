# ps5rs Roadmap — Virtual Loader, Linking, and Knowledge Graph

## Project Vision

ps5rs focuses first on static analysis and virtual loading. Execution is a
future possibility, but not the current objective. All loader components are
designed to preserve that path without requiring an emulator today.

```
                   PS5 Software
                        |
                        v
                SELF / ELF Analysis
                        |
                        v
                Virtual Loader Layer
                        |
            +-----------+-----------+
            |                       |
            v                       v
      Knowledge Graph         Runtime Foundation
            |                       |
            |                  (future)
            |                       |
            v                       v
      NID / Modules /        Execution Engine
      Dependencies           + Syscalls
                             + Threads
                             + Memory
                             + GPU abstraction
```

The product is a research and analysis platform:

- ELF/SELF understanding
- Dependency analysis
- NID intelligence
- Library usage statistics
- Offline export database
- Cross-game comparison
- Virtual loading with import resolution measurement

## Current State

### Completed

#### Binary Parsing

| Capability | Status |
|---|:---:|
| SELF parsing | ✓ |
| ELF extraction | ✓ |
| Program headers | ✓ |
| Dynamic section | ✓ |
| Imports | ✓ |
| Exports | ✓ |
| Symbols | ✓ |
| Relocations | ✓ |
| TLS info (ElfImage/BinaryImage) | ✓ |
| Init/fini arrays (ElfImage/BinaryImage) | ✓ |

#### Virtual Loader

```
Map → Relocate → Link
```

| Capability | Status |
|---|:---:|
| PT_LOAD mapping | ✓ |
| Memory regions | ✓ |
| Segment permissions | ✓ |
| RELATIVE relocations | ✓ |
| DT_RELACOUNT fast path | ✓ |
| GLOB_DAT handling | ✓ |
| JUMP_SLOT handling | ✓ |
| ABS64 (local + import) | ✓ |
| Import resolver trait | ✓ |
| 3-tier resolver (runtime → offline → stub) | ✓ |
| OfflineExportTable (system_modules/*.exports.json) | ✓ |
| Stub allocator | ✓ |
| ModuleGraph with unavailable tracking | ✓ |
| ModuleState enum (Mapped/Relocated/Linked/Initialized) | ✓ |

#### CLI

| Command | Status |
|---|:---:|
| `ps5rs load <file>` | ✓ |
| `ps5rs load <file> --json` | ✓ |
| `ps5rs load <file> --prx-dir` (auto-defaults) | ✓ |
| `ps5rs exports <file>` | ✓ |
| `ps5rs exports <file> --json --search` | ✓ |
| `ps5rs catalog sync` | ✓ |
| `ps5rs catalog push-unknown` | ✓ |
| `ps5rs export-scan <dir>` | ✓ |
| `ps5rs batch-load <games_dir>` | ✓ |

#### Analysis Database

| Capability | Status |
|---|:---:|
| NID catalog (154K+ entries) | ✓ |
| Supabase catalog sync | ✓ |
| Unknown NID export | ✓ |
| Game dataset (scan/analyze) | ✓ |
| Dashboard (static HTML) | ✓ |
| Library heatmaps | ✓ |
| Engine fingerprinting | ✓ |
| String-based detection | ✓ |

### Missing / Incomplete

| Item | Gap |
|---|---|
| TLS on `LoadedModule` | `ElfImage` parses PT_TLS, `load_elf()` discards it |
| Init/fini on `LoadedModule` | `ElfImage` parses all arrays, `load_elf()` discards them |
| `ModuleState::Initialized` | Variant defined, no code path transitions to it |
| JSON: copy/tls/ifunc/unknown counts | `RelocationSummary` tracks them, `ModuleInfo` omits them |
| JSON: per-library import breakdown | Only aggregate counts, not broken down by library |
| Dependency graph: per-edge metadata | Shows `[MISSING]` but not export counts or import requirements |
| `catalog accept` | No local review/accept workflow |

---

## Completed Phases

### Phase 1 — Complete TLS + Init/Fini Metadata

**Goal:** `LoadedModule` carries all metadata that `ElfImage` already parsed.
No execution, no TLS blocks, no thread storage.

#### 1a. TLS on `LoadedModule`

- **Files:** `crates/ps5-loader/src/mapper.rs`
- **Change:** Add `tls: Option<TlsInfo>` to `LoadedModule`. In `load_elf()`,
  propagate `image.tls` into the module. Validate alignment, record
  file/memory size.
- **Tests:** 3-5 tests: TLS from PT_TLS, zero-size TLS, missing TLS.
- **Effort:** ~45 min ✅

#### 1b. Init/fini arrays on `LoadedModule`

- **Files:** `crates/ps5-loader/src/mapper.rs`, `crates/ps5-loader/src/pipeline.rs`
- **Change:** Add `init_va`, `fini_va`, `init_array: Vec<u64>`,
  `preinit_array: Vec<u64>`, `fini_array: Vec<u64>` to `LoadedModule`.
  Populate from `image.init_array_va`/`image.init_array_sz` etc. by reading
  function pointers from mapped memory. Transition to `ModuleState::Initialized`.
- **Tests:** 3-5 tests: init array recorded, empty init array, preinit present.
- **Effort:** ~1.5h ✅

#### 1c. Expose in JSON + terminal output

- **Files:** `crates/ps5-cli/src/load.rs`
- **Change:** Add to `ModuleInfo`: `init_va`, `fini_va`, `init_array_count`,
  `tls` block. Show in `print_modules()` terminal view.
- **Effort:** ~30 min ✅

**Total Phase 1: ~3h ✅**

---

### Phase 2 — Rich Load Reports

**Goal:** Every `ps5rs load --json` output is complete enough to feed downstream
analysis (dashboard, batch-load aggregator).

#### 2a. Add missing relocation fields to JSON

- **Files:** `crates/ps5-cli/src/load.rs`
- **Change:** Add `copy`, `tls`, `ifunc`, `unknown` fields to `ModuleInfo`
  (already in `RelocationSummary`, just not serialized).
- **Effort:** ~15 min ✅

#### 2b. Per-library import breakdown

- **Files:** `crates/ps5-cli/src/load.rs`, `crates/ps5-loader/src/resolver.rs`
- **Change:** Add `LibraryImportSummary { library, resolved, known, stubbed }`
  to `ModuleInfo`. `CrossModuleResolver` already categorizes every import
  by result type — just group by library name.
- **Tests:** 2-3 tests: per-library counts add up to totals.
- **Effort:** ~2h ✅

#### 2c. Dependency graph metadata

- **Files:** `crates/ps5-cli/src/load.rs`
- **Change:** In `print_graph()` and `GraphInfo`, show per-edge:
  `exports_available`, `imports_required`, `status` (loaded/missing/offline).
- **Effort:** ~1h ✅

**Total Phase 2: ~3.5h ✅**

---

### Phase 3 — Export-Scan Bootstrapper

**Goal:** One command to populate `system_modules/` from any PRX directory.

#### 3a. `ps5rs export-scan` command

- **Files:** `crates/ps5-cli/src/cli.rs`, newly
  `crates/ps5-cli/src/export_scan.rs`
- **Signature:** `ps5rs export-scan <dir> [--output system_modules/]`
- **Behavior:**
  1. Walk `<dir>` for `*.prx` files
  2. For each PRX, parse ELF, extract exports
  3. Write `<module>.exports.json` to output directory
  4. Skip duplicates (same module name already exists with same hash)
- **Edge cases:** Non-ELF files, corrupt PRX, SELF-wrapped PRX
- **Tests:** 3-5 tests: directory with 0/1/N PRXes, duplicate detection.
- **Effort:** ~4h ✅

#### 3b. Run against SDK + all game PRXes

- Bootstraps `system_modules/` with all unique exports from:
  - Prospero SDK `target/sce_module/` (6 PRXes, already done)
  - All 39 game `sce_module/` directories
- **Result:** Comprehensive offline database covering SDK + game-specific
  versions.
- **Effort:** ~15 min ✅

**Total Phase 3: ~4.5h ✅**

---

### Phase 4 — Bulk Analysis Pipeline

**Goal:** Run the loader against the entire game corpus and collect statistics.

#### 4a. `ps5rs batch-load <games_dir>` command

- **Files:** `crates/ps5-cli/src/cli.rs`, newly
  `crates/ps5-cli/src/batch_load.rs`
- **Signature:** `ps5rs batch-load <games_dir> [--output analysis/load/]`
- **Behavior:**
  1. Walk `<games_dir>` (same format as `ps5rs scan`)
  2. For each game, run the full load pipeline (eboot + PRX dependencies)
  3. Write per-game `analysis/load/<game>.json` with full `LoadReport`
  4. Write `analysis/load/summary.json` with aggregate stats
- **Tests:** 3-4 tests: single game, multi-game, missing PRX dirs.
- **Effort:** ~6h ✅

#### 4b. Corpus analysis

- Run `batch-load` against all 39 games
- **Measured output:**
  - Average imports per game
  - Average resolution rate (resolved + known) vs stubbed
  - Most commonly unavailable system modules (ranked)
  - Games with the most unknown NIDs
  - Performance benchmarks (859K relocations on Octopath Traveler II)
- **Effort:** ~30 min (run time) ✅

**Total Phase 4: ~6.5h ✅**

---

### Phase 5 — Dashboard Loader Analytics

**Goal:** Loader coverage stats visible in the dashboard alongside existing
analysis data.

#### 5a. Extend data structures

- **Files:** `crates/ps5-dashboard/src/data.rs`
- **Change:** Add to `GameDetail`:
  - `load_state: Option<String>` (Linked/Initialized)
  - `imports_resolved/known/stubbed: Option<u32>`
  - `tls: Option<{address, file_size, memory_size, alignment}>`
  - `init_array_count/fini_array_count: Option<u32>`
  - `unavailable_modules: Vec<String>`
- Add to `DashboardData`: optional `loader_summary` section with aggregate
  resolution stats across all loaded games.
- **Effort:** ~3h ✅

#### 5b. Dashboard "Load Coverage" view

- **Files:** `crates/ps5-dashboard/src/html.rs`,
  `crates/ps5-dashboard/src/templates/`
- **Change:** New tab or section showing:
  - Per-game bar: resolved % / known % / stubbed %
  - Per-library table: which system modules are most/least covered
  - Unavailable module frequency list
  - ⚠️ Only shown when loader data is present (backward compat with old
    datasets)
- **Effort:** ~5h ✅

#### 5c. CLI integration

- **Files:** `crates/ps5-cli/src/dataset.rs`
- **Change:** `cmd_dashboard()` calls `data.inject_loader_data(&loader_dir)` after `compute()` to populate loader data from `analysis/load/` directory.
- **Effort:** ~30 min ✅

**Total Phase 5: ~8.5h ✅**

---

## Upcoming Phases

### Phase 6 — Catalog Accept Workflow

**Goal:** Complete the catalog cycle: scan → push → review → accept.

#### 6a. `catalog list-submissions`

- **Files:** `crates/ps5-cli/src/catalog.rs`
- **Signature:** `ps5rs catalog list-submissions [--pending]`
- **Behavior:** Fetch submissions from Supabase where `status = 'pending'`.
  Show NID, proposed name, library, submitter, evidence count,
  github_verified.
- **Effort:** ~2h

#### 6b. `catalog accept <nid>`

- **Files:** `crates/ps5-cli/src/catalog.rs`
- **Signature:**
  `ps5rs catalog accept <nid> [--name <name>] [--library <lib>]`
- **Behavior:**
  1. Look up NID in pending submissions
  2. Validate name format
  3. Insert into local `analysis/catalog/nids.csv`
  4. Optionally update Supabase submission status to `accepted`
- **Dry-run mode:** preview without writing.
- **Effort:** ~4h

**Total Phase 6: ~6h**

---

## Explicitly Deferred

| Feature | Reason |
|---|---|
| CPU execution / HLE | SharpEmu territory; our edge is analysis + virtual loading |
| Thread-local storage allocation | Just record metadata; don't allocate TLS blocks at runtime |
| `.init_array` execution | Metadata-only for now; execution requires runtime isolation |
| Full `libkernel` implementation | Not needed for analysis; system-module PRX dumps would help |
| NID catalog merge conflict resolution | Wait until v2 merge semantics hit scale limits |

---

## Summary

| Phase | Description | Effort | Status |
|---:|---|:---:|:---:|
| 1 | TLS + Init/fini metadata on LoadedModule | ~3h | ✅ |
| 2 | Rich load reports (JSON + terminal) | ~3.5h | ✅ |
| 3 | Export-scan bootstrapper | ~4.5h | ✅ |
| 4 | Bulk analysis pipeline | ~6.5h | ✅ |
| 5 | Dashboard loader analytics | ~8.5h | ✅ |
| 6 | Catalog accept workflow | ~6h | 🔜 |
| | **Total** | **~31.5h** | **26.5h done / 6h remaining** |

The narrative end-to-end:

```
ps5rs scan games/          → fingerprint everything
ps5rs batch-load games/    → virtually load everything, measure unknowns
ps5rs dashboard analysis/  → explore load coverage + NID intelligence
ps5rs catalog accept <nid> → grow the knowledge base
```

Not "run the games" — *understand every PS5 binary and build the largest open
knowledge base of PS5 software structure.*
