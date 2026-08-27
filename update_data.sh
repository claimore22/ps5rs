#!/bin/sh
set -e
ROMS="C:\Users\claimoar\Documents\ROMS\PS5"
OUT="analysis"

echo "[1/4] Scanning $ROMS -> $OUT (ps5-prx + ps5-image + ps5-schema + ps5-nid-db)"
cargo run -p ps5-cli -- scan --output "$OUT" --include-modules "$ROMS"

echo "[2/4] Validating dataset + middleware (ps5-signatures, ps5-deps, ps5-sdk-meta, ps5-firmware, ps5-shader)"
cargo run -p ps5-cli -- validate dataset "$OUT" || true
cargo run -p ps5-cli -- middleware "$ROMS" --output "$OUT/reports/middleware.json" || echo "middleware optional"

echo "[3/4] Deps / graph"
cargo run -p ps5-cli -- deps --format json --output "$OUT/reports/deps.json" "$ROMS"
mkdir -p "$OUT/reports"
cargo run -p ps5-cli -- analyze graph "$OUT" --format dot > "$OUT/reports/graph.dot"

echo "[4/4] Dashboard (ps5-dashboard consumes all above)"
cargo run -p ps5-cli -- dashboard "$OUT" --output "$OUT/dashboard/index.html"
# Multi-page variant:
# cargo run -p ps5-cli -- dashboard "$OUT" --output "$OUT/dashboard" --games "$ROMS"

echo "Done. Dashboard: $OUT/dashboard/index.html"
echo "Multi-page: $OUT/dashboard/ (index/shader/firmware/deps.html) via output dir"
