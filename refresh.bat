@echo off
setlocal EnableDelayedExpansion

REM ============================================================================
REM ps5rs - Full Refresh Pipeline
REM Rebuilds the binary, syncs NID catalog, rescans games, and regenerates
REM all analysis artifacts (dataset, load reports, dashboard).
REM
REM Usage:
REM   refresh.bat                 - use defaults (./games -> ./analysis)
REM   refresh.bat D:\Games C:\out  - custom games dir and output dir
REM   refresh.bat --no-build       - skip cargo build
REM   refresh.bat --clean          - wipe analysis/ first
REM ============================================================================

set "GAMES_DIR="
set "OUTPUT_DIR="
set "SKIP_BUILD=0"
set "CLEAN=0"
set "GAMES_DEFAULT=games"
set "OUTPUT_DEFAULT=analysis"
set "SYSTEM_MODULES=system_modules"

REM --- robust arg parsing (handles quoted paths with spaces, flags in any order) ---
:parse_args
if "%~1"=="" goto :args_done
if /I "%~1"=="--no-build" (set "SKIP_BUILD=1" & shift & goto :parse_args)
if /I "%~1"=="--clean"    (set "CLEAN=1" & shift & goto :parse_args)
if /I "%~1"=="--help"     goto :help
if /I "%~1"=="-h"         goto :help
if not defined GAMES_DIR  (set "GAMES_DIR=%~1" & shift & goto :parse_args)
if not defined OUTPUT_DIR (set "OUTPUT_DIR=%~1" & shift & goto :parse_args)
REM unknown positional - ignore
shift
goto :parse_args
:args_done
if not defined GAMES_DIR  set "GAMES_DIR=%GAMES_DEFAULT%"
if not defined OUTPUT_DIR set "OUTPUT_DIR=%OUTPUT_DEFAULT%"

echo ============================================================
echo  ps5rs refresh - %date% %time%
echo  Games : %GAMES_DIR%
echo  Output: %OUTPUT_DIR%
echo ============================================================
echo.

REM --- preflight ---
if not exist "%GAMES_DIR%" (
    echo [ERROR] Games directory not found: %GAMES_DIR%
    echo         Pass it explicitly: refresh.bat "D:\PS5\Games" analysis
    exit /b 1
)

