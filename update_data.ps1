$ROMS="C:\Users\claimoar\Documents\ROMS\PS5"
$OUT="analysis"

Write-Host "[1/4] Scanning $ROMS -> $OUT (ps5-prx + ps5-image + ps5-schema + ps5-nid-db)" -ForegroundColor Cyan
cargo run -p ps5-cli -- scan --output $OUT --include-modules "$ROMS"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/4] Validating dataset + middleware (ps5-signatures, ps5-deps, ps5-sdk-meta, ps5-firmware, ps5-shader)" -ForegroundColor Cyan
cargo run -p ps5-cli -- validate dataset "$OUT"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo run -p ps5-cli -- middleware "$ROMS" --output "$OUT\reports\middleware.json"
if ($LASTEXITCODE -ne 0) { Write-Host "middleware scan optional" -ForegroundColor Yellow }

Write-Host "[3/4] Deps / graph" -ForegroundColor Cyan
cargo run -p ps5-cli -- deps --format json --output "$OUT\reports\deps.json" "$ROMS"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force -Path "$OUT\reports" | Out-Null
cargo run -p ps5-cli -- analyze graph "$OUT" --format dot | Out-File -Encoding utf8 "$OUT\reports\graph.dot"

Write-Host "[4/4] Dashboard (ps5-dashboard consumes all above)" -ForegroundColor Cyan
cargo run -p ps5-cli -- dashboard "$OUT" --output "$OUT\dashboard\index.html"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Multi-page variant (shader/firmware/deps tabs, still file://, no server):
# cargo run -p ps5-cli -- dashboard "$OUT" --output "$OUT\dashboard" --games "$ROMS"

Write-Host "Done. Dashboard: $OUT\dashboard\index.html" -ForegroundColor Green
Write-Host "Multi-page: $OUT\dashboard\ (index/shader/firmware/deps.html) via output dir" -ForegroundColor Green
