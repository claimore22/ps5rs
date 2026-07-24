# ps5rs Naming Conventions

## Principles

1. **Standard ELF terminology is NOT renamed.** Use `ElfHeader`, `ProgramHeader`,
   `Relocation`, `DynamicEntry` — the same names used by LLVM, GNU binutils, LIEF,
   goblin, pyelftools. Contributors already know these.

2. **Sony-specific structures get new names.** Where the format diverges from standard
   ELF, we invent clear names:
   - `SelfImage` (not `SceSelfHeader`) — the top-level SELF container
   - `SelfSegmentEntry` — per-segment metadata in the SELF wrapper
   - `SceDynamicTag` — Sony-extended dynamic tags (`DT_SCE_*`)

3. **No SDK-derived names copied verbatim.** The SDK headers (`sce_elf.h`, `self.h`)
   are reference for understanding. We document what fields mean independently.

4. **Rust idioms over C conventions.**
   - `Error` enums, not error codes
   - `Option<T>` over nullable pointers
   - Zero-copy `&'a [u8]` borrows over owned `Vec<u8>`
   - Builder pattern for complex construction
   - Module-level visibility (`pub(crate)`) over `pub` everything

## Crate Naming

| Crate | Purpose |
|---|---|
| `ps5-format` | Shared types, error enums, constants |
| `ps5-self` | SELF container parsing (wrapper around ELF) |
| `ps5-elf` | ELF64 binary format parsing |
| `ps5-nid` | NID hash algorithm, name catalog, resolver |
| `ps5-cli` | Command-line interface (inspect, imports, etc.) |

## Type Naming

### ps5-format

```rust
pub enum ParseError { ... }        // NOT "SceError" or "ElfError"
pub struct Segment { ... }         // Generic segment (used by both SELF and ELF)
pub struct Relocation { ... }      // Standard relocation entry
pub struct SymbolEntry { ... }     // Dynamic symbol
pub struct DynamicEntry { ... }    // Dynamic section entry
```

### ps5-self

```rust
pub struct SelfImage<'a> { ... }          // Top-level parsed SELF container
pub struct SelfSegmentEntry { ... }       // Per-segment metadata from SELF header
pub struct SelfSegmentFlags(u64);         // Bitflags for segment type/properties
```

### ps5-elf

```rust
pub struct ElfImage<'a> { ... }           // Parsed ELF64 binary
pub struct ElfHeader { ... }              // Standard ELF header (e_ident, e_type, etc.)
pub struct ProgramHeader { ... }          // Standard program header (p_type, p_flags, etc.)
pub struct SectionHeader { ... }          // Standard section header (if needed)
pub struct RelaEntry { ... }              // RELA relocation entry
pub struct SymEntry { ... }              // Symbol table entry
pub struct DynEntry { ... }              // Dynamic section entry
pub struct TlsInfo { ... }              // TLS metadata from PT_TLS
```

### ps5-nid

```rust
pub fn hash(name: &str) -> String        // NOT "compute_nid" or "sceNidHash"
pub struct Catalog { ... }               // NOT "NidDb" or "NidDatabase"
impl Catalog { pub fn resolve(&self, nid: &str) -> Option<&str> }
```

## ELF Constants (NOT renamed)

These are standard and must stay standard:

```rust
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_TLS: u32 = 7;

pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;

pub const R_X86_64_RELATIVE: u32 = 8;
pub const R_X86_64_GLOB_DAT: u32 = 6;
pub const R_X86_64_JUMP_SLOT: u32 = 7;
pub const R_X86_64_DTPMOD64: u32 = 16;
pub const R_X86_64_DTPOFF64: u32 = 17;
pub const R_X86_64_TPOFF64: u32 = 18;
```

## Sony-specific constants (renamed for clarity)

```rust
pub const PT_SCE_DYNLIBDATA: u32 = 0x61000000;
pub const PT_SCE_PROCPARAM: u32 = 0x61000001;
pub const PT_SCE_RELRO: u32 = 0x61000010;

pub const DT_SCE_JMPREL: u64 = 0x61000029;
pub const DT_SCE_PLTRELSZ: u64 = 0x6100002D;
pub const DT_SCE_RELA: u64 = 0x6100002F;
pub const DT_SCE_RELASZ: u64 = 0x61000031;
pub const DT_SCE_STRTAB: u64 = 0x61000035;
pub const DT_SCE_STRSZ: u64 = 0x61000037;
pub const DT_SCE_SYMTAB: u64 = 0x61000039;
pub const DT_SCE_SYMTABSZ: u64 = 0x6100003F;

pub const SELF_MAGIC_PS4: u32 = 0x4F153D1D;
pub const SELF_MAGIC_PS5: u32 = 0x5414F5EE;
```

## File Layout

```
crates/
  ps5-format/src/lib.rs      # Shared types + error enum
  ps5-self/src/lib.rs         # SelfImage parser
  ps5-self/src/segment.rs     # SelfSegmentEntry + flags
  ps5-elf/src/lib.rs          # ElfImage parser
  ps5-elf/src/header.rs       # ElfHeader
  ps5-elf/src/program.rs      # ProgramHeader
  ps5-elf/src/dynamic.rs      # Dynamic section parsing
  ps5-elf/src/relocation.rs   # Relocation parsing
  ps5-elf/src/symbol.rs       # Symbol table parsing
  ps5-nid/src/lib.rs          # hash() function
  ps5-nid/src/catalog.rs      # Catalog (name ↔ NID database)
  ps5-nid/src/algorithm.rs    # SHA1 + Sony base64
  ps5-cli/src/main.rs         # Entry point
  ps5-cli/src/inspect.rs      # "inspect" subcommand
  ps5-cli/src/imports.rs      # "imports" subcommand
  ps5-cli/src/segments.rs     # "segments" subcommand
```

## Error Handling

All crates return `Result<T, ps5_format::ParseError>`. Never `String`, never `Box<dyn Error>`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid magic bytes: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },

    #[error("unsupported version: {0}")]
    UnsupportedVersion(u16),

    #[error("file truncated: need {needed} bytes at offset {offset}, have {available}")]
    Truncated { offset: u64, needed: u64, available: u64 },

    #[error("unsupported ELF class: {0:?}")]
    UnsupportedClass(ElfClass),

    #[error("not an x86-64 binary (machine: {0:#x})")]
    NotX86_64(u16),

    #[error("missing required program header: {0}")]
    MissingProgramHeader(&'static str),
}
```

## Zero-Copy

All parser types borrow from the input `&'a [u8]`:

```rust
pub struct ElfImage<'a> {
    data: &'a [u8],           // borrow the original bytes
    header: ElfHeader,
    program_headers: Vec<ProgramHeader>,
    // ...
}
```

The CLI reads the file into a `Vec<u8>`, then passes `&bytes` to parsers.
