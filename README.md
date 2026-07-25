# ps5rs

A PS5 binary analysis toolkit written in Rust. Parses SELF/ELF containers, resolves NID imports, and analyzes collections of PS5 game dumps.

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
| `ps5-analysis` | Analysis engine: scanner, dataset, reports, export |
| `ps5-cli` | Command-line interface (inspect, scan, analyze) |

## Building

```sh
cargo build --release
```

## Usage

```sh
# Build
cargo build --release

# Binary is at:
./target/release/ps5rs.exe   # Windows
./target/release/ps5rs       # Linux/macOS
```

### Scan a game directory (creates dataset)

The primary workflow: scan game binaries once, then run any number of reports instantly without re-parsing.

```sh
# Scan games into a dataset directory
ps5rs scan ./games --output analysis/

# The dataset is a portable directory of JSON files:
# analysis/
#   manifest.json       # schema version, tool, timestamp, image count
#   images/
#     GameTitle.json     # BinaryImageDocument per game
#     AnotherGame.json
```

Each game's `eboot.bin` is parsed into a `BinaryImage` and serialized as an individual JSON file. The manifest tracks schema version for forward compatibility.

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
```

All report commands accept `--format` (terminal/csv/json/dot) and `-o` (output file).

### Inspect a binary

```sh
ps5rs inspect path/to/eboot.bin
```

Shows platform, SELF segments (offsets, sizes, flags), ELF header fields (entry point, program headers, symbols, relocations, TLS), and a summary of imports by library + resolved name.

```
ps5rs v0.1.0 — PS5 binary inspector
File: eboot.bin
Size: 52428800 bytes

Platform: Ps5
SELF segments: 4
  [0] offset=0x1000 file_size=0x3a0000 mem_size=0x3a0000 flags=DATA
  [1] offset=0x3a1000 file_size=0x1800000 mem_size=0x1800000 flags=CODE
  ...

ELF type: 0x3
Entry point: 0x800001000
Imports: 1423
```

### List imports

```sh
ps5rs imports path/to/eboot.bin
```

Lists all NID imports with resolved function names and source libraries.

```
NID                                                    Resolved          Library
---------------------------------------------------------------------------
mFq1M6vw-JM                                            sceKernelLoad     libkernel
...
```

### Show segments

```sh
ps5rs segments path/to/eboot.bin
```

Lists ELF program headers with type, flags (RWX), offsets, vaddr, file/mem sizes. Also shows SELF data segment mappings when present.

### Show dynamic entries

```sh
ps5rs dynamic path/to/eboot.bin
```

Lists all dynamic section entries (DT_NEEDED, DT_STRTAB, DT_SCE_* tags, etc.) and resolved import library names.

### Show symbols

```sh
ps5rs symbols path/to/eboot.bin
```

Lists symbol table entries with section index, bind type, value, size, and resolved names.

### Hash a function name to NID

```sh
ps5rs nid sceKernelLoadStartModule
```

Computes the NID hash for a given function name using Sony's SHA1 + custom base64 algorithm.

```
sceKernelLoadStartModule -> 4ZjF4RQH3k8
```

### Analyze a game directory (legacy)

The `analyze collect` command still works directly against raw game dumps (without creating a dataset):

```sh
ps5rs analyze collect /path/to/games -o database.json
```

Expected directory structure:
```
/path/to/games/
  GameTitle-PPSA00000/
    eboot.bin           # main game binary
    sce_module/         # optional PRX modules
      libc.prx
      libScePfs.prx
  AnotherGame-PPSA00001/
    eboot.bin
```

By default only `eboot.bin` is analyzed (PRX modules in `sce_module/` are skipped). Add `--include-prx` to the collector if you want system modules included.

## Architecture

```
                 eboot.bin
                     |
                     v
              ps5-image builder
                     |
                     v
             BinaryImage JSON
                     |
                     v
              AnalysisDataset
                     |
        +------------+-------------+
        |            |             |
      stats      inventory     heatmaps
```

The `BinaryImage` IR decouples raw parsers from consumers. The `scan` command produces a dataset of `BinaryImageDocument` JSON files. All `analyze` reports consume the dataset without touching raw binaries, making iteration fast and portable.

## Analysis Reports

The `ps5-analysis` crate provides two report pipelines:

**Dataset reports** (read from JSON files, no binary parsing):
- **Statistics**: total imports, unique NIDs, resolution rate, most common symbols
- **Import inventory**: which libraries are used by how many games and how often
- **Unknown NIDs**: unresolved hashes sorted by frequency (directly feeds catalog growth)
- **Heatmap**: library × game usage matrix
- **Frequency**: NID usage ranking across all games
- **Unresolved**: NIDs not found in the name catalog (per game)
- **Dependency graph**: game → library → NID relationships (Graphviz DOT export)

Export formats: JSON (versioned), CSV, Graphviz DOT.

## NID Database

The CLI ships with an embedded NID database (`data/nids.csv`) containing ~154K hash-to-name mappings. This file is not tracked in git -- you must provide it for building.

## Acknowledgements

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
