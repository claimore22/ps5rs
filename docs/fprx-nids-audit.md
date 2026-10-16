# fprx-nids Audit Report

Upgraded `ps5rs fprx-nids` from a flat NID lister into an NID + ASCII-string
correlation command. Status: implemented, tested, validated.

## 1. Files changed

| File | Change |
|---|---|
| `crates/ps5-cli/src/fprx_nids.rs` | Rewritten: strict NID matcher, string extractor, classifier, correlator, `ScanError`, two CSV writers, 26 inline tests |
| `crates/ps5-cli/src/cli.rs` | `FprxNids` variant: replaced `--with-path`/`--with-offsets` with `--nearby-bytes`, `--min-string-length`, `--max-candidates-per-nid`, `--nids`, `--strings-output`; `-o/--output` now defaults to `<path>/fprx-nids.csv` instead of stdout |
| `crates/ps5-cli/src/main.rs` | Dispatch updated for the new arguments |
| `crates/ps5-cli/Cargo.toml` | Added `thiserror = "2"` (already used by `ps5-format`, `ps5-prx`, `ps5-ble-loader`, `ps5-memory-safe`) |
| `docs/fprx-nids-audit.md` | This report (new) |

No other command was touched. No new workspace-level dependencies.

## 2. Command syntax

```powershell
ps5rs.exe fprx-nids <PATH> [OPTIONS]

Options:
  -o, --output <OUTPUT>                 Primary CSV (default: <PATH>/fprx-nids.csv)
      --nearby-bytes <N>                Nearby window in bytes [default: 4096]
      --min-string-length <N>           Minimum ASCII run length [default: 4, min: 1]
      --max-candidates-per-nid <N>      Cap of nearby rows per NID, nearest first [default: 25]
      --nids <FILE>...                  Extra NID catalog CSVs (repeatable)
      --strings-output <FILE>           Also write the full string inventory CSV
      --unknown-only                    Only emit NIDs missing from the catalog
      --include-eboot                   Also scan eboot.bin files
```

`--unknown-only` filters rows to NIDs the active catalog cannot resolve —
builtins plus `data/nids.csv` plus any `--nids` extras (custom sources slot
in the same way). Known NIDs are dropped entirely (no nearby/same-file/
known rows for them); the stderr summary reports the filtered count
(`unknown-only: N catalog-known nid(s) filtered out`). Validated on Stray:
48,891 NIDs → 48,890 filtered → 20 unknown rows, 0 known rows.

`--include-eboot` adds `eboot.bin` files (case-insensitive, backups still
skipped) to the walk. Validated on Stray: 7 → 8 modules, +495 NIDs
(+12,878 rows) from the 149 MB eboot.

Behavioral changes from the previous version (documented, intentional):

- Default output is a file (`<root>/fprx-nids.csv`), no longer stdout.
  Pass `-o` explicitly to keep scan trees read-only.
- NID matching is strict (see §3); the old loose `seg#seg` tails are gone.
- Output schema replaced by the correlation schema (see §4).

## 3. NID recognition (strict)

Exactly 15 bytes: 11 × `[A-Za-z0-9+/−]`, `#`, one `[A-Z]`, `#`, one `[A-Z]`
(e.g. `amVp+qf6SDk#D#A`). The byte before the match must not be `#` or an
NID-alphabet byte, so longer runs cannot match by suffix. No NUL terminator
required. Matches are deduplicated per file (first offset kept) and kept
across files.

Consequence: real-world symbols with non-letter suffixes such as
`bzQExy189ZI#3#1` are rejected as malformed under these rules. They remain
visible to ELF import-table parsing (`ps5rs symbols`/`imports`), which is the
stronger mechanism (see §8).

## 4. CSV schemas

Primary report (`nid,suffix,module_name,relative_path,nid_offset,
matched_ascii_string,string_offset,offset_distance,string_category,
match_type,confidence,mapping_source`):

- Offsets are `0x` hex; `offset_distance` is a decimal byte difference.
- `match_type`: `nearby_candidate` (confidence `low`), `same_file_candidate`
  (`none`), `unmatched` (empty confidence), `known_mapping` (`high`).
- Non-nearby rows leave candidate fields empty. `mapping_source` is set only
  for `known_mapping` rows (catalog `sources`, else `builtin-catalog`).
- Sorted by relative path, NID offset, candidate offset (empty first), NID,
  candidate string. `csv` crate handles quoting.

String inventory (`--strings-output`):
`string,string_offset,string_length,string_category,module_name,relative_path`.

