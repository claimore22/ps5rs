# ps5rs — Big Pickle Implementation Plan

You are working on the ps5rs repository.

MISSION:
Turn ps5rs from a collection of partially connected analysis crates into an
integrated PS5 binary research platform.

BIG_PLAN.md (this file) is the authoritative implementation plan and has priority over all other planning/audit documents.

IMPORTANT:
The repository was audited on 2026-08-27 on branch `elfspector-dev`.
The audit is the current implementation baseline.

Read first:

BIG_PLAN.md HAS PRIORITY  over those :
- AGENTS.md
- ROADMAP.md
- UPGRADE_PLAN.md
- UPGRADE_PLAN2.md
- IMPLEMENTATION_AUDIT.md

DO NOT assume any of these documents are accurate.
Verify the current source before changing code.

The implementation audit is especially important:
`IMPLEMENTATION_AUDIT.md`

============================================================
HARD RULES
============================================================

1. Preserve existing working behavior.
2. Inspect existing implementations before rewriting anything.
3. Small, idiomatic Rust modules.
4. No unwrap().
5. No expect().
6. No panic!() for normal error handling.
7. Prefer Result<T, E> / Option<T>.
8. No unnecessary dependencies.
9. No fake implementations.
9B. 
# 18. EXTERNAL VALIDATION CORPUS

The development environment may contain additional tool/sample files that are
available ONLY as local validation inputs.

These external validation inputs are NOT project assets.

## Rules

- Do not copy these files into the repository.
- Do not commit them.
- Do not embed their contents into Rust source.
- Do not generate fixtures by copying proprietary SDK content into the repository.
- Do not add hardcoded absolute paths to source code.
- Do not mention proprietary/local SDK paths in committed project documentation,
  source code, tests, schemas, or generated permanent datasets.
- Do not redistribute extracted proprietary headers, libraries, symbols, strings,
  binaries, or SDK metadata.
- Validation commands may accept user-supplied external paths.
- Validation results must contain aggregated/derived information only.
- If a test requires an external SDK/sample corpus, it must skip cleanly when the
  corpus is unavailable rather than failing normal CI.
- The normal workspace build/test must not require the external corpus.

The implementation must treat these files as an external oracle for validation,
not as a dependency of ps5rs.

## External validation categories

Use available external material to validate:

### Native binaries

Search supplied validation roots for:

- `.elf`
- `.prx`
- `.sprx`
- `.self`
- `.o`
- `.so`

Use them to validate:

- ELF parsing
- SELF extraction
- PRX detection
- module metadata
- imports
- exports
- NIDs
- library versions
- relocations
- dependencies
- symbols
- loader behavior

### Static libraries

If `.a` archives are available, use them only as external validation inputs.

Validate:

- object/member discovery
- symbol extraction
- NID/stub metadata where legitimately observable
- SDK metadata ingestion
- ABI/signature correlation

Do not copy archive contents into the repository.

### SDK/sample source

If `.cpp`, `.c`, `.h`, or `.hpp` sample files are available:

Use them as an external validation oracle.

Extract only transient test expectations while running validation.

For example:

    source API call
          ↓
    compiled binary import
          ↓
    NID
          ↓
    ps5rs resolution
          ↓
    expected vs actual

This must validate that ps5rs correctly associates observed binary imports with
known APIs.

Do NOT make the committed project depend on proprietary headers or source files.

### Shader artifacts

Search external validation roots for:

- `.pssl`
- `.ags`
- `.agsd`
- `.sb`

Use these to validate:

- shader artifact discovery
- format identification
- shader parsing
- stage detection
- metadata extraction
- entry points
- hashes
- resources
- debug information
- cross-artifact relationships

Where both source shader and compiled shader exist, use the source as a validation
oracle for the compiled artifact.

Do not copy shader files into the repository.

### Metadata/configuration

Where present, inspect:

- `.json`
- `.xml`
- `.map`
- `.sym`
- `.txt`
- `.log`

Only use formats whose semantics can be established.

`.map`/`.sym` files may be useful for validating symbol/address relationships.

