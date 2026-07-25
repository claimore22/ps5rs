# ps5rs

A PS5 binary analysis toolkit written in Rust. Parses SELF/ELF containers, resolves NID imports, and analyzes collections of PS5 game dumps.

## Why Rust?

The PS5 runs on x86-64 (AMD Zen 2), so PS5 game code can execute natively on any modern PC without CPU emulation. The hard part is everything around the CPU: parsing Sony's SELF/ELF module format, resolving NID-hashed imports, HLE of system libraries, and ABI translation. Rust eliminates an entire class of memory safety bugs (buffer overflows, use-after-free, data races, null pointer dereferences) that plague unsafe native code, while matching C++ performance. For a project that parses untrusted binary data from third-party game dumps, this safety guarantee is critical.

## Crates

| Crate | Purpose |
|---|---|
| `ps5-format` | Shared types, error enums, ELF/SELF constants |
| `ps5-self` | SELF container parsing (PS4/PS5 wrappers around ELF) |
| `ps5-elf` | ELF64 binary format parsing (headers, segments, symbols, relocations) |
| `ps5-nid` | NID hash algorithm (SHA1 + Sony custom base64), name catalog, resolver |
| `ps5-analysis` | Analysis engine: collect imports, build heatmaps, dependency graphs |
| `ps5-cli` | Command-line interface (inspect, imports, segments, analysis) |

## Building

```sh
cargo build --release
```

## Usage

```sh
# Inspect a SELF/ELF binary
ps5-cli inspect path/to/eboot.bin

# List imports with NID resolution
ps5-cli imports path/to/eboot.bin

# Analyze a directory of game dumps
ps5-cli analyze collect --dir /path/to/games
ps5-cli analyze stats --dir /path/to/games
ps5-cli analyze heatmap --dir /path/to/games
```

## Analysis

The `ps5-analysis` crate collects import data from game dumps and produces:

- **Statistics**: total imports, unique NIDs, resolution rate, most common symbols
- **Heatmap**: library x game usage matrix
- **Frequency**: NID usage ranking across all games
- **Unresolved**: NIDs not found in the name catalog
- **Dependency graph**: game -> library -> NID relationships (Graphviz DOT export)

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