`string_category` priority: `compat_wrapper_candidate`
(`fmodstudio_real_*`) → `sce_kernel_api_candidate` (`sceKernel*`) →
`fmod_api_candidate` (contains `fmod` case-insensitively, or
`FMOD::`/`EventDescription::`/`EventInstance::`/`Studio::` prefix) →
`sce_api_candidate` (`sce*`) → `cpp_method` (`UpperHead::tail`) → `other`.
Heuristics only; classification never alters the string.

## 5. Correlation rules

- Window search is binary-indexed (`partition_point`), not a Cartesian product.
- A string overlapping the NID's own 15 bytes is never its own candidate.
- Strings that are themselves exactly NID-shaped stay in the inventory but are
  excluded from candidacy, so capped windows are not filled with neighboring
  symtab entries.
- A NID with in-window candidates emits one row per candidate (ambiguity kept).
  A NID with strings in-file but none nearby emits one `same_file_candidate`
  row; a NID in a string-less file emits one `unmatched` row.
- `--max-candidates-per-nid` (default 25) keeps the nearest candidates when a
  window overflows; overflow is counted and reported, never silent.
- `known_mapping` rows come from `ps5_nid::Catalog::resolve` on the 11-char
  hash (the catalog's documented key: `hash(name)`), with library/source
  metadata quoted verbatim. Suffix letters are recorded, never interpreted as
  library proof.

## 6. Errors and completeness

`ScanError` (`thiserror`): `RootNotDirectory`, `NoModules`,
`BadMinLength`, `ReportCreate`, `ReportWrite`. No `unwrap`/`panic` in
non-test code. Modules are processed one file at a time; only report rows
accumulate. Unreadable files and unreadable directories are recorded with
reasons, listed on stderr, and mark the scan `INCOMPLETE` (outputs are still
written, exit code is 1). Backup files (`*.esbak/*.bak/*.orig`) are counted
as skipped, not failures.

## 7. Validation runs

Smoke (Stray, 7 modules, 8 backups skipped, 0 failed):

- NID-shaped strings: 48,891; printable strings: 101,140.
- Rows: 109,580 (12,785 nearby / 47,903 same-file / 0 unmatched / 48,892 known).
- Cap overflow: 1,046 pairs across 55 NIDs.
- Spot checks: CSV NID count for `libSceNpCppWebApi.prx` equals its symtab
  size minus the empty entry; bytes re-read at reported offsets match.

Full corpus (34 games under `ROMS/PS5`, release binary, defaults):

- Modules: 530 found, 530 scanned, 0 failed, 124 backup(s) skipped.
- NID-shaped strings: 1,717,542; printable strings: 9,827,040.
- Rows: 4,327,387 (999,011 nearby / 1,641,861 same-file / 0 unmatched /
  1,686,515 known).
- Cap overflow: 520,581 pairs beyond `--max-candidates-per-nid 25` across
  11,086 NIDs, nearest kept, explicitly reported on stderr.
- Report: `analysis/fprx-full.csv`. No `INCOMPLETE` flag; exit code 0.

## 8. Tests

26 focused tests in `fprx_nids.rs` (all passing): strict accept/reject matrix
(slash alphabet, digit/lowercase suffixes, length variants, prefix boundaries,
EOF-exact, no-NUL), offset-exact string extraction (SOF/EOF runs, high bytes,
min-length levels), all six categories plus priority, mixed-case/recursive
traversal with backup accounting, bad-input errors, injected read-failure
accounting, nearby/same-file/unmatched rows, ambiguity preservation, nearest
cap with overflow counts, repeat strings, overlap exclusion, NID-shaped
exclusion, catalog known-mapping, CSV schema/escaping/round-trip/sort order,
end-to-end both-CSV run on synthetic trees. No proprietary binaries, no
machine-specific paths, no ignored tests.

Workspace: `cargo fmt --check --all` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean,
`cargo test --workspace` all green (51 `ps5-cli` tests incl. the 26 new ones).

## 9. Known limitations

- Strict matcher rejects digit-suffixed symbols (`#3#1`); ELF parsing covers them.
- Dense modules yield large CSVs even with the cap; raise
  `--min-string-length` or lower `--nearby-bytes`/`--max-candidates-per-nid`
  to shrink output.
- String inventory is streamed to disk when `--strings-output` is set, but
  correlation rows accumulate in memory (bounded by NIDs × cap + extras).
- Symlink cycles are bounded by a 32-level walk-depth guard, reported as a
  traversal note if hit.
- Adjacent duplicate NID-shaped strings in one file collapse to first offset.

## 10. Disclaimer

**ASCII correlation does not prove an NID-to-symbol mapping.** Nearby,
same-file, and categorical co-occurrence are investigative leads with `low`
(or no) confidence. Verified mappings come only from `known_mapping` catalog
rows, and ultimately from ELF import-table parsing (`ps5rs symbols`,
`ps5rs imports`, `ps5rs unknown-nids`) and independent export databases.