if "%CLEAN%"=="1" (
    echo [CLEAN] Removing %OUTPUT_DIR% ...
    if exist "%OUTPUT_DIR%" rmdir /S /Q "%OUTPUT_DIR%"
    echo(
)

REM ===================================================================
REM 1. Build
REM ===================================================================
if "%SKIP_BUILD%"=="0" (
    echo [1/8] Building ps5rs release ...
    cargo build --release
    if errorlevel 1 (
        echo [ERROR] cargo build failed
        exit /b 1
    )
    echo(
) else (
    echo [1/8] Skipping build --no-build
    echo(
)

REM ===================================================================
REM 2. Catalog - local nids.csv only (Supabase sync disabled per user request)
REM ===================================================================
echo [2/8] Using local NID catalog - Supabase sync skipped
echo        Catalog: data/nids.csv ^(165k+ entries, 6-col consolidated, built-in^)
echo        No network download - HV4j+E0MBHE etc. resolved from local file
echo.

REM ===================================================================
REM 3. Scan - eboot.bin -> BinaryImage dataset
REM ===================================================================
echo [3/8] Scanning games: %GAMES_DIR% -^> %OUTPUT_DIR%
echo        This re-resolves NIDs with the patched ps5-nid parser.
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" scan "%GAMES_DIR%" --output "%OUTPUT_DIR%"
) else (
    cargo run --release --bin ps5rs -- scan "%GAMES_DIR%" --output "%OUTPUT_DIR%"
)
if errorlevel 1 (
    echo [ERROR] scan failed
    exit /b 1
)
echo.

echo [3b/8] Scanning with PRX modules ^(sce_module/*.prx^) ...
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" scan "%GAMES_DIR%" --output "%OUTPUT_DIR%_with_modules" --include-modules
) else (
    cargo run --release --bin ps5rs -- scan "%GAMES_DIR%" --output "%OUTPUT_DIR%_with_modules" --include-modules
)
if errorlevel 1 echo [WARN] scan --include-modules failed
echo.

REM ===================================================================
REM 4. Export-scan - build offline export DB for loader
REM ===================================================================
echo [4/8] Export-scan: building %SYSTEM_MODULES% from PRXes ...
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" export-scan "%GAMES_DIR%" --output "%SYSTEM_MODULES%"
) else (
    cargo run --release --bin ps5rs -- export-scan "%GAMES_DIR%" --output "%SYSTEM_MODULES%"
)
if errorlevel 1 echo [WARN] export-scan failed
echo.

REM ===================================================================
REM 5. Batch-load - virtual loader across all games
REM ===================================================================
echo [5/8] Batch-load: running loader pipeline on all games ...
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" batch-load "%GAMES_DIR%" --output "%OUTPUT_DIR%\load"
) else (
    cargo run --release --bin ps5rs -- batch-load "%GAMES_DIR%" --output "%OUTPUT_DIR%\load"
)
if errorlevel 1 echo [WARN] batch-load failed
echo.

REM ===================================================================
REM 6. Analysis reports - stats, unknown NIDs (now catalog-aware)
REM ===================================================================
echo [6/8] Generating analysis reports ...
echo        - stats
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" analyze stats "%OUTPUT_DIR%" --format terminal
) else (
    cargo run --release --bin ps5rs -- analyze stats "%OUTPUT_DIR%" --format terminal
)
echo.
echo        - unknown NIDs ^(re-resolved with live catalog - HV4j should be 0^)
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" analyze unknown "%OUTPUT_DIR%" --format terminal
) else (
    cargo run --release --bin ps5rs -- analyze unknown "%OUTPUT_DIR%" --format terminal
)
echo.
echo        - engines
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" analyze engines "%OUTPUT_DIR%" --format terminal
) else (
    cargo run --release --bin ps5rs -- analyze engines "%OUTPUT_DIR%" --format terminal
)
echo.
echo        - imports
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" analyze imports "%OUTPUT_DIR%" --format terminal
) else (
    cargo run --release --bin ps5rs -- analyze imports "%OUTPUT_DIR%" --format terminal
)
echo.

echo        Exporting unknown NIDs to %OUTPUT_DIR%\unknown_nids.json ...
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" analyze unknown "%OUTPUT_DIR%" --format json -o "%OUTPUT_DIR%\unknown_nids.json"
    "target\release\ps5rs.exe" analyze unknown "%OUTPUT_DIR%" --format csv -o "%OUTPUT_DIR%\unknown_nids.csv"
    "target\release\ps5rs.exe" export-unknown "%OUTPUT_DIR%" -o "%OUTPUT_DIR%\export_unknown_by_freq.csv"
    "target\release\ps5rs.exe" export-unknown "%OUTPUT_DIR%" --group-by library -o "%OUTPUT_DIR%\export_unknown_by_lib.csv"
) else (
    cargo run --release --bin ps5rs -- analyze unknown "%OUTPUT_DIR%" --format json -o "%OUTPUT_DIR%\unknown_nids.json"
    cargo run --release --bin ps5rs -- analyze unknown "%OUTPUT_DIR%" --format csv -o "%OUTPUT_DIR%\unknown_nids.csv"
    cargo run --release --bin ps5rs -- export-unknown "%OUTPUT_DIR%" -o "%OUTPUT_DIR%\export_unknown_by_freq.csv"
    cargo run --release --bin ps5rs -- export-unknown "%OUTPUT_DIR%" --group-by library -o "%OUTPUT_DIR%\export_unknown_by_lib.csv"
)
echo.

REM ===================================================================
REM 7. Dashboard - static HTML (self-contained, no CDN)
REM ===================================================================
echo [7/8] Generating dashboard ...
if exist "target\release\ps5rs.exe" (
    "target\release\ps5rs.exe" dashboard "%OUTPUT_DIR%" --output "%OUTPUT_DIR%\dashboard" --games "%GAMES_DIR%"
) else (
    cargo run --release --bin ps5rs -- dashboard "%OUTPUT_DIR%" --output "%OUTPUT_DIR%\dashboard" --games "%GAMES_DIR%"
)
if errorlevel 1 echo [WARN] dashboard generation failed
echo        -^> %OUTPUT_DIR%\dashboard\index.html
echo.

REM ===================================================================
REM 8. Tests - verify patched parser + stale-dataset fix
REM ===================================================================
echo [8/8] Running regression tests ...
cargo test -p ps5-nid --lib
if errorlevel 1 echo [WARN] ps5-nid tests failed
cargo test -p ps5-analysis --lib reports::unknown_nids
if errorlevel 1 echo [WARN] ps5-analysis tests failed
echo.

echo ============================================================
echo  Refresh complete - %date% %time%
echo ============================================================
echo  Outputs:
echo    Dataset ........ %OUTPUT_DIR%\images\ ^(BinaryImage JSON^)
echo    With modules ... %OUTPUT_DIR%_with_modules\
echo    Load reports ... %OUTPUT_DIR%\load\
echo    System modules . %SYSTEM_MODULES%\
echo    Dashboard ...... %OUTPUT_DIR%\dashboard\index.html
echo    Unknown JSON ... %OUTPUT_DIR%\unknown_nids.json
echo    Unknown CSV .... %OUTPUT_DIR%\unknown_nids.csv
echo.
echo  Next:
echo    ps5rs analyze heatmap %OUTPUT_DIR%
echo    ps5rs analyze graph %OUTPUT_DIR% --format dot -o graph.dot
echo    ps5rs middleware "%GAMES_DIR%" --format json -o middleware.json
echo    ps5rs dashboard %OUTPUT_DIR% --games "%GAMES_DIR%"
echo ============================================================
exit /b 0

:help
echo Usage: refresh.bat [GAMES_DIR] [OUTPUT_DIR] [OPTIONS]
echo.
echo   GAMES_DIR   Directory containing game subfolders with eboot.bin ^(default: games^)
echo   OUTPUT_DIR  Dataset output directory ^(default: analysis^)
echo.
echo Options:
echo   --clean     Delete OUTPUT_DIR before scanning
echo   --no-build  Skip cargo build --release
echo   --help      Show this help
echo.
echo Examples:
echo   refresh.bat
echo   refresh.bat "D:\PS5\Games" analysis --clean
echo   refresh.bat "C:\Users\claimoar\Documents\ROMS\PS5" analysis --clean
echo   refresh.bat --no-build
exit /b 0
