#!/usr/bin/env bash
# ============================================================================
# ps5rs - Full Refresh Pipeline (bash / sh)
# Rebuilds the binary, syncs NID catalog, rescans games, and regenerates
# all analysis artifacts (dataset, load reports, dashboard).
#
# Usage:
#   ./refresh.sh                         # defaults: ./games -> ./analysis
#   ./refresh.sh /mnt/games ./analysis   # custom paths
#   ./refresh.sh --no-build              # skip cargo build
#   ./refresh.sh --clean                 # wipe analysis/ first
#   ./refresh.sh --help
# ============================================================================
set -euo pipefail

GAMES_DEFAULT="games"
OUTPUT_DEFAULT="analysis"
SYSTEM_MODULES="system_modules"
SKIP_BUILD=0
CLEAN=0
GAMES_DIR=""
OUTPUT_DIR=""

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build) SKIP_BUILD=1; shift ;;
    --clean)    CLEAN=1; shift ;;
    --help|-h)
      echo "Usage: refresh.sh [GAMES_DIR] [OUTPUT_DIR] [OPTIONS]"
      echo ""
      echo "  GAMES_DIR   Directory containing game subfolders with eboot.bin (default: games)"
      echo "  OUTPUT_DIR  Dataset output directory (default: analysis)"
      echo ""
      echo "Options:"
      echo "  --clean     Delete OUTPUT_DIR before scanning"
      echo "  --no-build  Skip cargo build --release"
      echo "  --help      Show this help"
      echo ""
      echo "Examples:"
      echo "  ./refresh.sh"
      echo "  ./refresh.sh /mnt/ps5/games analysis --clean"
      echo "  ./refresh.sh --no-build"
      exit 0
      ;;
    --*) echo "[ERROR] Unknown option: $1" >&2; exit 1 ;;
    *)
      if [[ -z "$GAMES_DIR" ]]; then GAMES_DIR="$1"
      elif [[ -z "$OUTPUT_DIR" ]]; then OUTPUT_DIR="$1"
      else echo "[ERROR] Too many positional args: $1" >&2; exit 1
      fi
      shift
      ;;
  esac
done

GAMES_DIR="${GAMES_DIR:-$GAMES_DEFAULT}"
OUTPUT_DIR="${OUTPUT_DIR:-$OUTPUT_DEFAULT}"

# --- locate ps5rs binary (prefer release, fallback to cargo run) ---
PS5RS="target/release/ps5rs"
if [[ ! -x "$PS5RS" ]]; then
  # Windows exe fallback when running under Git Bash / MSYS
  if [[ -x "target/release/ps5rs.exe" ]]; then
    PS5RS="target/release/ps5rs.exe"
  else
    PS5RS="cargo run --release --bin ps5rs --"
  fi
fi
# helper: run ps5rs either as binary or via cargo
run_ps5rs() {
  if [[ -x "target/release/ps5rs" ]]; then
    target/release/ps5rs "$@"
  elif [[ -x "target/release/ps5rs.exe" ]]; then
    target/release/ps5rs.exe "$@"
  else
    cargo run --release --bin ps5rs -- "$@"
  fi
}

echo "============================================================"
echo " ps5rs refresh - $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo " Games : $GAMES_DIR"
echo " Output: $OUTPUT_DIR"
echo " Binary: $PS5RS"
echo "============================================================"
echo ""

# --- preflight ---
if [[ ! -d "$GAMES_DIR" ]]; then
  echo "[ERROR] Games directory not found: $GAMES_DIR" >&2
  echo "        Pass it explicitly: ./refresh.sh \"/mnt/ps5/games\" analysis" >&2
  exit 1
fi

if [[ "$CLEAN" -eq 1 ]]; then
  echo "[CLEAN] Removing $OUTPUT_DIR ..."
  rm -rf -- "$OUTPUT_DIR"
  echo ""
fi

# ===================================================================
# 1. Build
# ===================================================================
if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "[1/8] Building ps5rs (release) ..."
  cargo build --release
  echo "       -> target/release/ps5rs"
  echo ""
else
  echo "[1/8] Skipping build (--no-build)"
  echo ""
fi

# ===================================================================
# 2. Catalog - local nids.csv only (Supabase sync disabled per user request)
# ===================================================================
echo "[2/8] Using local NID catalog - Supabase sync skipped"
echo "       Catalog: data/nids.csv (165k+ entries, 6-col consolidated, built-in)"
echo "       No network download - HV4j+E0MBHE etc. resolved from local file"
echo ""

