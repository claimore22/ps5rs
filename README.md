# ps5rs

A PS5 binary analysis toolkit written in Rust. Parses SELF/ELF containers, resolves NID imports, extracts clean ELFs, and analyzes collections of PS5 game dumps.

## Why Rust?

The PS5 runs on x86-64 (AMD Zen 2), so PS5 game code can execute natively on any modern PC without CPU emulation. The hard part is everything around the CPU: parsing Sony's SELF/ELF module format, resolving NID-hashed imports, HLE of system libraries, and ABI translation. Rust eliminates an entire class of memory safety bugs (buffer overflows, use-after-free, data races, null pointer dereferences) that plague unsafe native code, while matching C++ performance. For a project that parses untrusted binary data from third-party game dumps, this safety guarantee is critical.

## Crates

| Crate | Purpose |
|---|---|
| `ps5-format` | Shared types, error enums, ELF/SELF constants, SHA-256 utility |
| `ps5-self` | SELF container parsing (PS4/PS5 wrappers around ELF) |
| `ps5-elf` | ELF64 binary format parsing (headers, segments, symbols, relocations) |
| `ps5-nid` | NID hash algorithm (SHA1 + Sony custom base64), name catalog, resolver |
| `ps5-image` | BinaryImage IR: normalized abstraction with JSON serialization |
| `ps5-analysis` | Analysis engine: scanner, dataset, batch extraction, reports, export |
| `ps5-dashboard` | Static HTML dashboard generator (self-contained, no CDN dependencies) |
| `ps5-cli` | Command-line interface |

## Building

```sh
cargo build --release
```

## Quick Start

The full analysis pipeline — scan game dumps, extract clean ELFs, validate, and generate a dashboard:

```sh
# 1. Scan games into a dataset
ps5rs scan ./games --output analysis/

# 2. Extract clean ELFs from SELF containers
ps5rs batch-extract ./games --output analysis/

# 3. Validate dataset (SHA256 cross-reference)
ps5rs validate analysis/

# 4. Run analysis reports
ps5rs analyze stats analysis/
ps5rs analyze imports analysis/
ps5rs analyze engines analysis/

# 5. Generate interactive dashboard
ps5rs dashboard analysis/
```

## Usage

### Scan a game directory (creates dataset)

The primary workflow: scan game binaries once, then run any number of reports instantly without re-parsing.

```sh
ps5rs scan ./games --output analysis/
```

Each game's `eboot.bin` is parsed into a `BinaryImage` and serialized as an individual JSON file. The manifest tracks schema version and game metadata (from `sce_sys/param.json`) for forward compatibility.

```
analysis/
  manifest.json       # schema v2: tool, timestamp, image count, game metadata
  images/
    GameTitle.json    # BinaryImageDocument per game
    AnotherGame.json
```

### Extract clean ELFs from SELF containers

Sony's SELF format wraps ELF code segments with encryption and metadata. The `extract` commands produce clean, standard ELF files that tools like `readelf` can analyze.

```sh
# Extract a single binary
ps5rs extract path/to/eboot.bin -o output.elf

# Batch extract all games
ps5rs batch-extract ./games --output analysis/
```

Batch extract produces:
```
analysis/extracted/
  manifest.json       # SHA256 linkage between source and extracted ELFs
  Game1.elf
  Game2.elf
  ...
```

### Validate a dataset

Cross-references dataset images with extracted ELFs via SHA256 checksums.

```sh
ps5rs validate analysis/
```

Produces `analysis/reports/validation.json` with per-game pass/fail status, NID resolution rates, and SHA256 match results.

### Generate a dashboard

Produces a self-contained HTML dashboard with no external dependencies — works offline.

```sh
ps5rs dashboard analysis/
# Output: analysis/dashboard/index.html
```

The dashboard includes:
- **Overview cards**: total games, imports, unique NIDs, resolution rate
- **Games table**: sortable by name, segments, imports, NID resolution, memory segments
- **Library heatmap**: log-scaled import frequency across all games
- **NID analysis**: pie chart (resolved vs unknown), top 20 NIDs by frequency
- **Library NID breakdown**: top 10 NIDs per library with resolved names
- **Segment sizes**: stacked bar chart (RX/R/RW/Other) per game
- **Library priority**: sorted by game count with unique NID counts

### Analyze a dataset

All `analyze` subcommands read from a dataset directory (no binary parsing). If a dataset doesn't exist, they fall back to scanning from raw binaries.

```sh
# Statistics (total imports, resolution rate, most common NID)
ps5rs analyze stats analysis/

# Import inventory: which libraries are used by how many games
ps5rs analyze imports analysis/

# Unknown NIDs: unresolved hashes sorted by frequency (catalog growth targets)
ps5rs analyze unknown analysis/

# Library x game heatmap
ps5rs analyze heatmap analysis/

# NID frequency ranking (top 50)
ps5rs analyze frequency analysis/

# Unresolved NIDs (per game)
ps5rs analyze unresolved analysis/

# Dependency graph (Graphviz DOT or JSON)
ps5rs analyze graph analysis/ -o graph.dot
ps5rs analyze graph analysis/ --include-nids --format json -o graph.json

# Library versions across games
ps5rs analyze library-versions analysis/

# Engine detection (Unity/Unreal/Godot/native)
ps5rs analyze engines analysis/
```

All report commands accept `--format` (terminal/csv/json/dot) and `-o` (output file). Add `--include-modules` to `scan` or `analyze` to include PRX system modules alongside `eboot.bin`.

### Inspect a binary

```sh
ps5rs inspect path/to/eboot.bin
```

Shows platform, SELF segments (offsets, sizes, flags), ELF header fields (entry point, program headers, symbols, relocations, TLS), and a summary of imports by library + resolved name.

