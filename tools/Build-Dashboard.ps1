[CmdletBinding()]
param(
    [string[]]$GamesDir = @("PUT YOUR GAMES FOLDER HERE"),
    [string]$OutputDir = "C:\Users\claimoar\Documents\Rust\ps5rs\analysis",
    [switch]$SkipBuild,
    [switch]$SkipMiddleware,
    [switch]$Append
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$datasetDir = Join-Path $OutputDir "dataset"
$dashboardDir = Join-Path $OutputDir "dashboard"
$ps5rs = Join-Path $repo "target\release\ps5rs.exe"

foreach ($dir in $GamesDir) {
    if (-not (Test-Path -LiteralPath $dir)) {
        throw "Games folder does not exist: $dir"
    }
}

if (-not $SkipBuild) {
    Write-Host "[1/3] Building ps5rs (release)..." -ForegroundColor Cyan
    Push-Location $repo
    try {
        cargo build --release -p ps5-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $ps5rs)) {
    throw "ps5rs binary not found: $ps5rs (run without -SkipBuild once)"
}

Write-Host "[2/3] Scanning $($GamesDir.Count) games folder(s) -> $datasetDir..." -ForegroundColor Cyan
$first = $true
foreach ($dir in $GamesDir) {
    $scanArgs = @("scan", $dir, "-o", $datasetDir, "--include-modules")
    if ($Append -or -not $first) { $scanArgs += "--append" }
    & $ps5rs @scanArgs
    if ($LASTEXITCODE -ne 0) { throw "scan failed for $dir (exit $LASTEXITCODE)" }
    $first = $false
}

Write-Host "[3/3] Building dashboard -> $dashboardDir..." -ForegroundColor Cyan
$dashArgs = @("dashboard", $datasetDir, "-o", $dashboardDir)
if (-not $SkipMiddleware) {
    foreach ($dir in $GamesDir) { $dashArgs += @("--games", $dir) }
}
& $ps5rs @dashArgs
if ($LASTEXITCODE -ne 0) { throw "dashboard failed (exit $LASTEXITCODE)" }

Write-Host "Done." -ForegroundColor Green
Write-Host "  Dataset:   $datasetDir"
Write-Host "  Dashboard: $(Join-Path $dashboardDir 'index.html')"
