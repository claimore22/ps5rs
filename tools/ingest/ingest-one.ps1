<#
  ingest-one.ps1 — serial per-game gate for the ps5rs ingest farm.
  Called once per finished JDownloader package (or manually for tests).

  Usage:
    powershell -NoProfile -ExecutionPolicy Bypass -File ingest-one.ps1 -PackageDir "<jd package folder>"

  Exit codes: 0 = OK (next URL), 2 = SKIP_OVERSIZE (next URL), 1 = ABORT (queue held).
  On ABORT the script stops the JDownloader process to hold the queue.
#>
param(
    [Parameter(Mandatory = $true)][string]$PackageDir,
    [string]$FarmRoot = (Join-Path (Split-Path "C:\Users\claimoar\Documents\Rust\ps5rs" -Parent) "ps5farm"),
    [string]$Repo = "C:\Users\claimoar\Documents\Rust\ps5rs",
    [string]$Corpus = "C:\Users\claimoar\Documents\ROMS\PS5",
    [string]$ArchivePassword = "SuperPSX",
    [long]$SafetyMarginGB = 20,
    [long]$DatasetOverheadGB = 1
)

$ErrorActionPreference = "Stop"
$PS5RS = Join-Path $Repo "target\release\ps5rs.exe"
$DS = Join-Path $Repo "analysis_with_modules"
$QueueLog = Join-Path (Split-Path $PSCommandPath -Parent) "queue.log"
$WorkRoot = Join-Path $FarmRoot "work"
$FailedRoot = Join-Path $FarmRoot "failed"
$GB = 1GB

function Log($msg) {
    $line = "{0:yyyy-MM-dd HH:mm:ss} {1}" -f (Get-Date), $msg
    Write-Host $line
    Add-Content -LiteralPath $QueueLog -Value $line
}

function Abort($msg) {
    Log "ABORT | $msg"
    try { Stop-Process -Name "JDownloader2" -Force } catch {}
    exit 1
}

function Skip($msg) {
    Log "SKIP_OVERSIZE | $msg"
    exit 2
}

# --- 0. preconditions -------------------------------------------------
if (-not (Test-Path -LiteralPath $PackageDir)) { Abort "package dir missing: $PackageDir" }
if (-not (Test-Path -LiteralPath $PS5RS)) { Abort "ps5rs binary missing: $PS5RS (run cargo build --release)" }
foreach ($tool in @("7z")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { Abort "required tool missing from PATH: $tool" }
}
$gitDirty = (& git -C $Repo status --porcelain --untracked-files=no) | Where-Object { $_ -ne "" }
if ($gitDirty) { Abort "git tree dirty, commit or stash first" }
New-Item -ItemType Directory -Force -Path $WorkRoot, $FailedRoot | Out-Null

$archives = Get-ChildItem -LiteralPath $PackageDir -File -Include *.zip, *.7z, *.rar, *.001 -Recurse |
    Where-Object { $_.Name -notmatch '\.r\d+$|\.part\d+\.rar$' } |
    Select-Object -First 1
if (-not $archives) { Abort "no archive found in $PackageDir" }
$archive = $archives[0]
Log "START | package=$PackageDir archive=$($archive.Name) size=$([math]::Round($archive.Length / $GB, 2)) GB"

# --- 1. size gate (headers only, no extraction) ------------------------
$listing = & 7z l -slt -- $archive.FullName 2>&1
if ($LASTEXITCODE -ne 0) {
    Move-Item -LiteralPath $PackageDir -Destination (Join-Path $FailedRoot ((Get-Item -LiteralPath $PackageDir).Name)) -Force
    Log "FAIL_CORRUPT | 7z cannot list $($archive.Name), moved to failed, continuing queue"
    exit 2
}
$estimate = 0
foreach ($line in $listing) {
    if ($line -match '^Size = (\d+)$') { $estimate += [long]$Matches[1] }
}
$drive = (Get-Item -LiteralPath $Corpus).PSDrive.Name
$free = (Get-PSDrive -Name $drive).Free
$need = $estimate + $DatasetOverheadGB * $GB
$margin = $SafetyMarginGB * $GB
Log ("ESTIMATE | unpacked={0:N2} GB free={1:N2} GB drive={2}" -f ($estimate / $GB), ($free / $GB), $drive)
if ($need -gt ($free - $margin)) {
    $url = $archive.Name
    Remove-Item -LiteralPath $PackageDir -Recurse -Force
    Skip "too big to fit (need $([math]::Round($need / $GB, 2)) GB). URL hint: $url. Archive deleted."
}