Logs and text files are diagnostic evidence only and must not become authoritative
binary-analysis facts unless their provenance is understood.

## Validation modes

Implement validation so external corpora can be supplied explicitly, for example:

    ps5rs validate external <path>

or an equivalent command consistent with the existing CLI architecture.

The command should produce structured aggregate results such as:

    validation-report.json
    validation-report.md

Include:

- files discovered by type
- files successfully parsed
- files rejected
- parser errors by category
- ELF/SELF/PRX statistics
- import/export statistics
- NID resolution statistics
- dependency statistics
- shader statistics
- source↔binary API correlation statistics
- ABI/signature validation statistics
- firmware/export-table validation statistics
- engine detection validation statistics

Do NOT include copied proprietary source/header contents.

## Reproducibility

External validation must be optional.

Normal:

    cargo test --workspace

must continue to work without the external corpus.

External validation may be run manually against locally available material.

The validation system should clearly distinguish:

    PASS
    FAIL
    SKIPPED — external corpus unavailable
    INSUFFICIENT EVIDENCE

Never turn "external corpus unavailable" into a fake PASS.

## Critical principle

The external SDK/sample corpus is a validation oracle, not part of ps5rs.

ps5rs must remain independently buildable and distributable without those files.


build a sample validation matrix rather eg :

                 parser   imports   NIDs   deps   shader   ABI   dashboard
ELF                ✓        ✓        ✓      ✓
PRX                ✓        ✓        ✓      ✓
SELF               ✓
.A                 ✓                 ✓                  ✓
CPP/HPP                               expected API       ✓
PSSL                                                    ✓
AGS                                                     ✓
AGSD                                                    ✓
SB                                                      ✓

## External validation executables (`.exe`)

Locally installed SDK/tool executables may be used as external validation oracles.

Examples include:

- compiler/toolchain executables
- shader compilation tools
- shader inspection/disassembly tools
- ELF/SELF/PRX inspection tools
- symbol/NID utilities
- packaging/build tools
- sample-generation tools
- metadata conversion tools
- debugging/analysis utilities

### Rules

- Do NOT copy `.exe` files into the repository.
- Do NOT commit `.exe` files.
- Do NOT redistribute proprietary executables.
- Do NOT hardcode absolute executable paths into Rust source.
- Do NOT make normal `cargo build` or `cargo test` depend on these executables.
- Do NOT silently execute arbitrary executables discovered on disk.
- External tools must be explicitly supplied/configured by the user.
- If an external tool is unavailable, validation should report `SKIPPED` or
  `INSUFFICIENT EVIDENCE`, not fabricate a result.
- Tool output must be treated as external validation evidence, not automatically
  as authoritative truth.
- Record the tool name/version when available.
- Do not store proprietary tool output containing SDK source, headers, symbols,
  or other restricted material in the repository.

### Tool-assisted validation

When an external tool can independently answer a question that ps5rs answers,
use it for differential validation.

The following local directories may be used as READ-ONLY validation/reference resources:

1. Local toolchain directory (path redacted):

   Use available command-line tools as independent validation oracles for ELF,
   shader, symbol, binary, metadata, and related analysis.

2. Complete sample workspace (path redacted):

   Use the COMPLETE tree for validation, including:
   - sample source code (`*.cpp`, headers, projects, etc.)
   - sample data/assets
   - shader sources
   - compiled shaders
   - compiled ELF/PRX/SPRX binaries
   - build/intermediate artifacts
   - release output directories and their resulting binaries
   - any other files produced by the sample builds

3. Real PS5 game corpus:
   C:\Users\claimoar\Documents\ROMS\PS5\

   This may contain both packed (`*.pak`) and unpacked/bare game content.
   Inventory and analyze useful artifact types rather than assuming a
   particular packaging format.

### Validation Strategy

Use the sample SOURCE → BUILD → RESULT relationship whenever possible:

    sample source
        ↓
    compiled artifact
        ↓
    ps5rs analysis
        ↓
    compare against independent tool/reference information

Use the executable tools above as validation oracles where appropriate.

