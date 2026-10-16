#!/usr/bin/env python3
"""Scan an FMOD SDK tree for lines containing " F_API " and hash the symbol.

Every matching line becomes one CSV row (no dedup): the NID is computed
from the word right after " F_API " with the PS5 algorithm
(SHA-1 over name bytes + 16-byte salt, first 8 digest bytes reversed,
custom base64 alphabet A-Za-z0-9+-, 11 chars).

Usage:
    python tools/fmod_fapi_scan.py [sdk_dir] [-o out.csv]
"""

import csv
import hashlib
import os
import sys

SALT = bytes([
    0x51, 0x8D, 0x64, 0xA6, 0x35, 0xDE, 0xD8, 0xC1,
    0xE6, 0xB0, 0x39, 0xB1, 0xC3, 0xE5, 0x52, 0x30,
])
B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-"

DEFAULT_SDK = r"C:\Program Files (x86)\FMOD SoundSystem\FMOD Studio API Window_202"


def nid(name):
    digest = hashlib.sha1(name.encode("utf-8") + SALT).digest()
    rev = digest[7::-1]
    chars = []
    for i in (0, 3):
        triple = (rev[i] << 16) | (rev[i + 1] << 8) | rev[i + 2]
        chars += [
            B64[(triple >> 18) & 63],
            B64[(triple >> 12) & 63],
            B64[(triple >> 6) & 63],
            B64[triple & 63],
        ]
    tail = (rev[6] << 16) | (rev[7] << 8)
    chars += [
        B64[(tail >> 18) & 63],
        B64[(tail >> 12) & 63],
        B64[(tail >> 6) & 63],
    ]
    return "".join(chars)


def word_after_fapi(line):
    _, _, tail = line.partition(" F_API ")
    word = tail.strip().split()[0] if tail.strip() else ""
    return word.split("(")[0]


def main(argv):
    assert nid("sceKernelSleep") == "-ZR+hG7aDHw", "algorithm self-check failed"
    assert nid("memcpy") == "Q3VBxCXhUHs", "algorithm self-check failed"

    sdk = DEFAULT_SDK
    out_path = "fmod-fapi-scan.csv"
    args = list(argv)
    while args:
        arg = args.pop(0)
        if arg == "-o" and args:
            out_path = args.pop(0)
        elif not arg.startswith("-"):
            sdk = arg
        else:
            sys.exit(f"unknown option: {arg}")

    if not os.path.isdir(sdk):
        sys.exit(f"not a directory: {sdk}")

    rows = 0
    skipped = 0
    with open(out_path, "w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["nid", "symbol", "file", "line_number", "line"])
        for root, _dirs, files in os.walk(sdk):
            for name in sorted(files):
                path = os.path.join(root, name)
                try:
                    with open(path, encoding="utf-8") as handle:
                        text = handle.read()
                except (UnicodeDecodeError, OSError) as exc:
                    print(f"skip {path}: {exc}", file=sys.stderr)
                    skipped += 1
                    continue
                rel = os.path.relpath(path, sdk)
                for number, line in enumerate(text.splitlines(), 1):
                    if " F_API " not in line:
                        continue
                    symbol = word_after_fapi(line)
                    writer.writerow([
                        nid(symbol) if symbol else "",
                        symbol,
                        rel,
                        number,
                        line.strip(),
                    ])
                    rows += 1

    print(f"wrote {rows} rows to {out_path} ({skipped} files skipped)")


if __name__ == "__main__":
    main(sys.argv[1:])
