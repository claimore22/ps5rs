#!/usr/bin/env python3
"""Hash every printable ASCII string under an SDK tree, report NID matches.

Usage:
    python tools/fmod_strings_match.py [api_dir] [nid ...] [--min-length N]

Defaults to the FMOD Studio API tree and the four NIDs under test.
Every occurrence is reported (no dedup). Only exact full-string matches
can hit: a symbol embedded in a longer run hashes differently.
"""

import hashlib
import os
import sys

SALT = bytes([
    0x51, 0x8D, 0x64, 0xA6, 0x35, 0xDE, 0xD8, 0xC1,
    0xE6, 0xB0, 0x39, 0xB1, 0xC3, 0xE5, 0x52, 0x30,
])
B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-"

DEFAULT_API = r"C:\Program Files (x86)\FMOD SoundSystem\FMOD Studio API Window_202\api"
DEFAULT_TARGETS = ["0+HgFkajHnk", "2Rb+xdt8XkU", "5QDnyU+SPxs", "H+G1XQRDyWk"]


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


def ascii_strings(data, min_length):
    results = []
    start = None
    for i, byte in enumerate(data):
        if 0x20 <= byte <= 0x7E:
            if start is None:
                start = i
        elif start is not None:
            if i - start >= min_length:
                results.append((start, data[start:i].decode("ascii")))
            start = None
    if start is not None and len(data) - start >= min_length:
        results.append((start, data[start:].decode("ascii")))
    return results


def token_variants(text):
    seen = set()
    for token in text.split():
        if token not in seen:
            seen.add(token)
            yield token, "token"
        stripped = strip_edges(token)
        if stripped and stripped not in seen:
            seen.add(stripped)
            yield stripped, "token-stripped"


def strip_edges(token):
    chars = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_:#")
    start = 0
    end = len(token)
    while start < end and token[start] not in chars:
        start += 1
    while end > start and token[end - 1] not in chars:
        end -= 1
    return token[start:end]


def main(argv):
    assert nid("sceKernelSleep") == "-ZR+hG7aDHw", "algorithm self-check failed"
    assert nid("memcpy") == "Q3VBxCXhUHs", "algorithm self-check failed"

    root = DEFAULT_API
    targets = list(DEFAULT_TARGETS)
    min_length = 4
    args = list(argv)
    positional = []
    while args:
        arg = args.pop(0)
        if arg == "--":
            positional.extend(args)
            break
        elif arg == "--min-length" and args:
            min_length = int(args.pop(0))
        elif arg.startswith("-"):
            sys.exit(f"unknown option: {arg}")
        else:
            positional.append(arg)
    if positional:
        root = positional[0]
        if len(positional) > 1:
            targets = positional[1:]
    wanted = set(targets)

    if not os.path.isdir(root):
        sys.exit(f"not a directory: {root}")

    files = 0
    strings = 0
    hits = 0
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in sorted(filenames):
            path = os.path.join(dirpath, name)
            try:
                with open(path, "rb") as handle:
                    data = handle.read()
            except OSError as exc:
                print(f"skip {path}: {exc}", file=sys.stderr)
                continue
            files += 1
            rel = os.path.relpath(path, root)
            for offset, text in ascii_strings(data, min_length):
                strings += 1
                candidates = [(text, "full")]
                candidates += list(token_variants(text))
                for candidate, kind in candidates:
                    digest = nid(candidate)
                    if digest in wanted:
                        print(f"{digest}  {candidate}  [{kind}] [{rel} @ 0x{offset:x}]")
                        hits += 1

    print(f"scanned {files} files, {strings} strings, {hits} matches", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1:])