# --- 2. extract + normalize into corpus --------------------------------
$gameName = (Get-Item -LiteralPath $PackageDir).Name -replace '[^\w\-\. ]', '_'
$work = Join-Path $WorkRoot $gameName
if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $work | Out-Null
& 7z x -y "-p$ArchivePassword" "-o$work" -- $archive.FullName | Out-Null
if ($LASTEXITCODE -ne 0) {
    & 7z x -y "-o$work" -- $archive.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Remove-Item -LiteralPath $work -Recurse -Force
        Move-Item -LiteralPath $PackageDir -Destination (Join-Path $FailedRoot $gameName) -Force
        Log "FAIL_CORRUPT | 7z extract failed for $($archive.Name), moved to failed, continuing queue"
        exit 2
    }
}
$eboot = Get-ChildItem -LiteralPath $work -File -Recurse -Filter "eboot.bin" | Select-Object -First 1
if (-not $eboot) {
    Remove-Item -LiteralPath $work -Recurse -Force
    Move-Item -LiteralPath $PackageDir -Destination (Join-Path $FailedRoot $gameName) -Force
    Log "FAIL_LAYOUT | no eboot.bin under $gameName, moved to failed, continuing queue"
    exit 2
}
$dest = Join-Path $Corpus $gameName
if (Test-Path -LiteralPath $dest) { Abort "corpus game already exists: $dest (duplicate package?)" }
Move-Item -LiteralPath $work -Destination $dest

# --- 3-7. ps5rs chain (cumulative rescan over whole corpus) -------------
function Run-Ps5rs([string[]]$PsArgs, [string]$step) {
    Log "RUN | ps5rs $step"
    & $PS5RS @PsArgs
    if ($LASTEXITCODE -ne 0) { Abort "ps5rs $step failed (exit $LASTEXITCODE). Corpus keeps $gameName; archive kept." }
}
Run-Ps5rs @("scan", $Corpus, "-o", $DS, "--include-modules", "--append") "scan --append"
Run-Ps5rs @("batch-load", $Corpus, "-o", (Join-Path $DS "load"), "--offline-dir", (Join-Path $Repo "system_modules")) "batch-load"
Run-Ps5rs @("unknown-nids", $Corpus, "-o", (Join-Path $DS "unknown-nids.json"), "--json") "unknown-nids"
Run-Ps5rs @("middleware", $Corpus, "-o", (Join-Path $DS "middleware.json"), "--format", "json") "middleware"
Run-Ps5rs @("inventory", $Corpus, "-o", (Join-Path $DS "inventory.json"), "--format", "json") "inventory"
Run-Ps5rs @("shader", $Corpus, "-o", (Join-Path $DS "shaders.json"), "--format", "json") "shader"
Run-Ps5rs @("analyze", "--include-modules", "unresolved", $DS, "--format", "json", "-o", (Join-Path $DS "unresolved.json")) "analyze unresolved"
Run-Ps5rs @("dashboard", $DS, "-o", (Join-Path $DS "dashboard"), "--games", $Corpus) "dashboard"
Run-Ps5rs @("validate", "dataset", $DS) "validate dataset"
Run-Ps5rs @("validate", "scan", $DS) "validate scan"

# --- 8. commit dataset ---------------------------------------------------
& git -C $Repo add analysis_with_modules 2>&1 | Out-Null
& git -C $Repo commit -m "data: add $gameName" 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Abort "git commit failed" }
$hash = (& git -C $Repo rev-parse --short HEAD).Trim()

# --- 9. cleanup + next ---------------------------------------------------
Remove-Item -LiteralPath $PackageDir -Recurse -Force
Log "OK | game=$gameName commit=$hash"
exit 0
