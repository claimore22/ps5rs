# Ingest farm — serial JDownloader → ps5rs pipeline

One game at a time: download → size gate → extract → cumulative rescan of
`analysis_with_modules` → loader → reports → dashboard → validate → commit →
delete → next URL.

## Files

| File | Role |
|---|---|
| `feeder.py` | crawler JSON → `.crawljob` packages (Game rows only, title-ID deduped) |
| `ingest-one.ps1` | per-package gate script (exit 0 OK / 2 SKIP / 1 ABORT) |
| `eventscripter-package-finished.js` | JD trigger → calls the ps1 |
| `queue.log` | append-only audit trail (created on first run) |

## Setup (once)

1. `cargo build --release` (fresh `target\release\ps5rs.exe`).
2. JDownloader: max simultaneous downloads = 1; Archive Extractor OFF;
   download target = your INBOX folder.
3. Event Scripter: new script, trigger **Package finished**, paste the `.js`
   file, fix the `ps1` path if the repo moves.
4. Test feeder dry run:
   `python feeder.py --json <crawler.json> --manifest ..\..\analysis_with_modules\manifest.json --out out --only PPSA22520`
   → one `.crawljob` (Animal Well) + `queue.tsv`. Drop the job file into JD
   (or double-click) to queue the supervised test download.
5. Farm root defaults to `<repo-parent>\ps5farm` (`work\`, `failed\` inside);
   override with `-FarmRoot` for another drive.

## Behavior notes

- Size gate is purely dynamic: skip iff `unpacked + 1 GB > free − 20 GB`.
  Skips delete the archive, log `SKIP_OVERSIZE` with the URL, queue continues.
- Corrupt archives / missing eboot.bin → moved to `failed\`, logged, queue continues.
- Systemic failures (ps5rs error, dirty git tree, duplicate game) → ABORT:
  JDownloader is stopped, files kept, fix manually and resume.
- `data\` NID promotion stays manual per game (review `unknown-nids.json`
  first); the script commits `analysis_with_modules\` only.
- Backups: `backup/pre-ingest` (pristine) and `backup/pre-ingest-with-worktree`.
