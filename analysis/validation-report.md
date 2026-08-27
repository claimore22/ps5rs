# Validation Report — 2026-08-27

- **Branch:** `update_plan_v2`
- **Dataset:** `analysis` (27 games, from `C:\Users\claimoar\Documents\ROMS\PS5`)
- **Tool:** ps5rs 0.4.0

## Summary

- **Total games:** 27
- **Valid ELF:** 27 / 27
- **NID resolution:** 98.6% avg (98.58% raw)
- **Parse errors:** 0
- **Lib versions found:** 0 (PS5 ELF libversion segment present but not yet decoded for these titles, or not present)

## Dashboard

- Generated via `cargo run -p ps5-cli -- dashboard analysis --output analysis/dashboard_honesty_test --games "C:\Users\claimoar\Documents\ROMS\PS5"`
- **Pages:** index.html, shader.html, firmware.html, deps.html (self-contained, file://)
- **Honest states verified:**
  - Shader tab: "Not yet analyzed — shader analysis not wired to real data." (total_shaders=0, by_stage empty, not total*2)
  - Firmware tab: "Not yet analyzed — firmware catalog not wired to dataset." (total_modules=0)
  - SDK Timeline: marked "Reference — not detected from current dataset"
  - SCE module_count: shows "Not available in current dataset" when 0
  - Middleware: 27 games, 316 modules (181 third-party, 126 Sony, 9 unknown) — injected via --games, not empty
  - Loader: per-game import resolution, unavailable modules, worst games

## Firmware Exports

- System modules: 8 export tables in `system_modules/*.exports.json` (libc, libSceFace, etc.)
- FirmwareCatalog::load_exports_from_dir now tested against real files (lib count, export count)
- NidDatabase::populate_from_exports_dir now imports firmware NIDs with Firmware source + Verified confidence

## Shader

- 649 shader files found (Content/Shaders .sb)
- Real header parsing: stage from Shdr byte 44 (1=vertex,2=pixel,3=compute), hash via SHA256, no DefaultHasher, no hexdump disassembly label, no synthetic default_sampler

## Tests

- `cargo fmt --check` — pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass
- `cargo test --workspace` — 540+ tests pass (including updated shader, firmware, abi, nid-db, dashboard)