# ===================================================================
# 3. Scan - eboot.bin -> BinaryImage dataset
#    Re-bakes resolved_name with the fixed 6-col parser (HV4j+E0MBHE etc.)
# ===================================================================
echo "[3/8] Scanning games: $GAMES_DIR -> $OUTPUT_DIR"
echo "       This re-resolves NIDs with the patched ps5-nid parser."
run_ps5rs scan "$GAMES_DIR" --output "$OUTPUT_DIR"
echo ""

# --- optional: scan with PRX modules ---
echo "[3b/8] Scanning with PRX modules (sce_module/*.prx) ..."
if ! run_ps5rs scan "$GAMES_DIR" --output "${OUTPUT_DIR}_with_modules" --include-modules; then
  echo "[WARN] scan --include-modules failed" >&2
fi
echo ""

# ===================================================================
# 4. Export-scan - build offline export DB for loader
# ===================================================================
echo "[4/8] Export-scan: building $SYSTEM_MODULES from PRXes ..."
if ! run_ps5rs export-scan "$GAMES_DIR" --output "$SYSTEM_MODULES"; then
  echo "[WARN] export-scan failed" >&2
fi
echo ""

# ===================================================================
# 5. Batch-load - virtual loader across all games
# ===================================================================
echo "[5/8] Batch-load: running loader pipeline on all games ..."
if ! run_ps5rs batch-load "$GAMES_DIR" --output "$OUTPUT_DIR/load"; then
  echo "[WARN] batch-load failed" >&2
fi
echo ""

# ===================================================================
# 6. Analysis reports - stats, unknown NIDs (now catalog-aware)
# ===================================================================
echo "[6/8] Generating analysis reports ..."
echo "       - stats"
run_ps5rs analyze stats "$OUTPUT_DIR" --format terminal || true
echo ""
echo "       - unknown NIDs (re-resolved with live catalog - HV4j should be 0)"
run_ps5rs analyze unknown "$OUTPUT_DIR" --format terminal || true
echo ""
echo "       - engines"
run_ps5rs analyze engines "$OUTPUT_DIR" --format terminal || true
echo ""
echo "       - imports"
run_ps5rs analyze imports "$OUTPUT_DIR" --format terminal || true
echo ""

# --- also export machine-readable unknown reports ---
echo "       Exporting unknown NIDs to $OUTPUT_DIR/unknown_nids.json ..."
run_ps5rs analyze unknown "$OUTPUT_DIR" --format json -o "$OUTPUT_DIR/unknown_nids.json" || true
run_ps5rs analyze unknown "$OUTPUT_DIR" --format csv -o "$OUTPUT_DIR/unknown_nids.csv" || true
run_ps5rs export-unknown "$OUTPUT_DIR" -o "$OUTPUT_DIR/export_unknown_by_freq.csv" || true
run_ps5rs export-unknown "$OUTPUT_DIR" --group-by library -o "$OUTPUT_DIR/export_unknown_by_lib.csv" || true
echo ""

# ===================================================================
# 7. Dashboard - static HTML (self-contained, no CDN)
# ===================================================================
echo "[7/8] Generating dashboard ..."
if ! run_ps5rs dashboard "$OUTPUT_DIR" --output "$OUTPUT_DIR/dashboard" --games "$GAMES_DIR"; then
  echo "[WARN] dashboard generation failed" >&2
fi
echo "       -> $OUTPUT_DIR/dashboard/index.html"
echo ""

# ===================================================================
# 8. Tests - verify patched parser + stale-dataset fix
# ===================================================================
echo "[8/8] Running regression tests ..."
cargo test -p ps5-nid --lib || echo "[WARN] ps5-nid tests failed" >&2
cargo test -p ps5-analysis --lib reports::unknown_nids || echo "[WARN] ps5-analysis tests failed" >&2
echo ""

echo "============================================================"
echo " Refresh complete - $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "============================================================"
echo " Outputs:"
echo "   Dataset ........ $OUTPUT_DIR/images/ (BinaryImage JSON)"
echo "   With modules ... ${OUTPUT_DIR}_with_modules/"
echo "   Load reports ... $OUTPUT_DIR/load/"
echo "   System modules . $SYSTEM_MODULES/"
echo "   Dashboard ...... $OUTPUT_DIR/dashboard/index.html"
echo "   Unknown JSON ... $OUTPUT_DIR/unknown_nids.json"
echo "   Unknown CSV .... $OUTPUT_DIR/unknown_nids.csv"
echo ""
echo " Next:"
echo "   ps5rs analyze heatmap $OUTPUT_DIR"
echo "   ps5rs analyze graph $OUTPUT_DIR --format dot -o graph.dot"
echo "   ps5rs middleware \"$GAMES_DIR\" --format json -o middleware.json"
echo "   ps5rs dashboard $OUTPUT_DIR --games \"$GAMES_DIR\""
echo "============================================================"
