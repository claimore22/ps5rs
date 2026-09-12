#!/usr/bin/env python3
"""Feeder: crawler JSON -> JDownloader .crawljob packages (one URL per game).

Reads the SUPER_PSX_CRAWLER ps5-games-with-links.json, keeps only base-game
rows (col0 starting with 'Game'), normalizes host names, picks one preferred
URL per game, and writes a .crawljob file per game plus a queue.tsv manifest.

Usage:
    python feeder.py --json <crawler.json> --manifest <analysis_with_modules/manifest.json>
                     --out <crawljob-dir> [--only PPSA22520] [--limit 50]

A .crawljob file (JSON array of flat job objects, per the official JD
folder-watch docs) dropped into JDownloader's folderwatch directory queues
the package. JD2 watches <install-dir>\folderwatch by default.
"""
import argparse
import json
import re
import sys
from pathlib import Path

TYPO_HOSTS = {
    "multihosts": "multihost", "mutlihost": "multihost",
    "mulithost": "multihost", "mutihost": "multihost",
    "multihost": "multihost", "gofile": "gofile",
}
JUNK_HOSTS = {"", "read before downloading", "guide here", "here",
              "here ", "siespta team", "i", "le", "data"}
PREFERRED = ["viki", "rootz", "onefile", "multihost", "gofile", "filek", "akr"]


def canon_host(raw):
    h = (raw or "").strip().lower()
    h = re.sub(r"\(.*\)", "", h).strip()
    h = TYPO_HOSTS.get(h, h)
    return None if h in JUNK_HOSTS or len(h) <= 2 else h


def game_rows(entry):
    for row in entry.get("table_data") or []:
        for cell in row:
            if not isinstance(cell, dict):
                continue
            c0, c1 = cell.get("col0"), cell.get("col1")
            if isinstance(c0, str) and c0.startswith("Game") and isinstance(c1, dict):
                yield c0.strip(), c1.get("links") or []


def version_of(entry):
    for row in entry.get("table_data") or []:
        for cell in row:
            if not isinstance(cell, dict):
                continue
            c0 = cell.get("col0")
            if isinstance(c0, str) and c0.startswith("Version"):
                m = re.search(r"PPSA\d+", str(cell.get("col1")))
                return m.group(0) if m else ""
    return ""


def password_of(entry, default="SuperPSX"):
    for row in entry.get("table_data") or []:
        for cell in row:
            if not isinstance(cell, dict):
                continue
            c0 = cell.get("col0")
            if isinstance(c0, str) and c0.startswith("Password"):
                c1 = cell.get("col1")
                return c1.strip() if isinstance(c1, str) and c1.strip() else default
    return default


def pick_url(entry):
    seen = {}
    for _row, links in game_rows(entry):
        for link in links:
            host = canon_host(link.get("text"))
            url = (link.get("url") or "").strip()
            if host and url and host not in seen:
                seen[host] = url
    if not seen:
        return None, None, {}
    order = PREFERRED + sorted(set(seen) - set(PREFERRED))
    chosen = next(h for h in order if h in seen)
    return chosen, seen[chosen], {h: u for h, u in seen.items() if h != chosen}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", required=True)
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", default="")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    games = json.load(open(args.json, encoding="utf-8"))
    have = set()
    try:
        md = json.load(open(args.manifest, encoding="utf-8"))
        have = {g.get("title_id", "") for g in md.get("games", [])}
    except FileNotFoundError:
        print("note: manifest not found, no dedupe", file=sys.stderr)

    out = Path(args.out)
    (out / "jobs").mkdir(parents=True, exist_ok=True)
    queued, skipped_have, skipped_nolink = 0, 0, 0
    with open(out / "queue.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("ppsa\tname\tchosen_host\tchosen_url\talternates\tpassword\n")
        for entry in games:
            ppsa = version_of(entry)
            if args.only and ppsa != args.only:
                continue
            if ppsa in have:
                skipped_have += 1
                continue
            host, url, alternates = pick_url(entry)
            if not url:
                skipped_nolink += 1
                print(f"skip (no game-row links): {entry.get('name')}", file=sys.stderr)
                continue
            pw = password_of(entry)
            pkg = f"{entry.get('name', 'game')} [{ppsa}]" if ppsa else entry.get("name", "game")
            job = [{
                "packageName": pkg,
                "text": url,
                "downloadPassword": pw,
                "extractPasswords": [pw],
                "comment": f"crawler {ppsa}; alternates: " +
                           ", ".join(f"{h}={u}" for h, u in alternates.items()),
                "enabled": "TRUE",
                "autoStart": "TRUE",
                "autoConfirm": "TRUE",
                "extractAfterDownload": "FALSE",
            }]
            safe = re.sub(r"[^\w\-\. ]", "_", pkg).strip()[:120]
            (out / "jobs" / f"{safe}.crawljob").write_text(
                json.dumps(job, indent=2), encoding="utf-8")
            alt = ";".join(f"{h}={u}" for h, u in alternates.items())
            tsv.write(f"{ppsa}\t{entry.get('name')}\t{host}\t{url}\t{alt}\t{pw}\n")
            queued += 1
            if args.limit and queued >= args.limit:
                break
    print(f"queued={queued} already_have={skipped_have} no_links={skipped_nolink}")


if __name__ == "__main__":
    main()