Validate ELF/SELF/PRX parsing, imports, exports, relocations, NIDs,
libraries, dependencies, symbols, shaders, metadata, engine/middleware
fingerprints, and other analysis features against real artifacts.

### STRICT REPOSITORY RULES

- These directories are EXTERNAL validation resources only.
- NEVER hardcode these local paths into `ps5rs`.
- NEVER require them for normal builds or tests.
- NEVER copy, commit, embed, or redistribute their contents.
- NEVER commit proprietary source, headers, binaries, libraries, shaders,
  sample data, or executable tool output.
- Tests may reference configurable external validation paths, but must
  gracefully skip when the resources are unavailable.
- Store only derived validation results such as counts, hashes where
  appropriate, pass/fail results, mismatches, and aggregated statistics.
- **NEVER mention the vendor/product name in committed source code,
  comments, documentation, schemas, tests, or generated project data.**
- The repository must remain completely self-contained without these
  external resources.

Example:

    external tool
          │
          ├── identifies ELF metadata
          ├── identifies imports
          ├── identifies symbols
          ├── identifies shader metadata
          └── identifies hashes
                  │
                  ▼
             ps5rs result
                  │
                  ▼
             compare results

The validation report should distinguish:

    MATCH
    MISMATCH
    UNSUPPORTED BY PS5RS
    UNSUPPORTED BY EXTERNAL TOOL
    TOOL FAILED
    TOOL UNAVAILABLE
    INSUFFICIENT EVIDENCE

### Executable discovery

Do not scan the entire machine indiscriminately.

Only inspect executables beneath validation roots explicitly supplied by the user.

For each candidate tool, determine:

- filename
- tool version, where safely obtainable
- supported input types
- command-line interface
- whether it can produce machine-readable output
- whether it can inspect without modifying the input
- whether its output can be safely reduced to aggregate validation data

Prefer machine-readable output such as JSON when available.

### Differential validation

Where practical, build validation adapters rather than embedding knowledge
about a particular proprietary executable into the core analyzer.

Conceptually:

    ExternalToolAdapter
          │
          ├── input artifact
          ├── invoke configured tool
          ├── parse machine-readable output
          └── produce ValidationObservation

Then:

    ValidationObservation
            ↕
       ps5rs Analysis
            ↓
       ValidationReport

The adapter must remain optional and isolated from the core analysis pipeline.

### Important

The goal is NOT to make ps5rs call proprietary tools to perform its analysis.

The goal is to independently validate ps5rs' own implementation.

If ps5rs says:

    NID 0x12345678 → function X

and an external tool independently says:

    NID 0x12345678 → function X

that is useful validation evidence.

Likewise, if ps5rs identifies a shader stage, hash, ELF segment,
relocation, symbol, or module dependency, an external tool can provide
an independent comparison.

Do not replace ps5rs implementation with a wrapper around an external executable.
I'd actually make .exe a first-class validation category

The full external corpus should therefore be:

External Validation Corpus
│
├── Native binaries
│   ├── .elf
│   ├── .prx
│   ├── .sprx
│   ├── .self
│   ├── .o
│   └── .so
│
├── Libraries / metadata
│   ├── .a
│   ├── .map
│   ├── .sym
│   ├── .json
│   └── .xml
│
├── Source
│   ├── .c
│   ├── .cpp
│   ├── .h
│   └── .hpp
│
├── Shaders
│   ├── .pssl
│   ├── .ags
│   ├── .agsd
│   └── .sb
│
└── External tools
    └── .exe
        ├── compiler/toolchain
        ├── shader tools
        ├── binary inspection
        ├── symbol/NID tools
        ├── packaging tools
        └── other analysis utilities

The .exe category is potentially one of the strongest validation sources, because if one of those tools can
independently parse/compile/inspect an artifact,

10. No placeholder APIs merely to make architecture look complete.
11. No synthetic analysis presented as real analysis.
12. Every important analysis result must have structured data.
13. Dashboard consumes structured analysis data.
14. Do not independently rediscover information in dashboard code.
15. Do not invent exact versions or signatures.
16. Unknown information must remain explicitly unknown.
17. Add tests for real behavior.
18. Do not mark a task complete merely because code compiles.
19. Do not expand CPU emulation/HLE unless required by analysis.
20. JSON-only persistence. No SQLite.
21. Dashboard must remain self-contained/file:// compatible.
22. Avoid regex if AGENTS.md prohibits it; use the repository's established
    string matching approach.
