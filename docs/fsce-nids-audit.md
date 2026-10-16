# fsce-nids Audit Report

New `ps5rs fsce-nids` command: recursive `.prx`/`.sprx` scan for
`^sce[A-Z]` symbols with NIDs computed by the verified workspace algorithm.
Status: implemented, tested, validated. `fprx-nids` and all other commands
untouched (only shared-walker visibility widened; behavior unchanged).

## 1. Files changed

| File | Change |
|---|---|
| `crates/ps5-cli/src/fsce_nids.rs` | New: token scanner, `nid_forms`, `FsceError`, streaming CSV writer, `run_scan`/`cmd_fsce_nids`, 14 inline tests |
| `crates/ps5-cli/src/cli.rs` | New `FsceNids` variant (`path`, `-o/--output`) |
| `crates/ps5-cli/src/main.rs` | `mod fsce_nids` + dispatch arm |
| `crates/ps5-cli/src/fprx_nids.rs` | `Collected`/`FailedFile`/`collect_prx_files` widened to `pub(crate)` (no logic change) |
| `docs/fsce-nids-audit.md` | This report (new) |

No dependency changes. `data/nids.csv` is never written by this command:
`run_scan` refuses any output path named `nids.csv`
(`FsceError::CatalogOverwriteRefused`, before any file is created), and a
test proves a decoy `nids.csv` inside the scanned tree keeps its exact bytes.
SHA-256 of `data/nids.csv` verified identical before/after validation runs.

## 2. CLI usage

```powershell
ps5rs.exe fsce-nids <folder>
ps5rs.exe fsce-nids <folder> --output <csv-path>
ps5rs.exe fsce-nids <folder> --include-eboot
```

Default output: `<folder>\fsce-nids.csv`. Missing directory, empty module set,
and `nids.csv` outputs are hard errors (exit 1, nothing written). Unreadable
files are listed with reasons and mark the scan `INCOMPLETE` (outputs still
written, exit 1).

## 3. NID algorithm and verification

Reused, not reinvented. Two independent implementations agree byte-for-byte:

- Workspace: `ps5_nid::hash(name)` = SHA-1(`name_bytes` + 16-byte salt
  `51 8D 64 A6 … E5 52 30`), reverse the first 8 digest bytes, custom-base64
  (`A-Za-z0-9+-`) → 11 chars (`crates/ps5-nid/src/algorithm.rs`).
  `ps5_nid::nid_to_u64` decodes the text back to `u64`.
- AnyPS5 (`Documents/Rust/AnyPS5`, `core/libs/nid/src/NidCompute.cpp`):
  identical salt, identical reversal, identical alphabet, identical 11-char
  layout. Its `ComputeNid(symbolName, libraryName)` **ignores `libraryName`**
  (`NidResolver.cpp` even passes `""`), proving symbol-only computation is
  correct — no library context is fabricated.

Therefore every row carries `algorithm_status = "verified"` (no provisional
path was needed), `nid` = numeric `0x%016x`, `nid_encoding` = 11-char text.

Known pairs tested (all pass, cross-checked against `data/nids.csv` rows
including `nid_hex`):

| symbol | text | numeric |
|---|---|---|
| `memcpy` | `Q3VBxCXhUHs` | — |
| `sceKernelSleep` | `-ZR+hG7aDHw` | `0xfd947e846eda0c7c` |
| `scePthreadCreate` | `6UgtwV+0zb4` | `0xe9482dc15fb4cdbE` |
| `sceKernelAddUserEvent` | `4R6-OvI2cEA` (ps5-nid vector) | — |

## 4. Detection rules

Token scan over raw bytes (no whole-file decode, exact offsets): `sce` or
`Sce` followed by an uppercase letter at a position whose previous byte is
not `[A-Za-z0-9_]` nor glue `#@.:$~`; maximal `[A-Za-z0-9_]*` extension; the
following byte must not be a symbol or glue byte either. `SCE`, `sCe`, and
`scE` forms stay rejected. This accepts `call sceKernelFoo now` and
`...sceLibcMspaceMallocStats().` but rejects `sceKernelFoo#lib` (no suffix
stripping), `ns::sceKernelFoo` (no namespace stripping), `sceKernelFoo@1`,
`sceFoo.prx`, `xsceFoo`, and `a.sceFoo`. No NUL terminator required; offset-0
and exact-EOF matches handled.

Every occurrence is a row — no dedup within/across files (explicitly tested).

## 5. CSV schema

`symbol,nid,nid_encoding,module_name,relative_path,file_offset,algorithm_status`
— `file_offset` is `0x` hex. Header always written, even with zero findings.
`csv` crate escaping (covered with a comma-filename round-trip test). Rows
stream per file in sorted-file order, which equals the required
(relative path, file offset, symbol) order without accumulating rows.

## 6. Validation runs

Smoke (Stray, 7 modules, 8 backups skipped, 0 failed): 21 occurrences —
20 `Sce*` names (`SceLibcHeap`, `SceKernelInternalMemory`,
`SceNpCppWebApi*`, `SceJobManager*`, `SceGameRight`, …) plus
`sceLibcMspaceMallocStats` @ `0xefa8f` in `libc.prx`, tokenized from the log
sentence `...before sceLibcMspaceMallocStats().` (a run-level matcher would
have missed it). `--include-eboot` adds `eboot.bin` (case-insensitive) to
the walk; Stray smoke: 7 → 8 modules, 21 → 30 occurrences (incl. `sceAjm*`
symbols from the eboot).

Full corpus (34 games, current `[sS]ce[A-Z]` build, defaults):

- Modules: 530 found, 530 scanned, 0 failed, 124 backup(s) skipped.
- SCE symbol occurrences: 1,876 rows (239 distinct symbols, repeats kept —
  no dedup within or across files), all `algorithm_status = verified`.
- Report: `analysis/fsce-full.csv` (header + 1,876 rows). No `INCOMPLETE`
  flag; exit code 0.

## 7. Tests

14 focused tests in `fsce_nids.rs`, all passing: prefix accept/reject matrix
(upper/lower/digit/`sce`-less), glue rejection both sides, dash/space
delimiters, SOF/EOF boundaries, multi-symbol files, repeat preservation (3
occurrences → 3 records), cross-file non-dedup, mixed-case extensions,
recursion, CSV escaping + sort order, header-only empty result, bad inputs,
injected read failure, `nids.csv` refusal + decoy preservation, trusted
algorithm vectors incl. DB `nid_hex` values.

Workspace: `cargo fmt --check --all` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean,
`cargo test --workspace` all green (65 `ps5-cli` tests incl. the 14 new ones).

## 8. Known limitations

- Dotted/versioned/suffixed names (`sceFoo@1`, `sceFoo.prx`, `a.sceFoo`)
  are rejected rather than stripped — conservative by design.
- Large corpora yield one row per occurrence (no dedup by design); CSVs can
  grow into the millions of rows.
- `nid` numeric form is ps5-nid's decode of the text form; it matches
  `data/nids.csv` `nid_hex` on tested pairs.
