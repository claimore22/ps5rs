#Requires -Version 5.1
<#
.SYNOPSIS
  ps5rs - Full Refresh Pipeline (PowerShell)

.DESCRIPTION
  Rebuilds the binary, syncs NID catalog, rescans games, and regenerates
  all analysis artifacts (dataset, load reports, dashboard).

.PARAMETER GamesDir
  Directory containing game subfolders with eboot.bin (default: games)

.PARAMETER OutputDir
  Dataset output directory (default: analysis)

.PARAMETER NoBuild
  Skip cargo build --release

.PARAMETER Clean
  Delete OutputDir before scanning

.EXAMPLE
  .\refresh.ps1
  .\refresh.ps1 -GamesDir "D:\PS5\Games" -OutputDir analysis -Clean
  .\refresh.ps1 -NoBuild
#>
[CmdletBinding()]
param(
  [string]$GamesDir = "games",
  [string]$OutputDir = "analysis",
  [switch]$NoBuild,
  [switch]$Clean,
  [string]$SystemModules = "system_modules",
  [switch]$Help,
  [Parameter(ValueFromRemainingArguments=$true)]
  [string[]]$Remaining
)

if ($Help -or $Remaining -contains "--help" -or $Remaining -contains "-h" -or $GamesDir -in @("--help","-h","-Help","help","/?")) {
  Write-Host "Usage: refresh.ps1 [-GamesDir <path>] [-OutputDir <path>] [-NoBuild] [-Clean] [-Help]"
  Write-Host ""
  Write-Host "  -GamesDir   Directory containing game subfolders with eboot.bin (default: games)"
  Write-Host "  -OutputDir  Dataset output directory (default: analysis)"
  Write-Host ""
  Write-Host "  -NoBuild    Skip cargo build --release"
  Write-Host "  -Clean      Delete OutputDir before scanning"
  Write-Host "  -Help/-h    Show this help"
  Write-Host ""
  Write-Host "Examples:"
  Write-Host '  .\refresh.ps1'
  Write-Host '  .\refresh.ps1 -GamesDir "D:\PS5\Games" -OutputDir analysis -Clean'
  Write-Host '  .\refresh.ps1 -NoBuild'
  Write-Host '  .\refresh.ps1 -Help'
  Write-Host '  .\refresh.ps1 --help   # also works'
  exit 0
}
if ($Remaining -and $Remaining[0] -and $GamesDir -eq "games" -and $Remaining[0] -notlike "-*") {
  $GamesDir = $Remaining[0]
  if ($Remaining.Count -gt 1 -and $Remaining[1] -notlike "-*") { $OutputDir = $Remaining[1] }
}

Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"