23. Do not add proprietary SDK material to the repository.
24. Do not remove existing useful functionality simply because it is incomplete.
    Replace synthetic behavior with honest empty/unknown states first.
	

DEFINITION OF DONE FOR EVERY FEATURE:

[ ] Implemented
[ ] Integrated
[ ] Tested
[ ] Used by real PS5 data
[ ] Represented in structured data/schema
[ ] CLI exposed where appropriate
[ ] Dashboard exposed where appropriate
[ ] Documented
[ ] No synthetic/fabricated claims

============================================================
PHASE 0 — HONESTY GATE / REPAIR CURRENT STATE
============================================================

Goal:
Make ps5rs trustworthy before adding more features.

------------------------------------------------------------
0.1 Dashboard synthetic data
------------------------------------------------------------

Find and remove/replace all synthetic analysis.

Known problems:

- compute_shader_summary()
- compute_firmware_summary()
- hardcoded SDK timeline
- sce_library_stats.module_count == 0
- synthetic shader stage classification
- synthetic shader resource counts
- synthetic firmware module counts

For anything not genuinely implemented:

DO NOT GUESS.

Render:

"Not yet analyzed"

or

"Unknown"

or

"Not available in current dataset"

instead of fabricated numbers.

Checklist:

[ ] Shader summary no longer invents vertex/pixel counts
[ ] Shader resources no longer use `total * 2`
[ ] Firmware summary no longer treats game count as firmware modules
[ ] SDK timeline is not presented as detected SDK information
[ ] module_count is either real or shown as unavailable
[ ] Dashboard has honest empty states
[ ] Tests prevent synthetic values from returning

------------------------------------------------------------
0.2 Middleware pipeline
------------------------------------------------------------

Current problem:

`ps5rs middleware` defaults to terminal output.

Existing update script writes:

analysis/reports/middleware.json

but the file is terminal text rather than JSON.

Fix the CLI/script integration.

Checklist:

[ ] middleware supports --format json
[ ] JSON output is valid JSON
[ ] update script uses --format json
[ ] dashboard consumes middleware JSON/structured data
[ ] middleware data is available without requiring accidental CLI flags
[ ] middleware failures are distinguishable from "no middleware detected"

------------------------------------------------------------
0.3 Dashboard --games integration
------------------------------------------------------------

Current dashboard script does not pass `--games`.

Fix so dashboard gets the game dataset required for game-level analysis.

Checklist:

[ ] update script passes game corpus
[ ] middleware appears when data exists
[ ] game information is populated
[ ] no duplicated scanning logic

------------------------------------------------------------
0.4 ps5-nid-db::from_catalog()
------------------------------------------------------------

Current implementation ignores its input.

Implement actual conversion:

Catalog -> NidDatabase

Preserve:

- NID
- library
- canonical name
- aliases
- source
- confidence
- tags where available

Checklist:

[ ] from_catalog actually imports records
[ ] indexes populated
[ ] duplicate behavior defined
[ ] conflict behavior defined
[ ] tests
[ ] real 165K catalog can flow into NidDatabase

------------------------------------------------------------
0.5 ps5-signatures zero-score detections
------------------------------------------------------------

Current SDK/third-party pattern detections can return:

score = 0
confidence = 0

without useful evidence.

Do not pretend this is detection.

Either:

[ ] implement actual scoring

OR

[ ] return an explicit "unknown/no evidence" result.

No fake positive/negative confidence.

------------------------------------------------------------
0.6 Firmware availability placeholder
------------------------------------------------------------

Current:

is_library_available() -> true

This is unacceptable.

Until real firmware matching exists:

[ ] replace unconditional true with Unknown/NotImplemented semantics

Then later implement properly in Phase 3.