### List imports

```sh
ps5rs imports path/to/eboot.bin
```

Lists all NID imports with resolved function names and source libraries.

### Hash a function name to NID

```sh
ps5rs nid sceKernelLoadStartModule
# -> 4ZjF4RQH3k8
```

Computes the NID hash for a given function name using Sony's SHA1 + custom base64 algorithm.

## Dataset Structure

```
analysis/
  manifest.json               # schema v2, tool version, game metadata (param.json)
  images/
    GameTitle.json            # BinaryImageDocument per game (BinaryImage IR)
  reports/
    validation.json           # SHA256 cross-reference results
    engine_hints.json         # Engine detection (Unity/Unreal/Godot/native)
    library_versions.json     # Aggregated library versions
  extracted/
    manifest.json             # SHA256 linkage between source and extracted ELFs
    GameTitle.elf             # Clean ELF extracted from SELF
  dashboard/
    index.html                # Self-contained interactive dashboard
```

Expected game dump directory:
```
/path/to/games/
  GameTitle-PPSA00000/
    sce_sys/param.json        # Game metadata (title, version, SDK)
    eboot.bin                 # main game binary (SELF or raw ELF)
  AnotherGame-PPSA00001/
    sce_sys/param.json
    eboot.bin
```

By default only `eboot.bin` is analyzed. Add `--include-modules` to include PRX modules in `sce_module/`.

## Architecture

```
                    eboot.bin (SELF or ELF)
                           |
              +------------+------------+
              |                         |
         ps5-self                  ps5-elf
      (SELF → clean ELF)       (ELF parsing)
              |                         |
              +------------+------------+
                           |
                      ps5-image
                   (BinaryImage IR)
                           |
              +------------+------------+
              |                         |
           scan /                  extract /
        batch-extract             batch-extract
              |                         |
       AnalysisDataset           Extracted ELFs
              |                         |
    +---------+---------+        validate (SHA256)
    |    |    |    |    |
  stats  imports  heatmap  ...  dashboard
```

The `BinaryImage` IR decouples raw parsers from consumers. The `scan` command produces a dataset of `BinaryImageDocument` JSON files. All `analyze` reports and the `dashboard` command consume the dataset without touching raw binaries, making iteration fast and portable.

## Analysis Reports

**Dataset reports** (read from JSON files, no binary parsing):
- **Statistics**: total imports, unique NIDs, resolution rate, most common symbols
- **Import inventory**: which libraries are used by how many games and how often
- **Unknown NIDs**: unresolved hashes sorted by frequency (directly feeds catalog growth)
- **Heatmap**: library x game usage matrix
- **Frequency**: NID usage ranking across all games
- **Unresolved**: NIDs not found in the name catalog (per game)
- **Dependency graph**: game -> library -> NID relationships (Graphviz DOT export)
- **Library versions**: aggregated SDK/library versions across games
- **Engine hints**: detects game engines from library patterns (Unity, Unreal, Godot)
- **Validation**: SHA256 cross-reference between dataset images and extracted ELFs

Export formats: JSON (versioned), CSV, Graphviz DOT.

## Dashboard

The `ps5-dashboard` crate generates a self-contained HTML file with all data embedded as a JSON blob. No CDN, no external dependencies — fully offline.

- Dark theme, responsive layout
- Sortable tables with filtering
- Log-scaled heatmap (handles skewed import counts: 12K libkernel vs 40 libSceAudio3d)
- CSS-only charts (conic-gradient pie, stacked bars, horizontal bar charts)
- Vanilla JS for interactivity (no frameworks)

## NID Database

The CLI ships with an embedded NID database (`data/nids.csv`) containing ~154K hash-to-name mappings with 97.8% resolution rate across 28 tested games. This file is not tracked in git -- you must provide it for building.

## Test Suite

268 tests across 8 crates covering ELF parsing, SELF extraction, NID hashing, BinaryImage IR, analysis reports, dataset operations, batch extraction, and dashboard generation.

## Acknowledgements

### Acelogic 
Proposed the idea of a NId heatmap on SharpEmu Discord.

### Prosper (ps5ys)

[Prosper](https://github.com/mattias800/ps5ys) by mattias800 is a user-space PS5 to PC compatibility layer -- think Proton/Wine, but for PS5. It runs PS5 game binaries on Linux and Windows by reimplementing the console's OS, ABI, and GPU stack natively, without CPU emulation. Prosper was the primary inspiration for this project's SELF/ELF parsing approach and NID resolution strategy. Its clean-room reimplementation of Sony's system libraries demonstrates that PS5 binary compatibility is achievable through careful reverse engineering without using any copyrighted Sony code.

### SharpEmu

[SharpEmu](https://github.com/sharpemu/sharpemu) is an experimental PS5 emulator written in C#, developed from scratch for Windows, Linux and macOS. Created by par274, it focuses on accuracy and infrastructure for PS5 system-level emulation. The NID name database used in this project was sourced from SharpEmu's community-maintained catalog. Licensed under GPL-2.0.

### Kyty / KytyPS5

[Kyty](https://github.com/InoriRus/Kyty) by InoriRus is one of the earliest PS4/PS5 emulator projects, providing valuable reference material for understanding PS5 native code execution and system call handling. The project has since been forked and extended by the community as [KytyPS5](https://github.com/KytyPS5/KytyPS5) (C++/Vulkan, GPL-2.0). Both projects have been instrumental in advancing public understanding of the PS5 platform.

### ShadPS4

[ShadPS4](https://github.com/shadps4-emu/shadPS4) is a mature PS4 emulator that pioneered many approaches to PS4/PS5 binary format handling and native code execution that influenced the wider PS5 emulation community.

## License

GPL-2.0-only. See [LICENSE](LICENSE) for details.