function Write-Step { param([string]$Msg) Write-Host $Msg -ForegroundColor Cyan }
function Write-Warn { param([string]$Msg) Write-Host "[WARN] $Msg" -ForegroundColor Yellow }
function Write-Err  { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

# --- locate ps5rs binary (prefer release, fallback to cargo run) ---
$PS5RS_EXE = "target\release\ps5rs.exe"
$PS5RS_UNIX = "target/release/ps5rs"
function Invoke-PS5RS {
  param([string[]]$Args)
  if (Test-Path $PS5RS_EXE) {
    & $PS5RS_EXE @Args
    return $LASTEXITCODE
  } elseif (Test-Path $PS5RS_UNIX) {
    & $PS5RS_UNIX @Args
    return $LASTEXITCODE
  } else {
    $cargoArgs = @("run","--release","--bin","ps5rs","--") + $Args
    & cargo @cargoArgs
    return $LASTEXITCODE
  }
}
function Test-PS5RSExists {
  return (Test-Path $PS5RS_EXE) -or (Test-Path $PS5RS_UNIX)
}

$resolvedGames = $GamesDir
$resolvedOutput = $OutputDir
$ps5rsLabel = if (Test-Path $PS5RS_EXE) { $PS5RS_EXE } elseif (Test-Path $PS5RS_UNIX) { $PS5RS_UNIX } else { "cargo run --release --bin ps5rs --" }

Write-Host "============================================================" -ForegroundColor White
Write-Host " ps5rs refresh - $(Get-Date -Format o)" -ForegroundColor White
Write-Host " Games : $resolvedGames" -ForegroundColor White
Write-Host " Output: $resolvedOutput" -ForegroundColor White
Write-Host " Binary: $ps5rsLabel" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor White
Write-Host ""

# --- preflight ---
if (-not (Test-Path $resolvedGames)) {
  Write-Err "Games directory not found: $resolvedGames"
  Write-Host '        Pass it explicitly: .\refresh.ps1 -GamesDir "D:\PS5\Games" -OutputDir analysis' -ForegroundColor Gray
  exit 1
}

if ($Clean) {
  Write-Step "[CLEAN] Removing $resolvedOutput ..."
  if (Test-Path $resolvedOutput) { Remove-Item -Recurse -Force $resolvedOutput }
  Write-Host ""
}

# ===================================================================
# 1. Build
# ===================================================================
if (-not $NoBuild) {
  Write-Step "[1/8] Building ps5rs (release) ..."
  cargo build --release
  if ($LASTEXITCODE -ne 0) { Write-Err "cargo build failed"; exit 1 }
  Write-Host "       -> $PS5RS_EXE" -ForegroundColor Gray
  Write-Host ""
} else {
  Write-Step "[1/8] Skipping build (-NoBuild)"
  Write-Host ""
}

# ===================================================================
# 2. Catalog - local nids.csv only (Supabase sync disabled per user request)
# ===================================================================
Write-Step "[2/8] Using local NID catalog - Supabase sync skipped"
Write-Host "       Catalog: data/nids.csv (165k+ entries, 6-col consolidated, built-in)" -ForegroundColor Gray
Write-Host "       No network download - HV4j+E0MBHE etc. resolved from local file" -ForegroundColor Gray
Write-Host ""

# ===================================================================
# 3. Scan - eboot.bin -> BinaryImage dataset
#    Re-bakes resolved_name with the fixed 6-col parser (HV4j+E0MBHE etc.)
# ===================================================================
Write-Step "[3/8] Scanning games: $resolvedGames -> $resolvedOutput"
Write-Host "       This re-resolves NIDs with the patched ps5-nid parser." -ForegroundColor Gray
$code = Invoke-PS5RS -Args @("scan", $resolvedGames, "--output", $resolvedOutput)
if ($code -ne 0) { Write-Err "scan failed"; exit 1 }
Write-Host ""

# --- optional: scan with PRX modules ---
Write-Step "[3b/8] Scanning with PRX modules (sce_module/*.prx) ..."
$code = Invoke-PS5RS -Args @("scan", $resolvedGames, "--output", "${resolvedOutput}_with_modules", "--include-modules")
if ($code -ne 0) { Write-Warn "scan --include-modules failed" }
Write-Host ""

# ===================================================================
# 4. Export-scan - build offline export DB for loader
# ===================================================================
Write-Step "[4/8] Export-scan: building $SystemModules from PRXes ..."
$code = Invoke-PS5RS -Args @("export-scan", $resolvedGames, "--output", $SystemModules)
if ($code -ne 0) { Write-Warn "export-scan failed" }
Write-Host ""

# ===================================================================
# 5. Batch-load - virtual loader across all games
# ===================================================================
Write-Step "[5/8] Batch-load: running loader pipeline on all games ..."
$code = Invoke-PS5RS -Args @("batch-load", $resolvedGames, "--output", "$resolvedOutput\load")
if ($code -ne 0) { Write-Warn "batch-load failed" }
Write-Host ""

# ===================================================================
# 6. Analysis reports - stats, unknown NIDs (now catalog-aware)
# ===================================================================
Write-Step "[6/8] Generating analysis reports ..."
Write-Host "       - stats" -ForegroundColor Gray
Invoke-PS5RS -Args @("analyze","stats",$resolvedOutput,"--format","terminal") | Out-Host
Write-Host ""
Write-Host "       - unknown NIDs (re-resolved with live catalog - HV4j should be 0)" -ForegroundColor Gray
Invoke-PS5RS -Args @("analyze","unknown",$resolvedOutput,"--format","terminal") | Out-Host
Write-Host ""
Write-Host "       - engines" -ForegroundColor Gray
Invoke-PS5RS -Args @("analyze","engines",$resolvedOutput,"--format","terminal") | Out-Host
Write-Host ""
Write-Host "       - imports" -ForegroundColor Gray
Invoke-PS5RS -Args @("analyze","imports",$resolvedOutput,"--format","terminal") | Out-Host
Write-Host ""

# --- also export machine-readable unknown reports ---
Write-Host "       Exporting unknown NIDs to $resolvedOutput\unknown_nids.json ..." -ForegroundColor Gray
Invoke-PS5RS -Args @("analyze","unknown",$resolvedOutput,"--format","json","-o","$resolvedOutput\unknown_nids.json") | Out-Null
Invoke-PS5RS -Args @("analyze","unknown",$resolvedOutput,"--format","csv","-o","$resolvedOutput\unknown_nids.csv") | Out-Null
Invoke-PS5RS -Args @("export-unknown",$resolvedOutput,"-o","$resolvedOutput\export_unknown_by_freq.csv") | Out-Null
Invoke-PS5RS -Args @("export-unknown",$resolvedOutput,"--group-by","library","-o","$resolvedOutput\export_unknown_by_lib.csv") | Out-Null
Write-Host ""

# ===================================================================
# 7. Dashboard - static HTML (self-contained, no CDN)
# ===================================================================
Write-Step "[7/8] Generating dashboard ..."
$code = Invoke-PS5RS -Args @("dashboard",$resolvedOutput,"--output","$resolvedOutput\dashboard","--games",$resolvedGames)
if ($code -ne 0) { Write-Warn "dashboard generation failed" }
Write-Host "       -> $resolvedOutput\dashboard\index.html" -ForegroundColor Gray
Write-Host ""

# ===================================================================
# 8. Tests - verify patched parser + stale-dataset fix
# ===================================================================
Write-Step "[8/8] Running regression tests ..."
cargo test -p ps5-nid --lib
if ($LASTEXITCODE -ne 0) { Write-Warn "ps5-nid tests failed" }
cargo test -p ps5-analysis --lib reports::unknown_nids
if ($LASTEXITCODE -ne 0) { Write-Warn "ps5-analysis tests failed" }
Write-Host ""

Write-Host "============================================================" -ForegroundColor White
Write-Host " Refresh complete - $(Get-Date -Format o)" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor White
Write-Host " Outputs:" -ForegroundColor White
Write-Host "   Dataset ........ $resolvedOutput\images\ (BinaryImage JSON)" -ForegroundColor Gray
Write-Host "   With modules ... ${resolvedOutput}_with_modules\" -ForegroundColor Gray
Write-Host "   Load reports ... $resolvedOutput\load\" -ForegroundColor Gray
Write-Host "   System modules . $SystemModules\" -ForegroundColor Gray
Write-Host "   Dashboard ...... $resolvedOutput\dashboard\index.html" -ForegroundColor Gray
Write-Host "   Unknown JSON ... $resolvedOutput\unknown_nids.json" -ForegroundColor Gray
Write-Host "   Unknown CSV .... $resolvedOutput\unknown_nids.csv" -ForegroundColor Gray
Write-Host ""
Write-Host " Next:" -ForegroundColor White
Write-Host "   ps5rs analyze heatmap $resolvedOutput" -ForegroundColor Gray
Write-Host "   ps5rs analyze graph $resolvedOutput --format dot -o graph.dot" -ForegroundColor Gray
Write-Host "   ps5rs middleware `"$resolvedGames`" --format json -o middleware.json" -ForegroundColor Gray
Write-Host "   ps5rs dashboard $resolvedOutput --games `"$resolvedGames`"" -ForegroundColor Gray
Write-Host "============================================================" -ForegroundColor White