------------------------------------------------------------
0.7 ABI provenance gate
------------------------------------------------------------

Audit all seed signatures.

Especially investigate entries such as:

free -> U32

Do not assume a signature is correct.

Add provenance:

Verified
Unverified
Unknown

Checklist:

[ ] fabricated signatures removed or marked unverified
[ ] verified signatures have a source
[ ] ABI consumers can distinguish verified/unverified
[ ] HLE validation does not treat unverified signatures as authoritative

------------------------------------------------------------
0.8 Dead template / root binary
------------------------------------------------------------

Investigate:

crates/ps5-dashboard/src/template.rs

and:

src/main.rs

Remove dead duplicate dashboard template if genuinely unused.

Wire root binary only if that is consistent with repository architecture.

Do not create unnecessary functionality just to eliminate a stub.

------------------------------------------------------------
0.9 Initialization honesty
------------------------------------------------------------

Document that loader currently reaches Initialized without executing:

- .preinit_array
- .init_array
- DT_INIT

Do not claim full module initialization.

Add a test/documented state if appropriate.

============================================================
PHASE 0 EXIT GATE
============================================================

Run:

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

Then run the real scan.

Do not proceed if Phase 0 introduces regressions.

============================================================
PHASE 1 — NID KNOWLEDGE + EVIDENCE SYSTEM
============================================================

Goal:

Make every important conclusion explainable.

Create a reusable evidence model.

Target concept:

Detection {
    value,
    confidence,
    evidence
}

Evidence should contain enough information to answer:

"Why did ps5rs think this?"

Possible evidence types:

- NID
- library
- symbol
- string
- module name
- source path
- export table
- SDK metadata
- firmware metadata
- shader metadata
- build ID
- file/module location
- cross-reference
- catalog source

Do not overdesign this.

------------------------------------------------------------
1.1 Stable evidence model
------------------------------------------------------------

[ ] Define Evidence
[ ] Define Detection
[ ] Add source/provenance information
[ ] Serialize to JSON
[ ] Add tests
[ ] Integrate with existing analysis structures

------------------------------------------------------------
1.2 NID knowledge layer
------------------------------------------------------------

Implement:

[ ] NID -> record
[ ] library -> records
[ ] name -> records
[ ] aliases -> records
[ ] source
[ ] confidence
[ ] tags
[ ] SDK/version metadata where known
[ ] engine classification
[ ] middleware classification

Add reverse lookup.

Do not automatically classify an NID as Unreal simply because
the string contains "Unreal".

Classification requires evidence.

------------------------------------------------------------
1.3 Firmware export tables
------------------------------------------------------------

Load:

system_modules/*.exports.json

The audit found 8 real tables.

Make these usable by ps5-nid-db / ps5-firmware.

Checklist:

[ ] loader
[ ] provenance
[ ] NID index
[ ] library index
[ ] version metadata where available
[ ] tests against actual export files

------------------------------------------------------------
1.4 Unknown NID research data
------------------------------------------------------------

For every unknown NID store enough context to investigate it:

- NID
- library
- occurrence count
- games
- modules
- nearby strings where available
- known aliases
- known cross references
- possible category
- confidence
- evidence

Do not fabricate "possible engine".

If there is no evidence:

possible_engine = unknown

============================================================
PHASE 2 — AUTHORITATIVE PRX/MODULE MODEL
============================================================

Goal:

ps5-prx becomes the authoritative module representation.

Current problem:

ps5-image parses ELF independently.

Fix integration rather than rewriting parsers.

------------------------------------------------------------
2.1 PrxModule
------------------------------------------------------------

Ensure PrxModule represents:

[ ] module type
[ ] ELF metadata
[ ] SONAME
[ ] DT_NEEDED
[ ] DT_SCE_NEEDED_LIB
[ ] imports
[ ] exports
[ ] NIDs
[ ] library versions
[ ] TLS
[ ] entry point
[ ] build ID
[ ] init/fini metadata
[ ] module dependencies

Only expose fields that can actually be derived.

------------------------------------------------------------
2.2 Integrate
------------------------------------------------------------

Wire PrxModule into:

[ ] ps5-image
[ ] ps5-loader
[ ] ps5-analysis
[ ] ps5-deps
[ ] ps5-cli

Avoid duplicate ELF/module models.

------------------------------------------------------------
2.3 Dependency graph v2
------------------------------------------------------------

Graph must represent:

game
eboot
PRX
SPRX
system library
imported library
exported library
unresolved dependency

Typed edges.

Add:

[ ] deps command
[ ] JSON output
[ ] DOT output
[ ] tests

Dashboard must consume this graph.

============================================================
PHASE 3 — REAL SHADER ANALYSIS
============================================================

This is a major priority.

There are 649 real shader files.

Current shader implementation is synthetic:

- keyword stage detection
- hexdump "disassembly"
- text resource scan
- AGC stub

Do NOT fake a shader parser.

------------------------------------------------------------
3.1 Corpus investigation
------------------------------------------------------------

Before implementing:

[ ] inspect actual .sb files
[ ] inspect actual PSSL-related artifacts
[ ] inspect .ags/.agsd files
[ ] identify repeated headers
[ ] identify metadata structures
[ ] identify hashes
[ ] identify stage information
[ ] identify resource metadata
[ ] identify debug information

Document discoveries.

------------------------------------------------------------
3.2 Implement real format parsing
------------------------------------------------------------

Only implement structures that are actually understood.

Target:

[ ] format identification
[ ] stage
[ ] entry point
[ ] shader hash
[ ] resource bindings
[ ] samplers
[ ] textures
[ ] buffers
[ ] render targets
[ ] vertex attributes
[ ] metadata
[ ] .agsd debug information where available

If something cannot be decoded:

represent it as raw/unknown structured data.

Never label a hexdump "disassembly".

------------------------------------------------------------
3.3 Shader dataset
------------------------------------------------------------

Add shader records to stable schema.

For each shader:

- game
- path
- format
- stage
- hash
- resources
- metadata
- debug info
- parse confidence/status

------------------------------------------------------------
3.4 Dashboard
------------------------------------------------------------

Shader tab must consume shader records.

Show:

[ ] shader count
[ ] stages
[ ] per-game inventory
[ ] hashes
[ ] resource information
[ ] metadata
[ ] debug information
[ ] parse status

No synthetic counts.

============================================================
PHASE 4 — SDK + ABI + FIRMWARE
============================================================

------------------------------------------------------------
4.1 SDK database
------------------------------------------------------------

Connect ps5-sdk-meta to real data sources available in the repository.

Sources may include:

- nids.csv
- stub-derived metadata
- existing project data

Only use data that is legitimately available to the project.

Each record:

- function
- library
- module
- category
- aliases
- version range
- confidence
- source

Checklist:

[ ] real import
[ ] indexes
[ ] provenance
[ ] version ranges
[ ] game usage report
[ ] unresolved SDK NIDs
[ ] SDK/library heatmap

------------------------------------------------------------
4.2 SDK detection

Combine:

- NIDs
- libraries
- symbols
- strings
- version hints
- module metadata

Do not claim an exact SDK version unless evidence supports it.

Example:

SDK major generation known
SDK minor version unknown

is valid.

------------------------------------------------------------
4.3 ABI

Replace fabricated seeds.

Each signature must have provenance:

[ ] verified
[ ] unverified
[ ] unknown

Verified signatures can be used for validation.

Unverified signatures cannot silently become authoritative.

Add:

[ ] signature comparison
[ ] HLE handler validation
[ ] mismatch reporting

------------------------------------------------------------
4.4 Firmware

Implement actual catalog loading from:

system_modules/*.exports.json

and game requirements.

Answer:

Game requires:
    libSceFoo >= X

Firmware provides:
    libSceFoo Y

Result:

compatible
insufficient
unknown

Only return "compatible" when evidence supports it.

============================================================
PHASE 5 — ENGINE FORENSICS
============================================================

Current engine detection is mostly strings.

Upgrade to multi-signal detection.

Signals:

[ ] strings
[ ] NIDs
[ ] libraries
[ ] module names
[ ] symbol names
[ ] source paths
[ ] build IDs
[ ] shader metadata
[ ] known fingerprints
[ ] version fingerprints

Support:

[ ] Unreal Engine 4
[ ] Unreal Engine 5
[ ] Unity
[ ] Godot
[ ] native/custom
[ ] unknown

------------------------------------------------------------
5.1 Unreal

Detect:

[ ] UE4 vs UE5
[ ] likely minor version
[ ] source paths
[ ] plugin paths
[ ] engine-specific NIDs
[ ] engine-specific symbols
[ ] fork/customization evidence

Potential evidence:

Engine/Source/Runtime
Engine/Plugins
FName
UObject
UE-specific symbols
known version fingerprints

Do NOT classify a generic NID as Unreal based solely on its name.

------------------------------------------------------------
5.2 Version uncertainty

Examples:

"Unreal Engine 5"
confidence 0.89

is acceptable.

"Unreal Engine 5.3"
only when evidence supports 5.3.

Never infer exact version merely because it is plausible.

============================================================
PHASE 6 — DASHBOARD V2
============================================================

Goal:

Make dashboard a research interface, not a pretty summary.

Required areas:

Overview
Games
Game Detail
Modules
Module Detail
NIDs
Libraries
Dependencies
Engines
Middleware
SDK
Firmware
Shaders
Evidence / Analysis

------------------------------------------------------------
6.1 Overview
------------------------------------------------------------

Real statistics:

[ ] games
[ ] binaries
[ ] PRX/SPRX
[ ] imports
[ ] resolved imports
[ ] unresolved imports
[ ] resolution %
[ ] unique NIDs
[ ] unknown NIDs
[ ] libraries
[ ] engines
[ ] engine versions
[ ] middleware
[ ] SDK generations
[ ] firmware versions
[ ] shaders
[ ] dependency edges

------------------------------------------------------------
6.2 Game detail
------------------------------------------------------------

Show:

identity
engine
engine version
confidence
evidence
SDK
NIDs
modules
dependencies
middleware
shaders
firmware compatibility

------------------------------------------------------------
6.3 NID research
------------------------------------------------------------

Per-NID page:

[ ] NID
[ ] hex/u64 representation
[ ] library
[ ] canonical name
[ ] aliases
[ ] source
[ ] confidence
[ ] games
[ ] modules
[ ] occurrences
[ ] evidence
[ ] classification

Unknown NID page:

[ ] frequency
[ ] library
[ ] games
[ ] modules
[ ] nearby strings
[ ] possible classification
[ ] confidence
[ ] evidence

Sorting:

[ ] frequency
[ ] library
[ ] game count
[ ] confidence

------------------------------------------------------------
6.4 Heatmaps
------------------------------------------------------------

Implement real:

[ ] game x library
[ ] game x NID
[ ] library x NID
[ ] resolved/unresolved
[ ] engine-specific NIDs

------------------------------------------------------------
6.5 Evidence UI
------------------------------------------------------------

Every major conclusion must expose:

WHY?

Example:

Engine:
Unreal Engine 5

Confidence:
0.91

Evidence:
- Engine/Source/Runtime
- Engine/Plugins
- UObject symbol
- UE5 fingerprint
- matching engine NIDs

============================================================
PHASE 7 — SCHEMA + DATASET
============================================================

ps5-schema becomes the stable interchange layer.

Define/validate:

[ ] game
[ ] module
[ ] image
[ ] NID
[ ] dependency
[ ] shader
[ ] SDK
[ ] firmware
[ ] detection
[ ] evidence

Do not leave major binary analysis information as arbitrary
serde_json::Value when a stable typed schema is practical.

Dashboard should consume schema/dataset objects.

============================================================
PHASE 8 — CLI COMPLETENESS
============================================================

Required commands:

[ ] scan
[ ] inspect
[ ] imports
[ ] strings
[ ] nid
[ ] deps
[ ] exports
[ ] catalog
[ ] middleware
[ ] validate
[ ] analyze
[ ] dashboard
[ ] shader
[ ] firmware
[ ] sdk

Every appropriate command supports:

[ ] useful human output
[ ] JSON output
[ ] actionable errors
[ ] --help
[ ] tests

Do not add commands that merely wrap an empty implementation.

============================================================
PHASE 9 — REAL DATA VALIDATION
============================================================

Run against:

C:\Users\claimoar\Documents\ROMS\PS5

Current known corpus:

27 games
649 shader files
8 firmware export tables
~165,654 builtin NIDs

Do not assume these counts remain unchanged.

Generate:

analysis/validation-report.json
analysis/validation-report.md

Record:

[ ] games
[ ] binaries
[ ] PRX/SPRX
[ ] imports
[ ] resolved
[ ] unresolved
[ ] unique NIDs
[ ] unknown NIDs
[ ] libraries
[ ] engines
[ ] engine versions
[ ] middleware
[ ] SDK
[ ] firmware
[ ] shaders
[ ] dependencies

Cross-check dashboard totals against validation report.

============================================================
PHASE 10 — TESTING / HARDENING
============================================================

Run:

cargo fmt --check

cargo clippy --workspace --all-targets --all-features -- -D warnings

cargo test --workspace

Also:

[ ] malformed ELF tests
[ ] malformed SELF tests
[ ] malformed PRX tests
[ ] malformed shader tests
[ ] NID database tests
[ ] schema compatibility tests
[ ] CLI tests
[ ] dashboard generation tests
[ ] evidence tests
[ ] regression tests

Fuzzing:

Investigate root `fuzz/`.

It currently contains 6 targets but is outside the workspace.

Do not pretend `cargo test --workspace` tests fuzz targets.

Make fuzzing integration/documentation explicit.

============================================================
EXECUTION ORDER
============================================================

DO NOT implement everything at once.

Use this exact order:

PHASE 0
  ↓
PHASE 1
  ↓
PHASE 2
  ↓
PHASE 3
  ↓
PHASE 4
  ↓
PHASE 5
  ↓
PHASE 6
  ↓
PHASE 7
  ↓
PHASE 8
  ↓
PHASE 9
  ↓
PHASE 10

After each phase:

1. Run tests.
2. Run clippy.
3. Inspect git diff.
4. Run relevant real-data analysis.
5. Update IMPLEMENTATION_AUDIT.md.
6. Mark only genuinely completed checklist items.
7. Record remaining limitations.

============================================================
IMPORTANT: DO NOT DO THIS
============================================================

DO NOT:

- invent shader metadata
- invent firmware compatibility
- invent SDK versions
- invent ABI signatures
- classify NIDs from one weak string match
- use hashes from DefaultHasher as cryptographic/stable shader identity
- call a hexdump a disassembly
- manufacture dashboard statistics
- make hardcoded dashboard claims look like detected facts
- create empty crates
- create TODO-only modules
- create compatibility shims hiding missing functionality
- rewrite working ELF/SELF parsing unnecessarily
- make the dashboard independently parse binaries
- silently swallow analysis errors
- silently convert unknown into false
- mark a phase complete because tests compile

============================================================
FINAL DEFINITION OF DONE
============================================================

The project is complete only when:

[ ] real PS5 corpus can be scanned end-to-end
[ ] all major analysis information survives into structured dataset
[ ] dashboard consumes that dataset
[ ] no synthetic statistics remain
[ ] unknown information is explicitly unknown
[ ] engine detection is evidence-based
[ ] SDK detection is evidence-based
[ ] firmware compatibility is evidence-based
[ ] NID knowledge layer works
[ ] unknown NIDs are researchable
[ ] shader analysis works on real corpus
[ ] dependency graph is real and queryable
[ ] ABI signatures have provenance
[ ] CLI exposes major subsystems
[ ] evidence is visible in dashboard
[ ] validation-report.json exists
[ ] validation-report.md exists
[ ] workspace tests pass
[ ] clippy passes
[ ] fmt passes
[ ] documentation matches reality

At the end, update:

IMPLEMENTATION_AUDIT.md
UPGRADE_PLAN.md
This file.

Do not mark unchecked items complete.

The audit and source code, not the old plan, are the authority.