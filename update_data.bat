@echo off
setlocal

set "ROMS=C:\Users\claimoar\Documents\ROMS\PS5"
set "OUT=analysis"
set "LOG=update.log"

echo ============================================================ > "%LOG%"
echo ps5rs analysis update started %date% %time% >> "%LOG%"
echo ============================================================ >> "%LOG%"

echo [1/4] Scanning %ROMS% -^> %OUT% (ps5-prx + ps5-image + ps5-schema + ps5-nid-db)
echo [1/4] Scanning %ROMS% -^> %OUT% (ps5-prx + ps5-image + ps5-schema + ps5-nid-db) >> "%LOG%"

cargo run -p ps5-cli -- scan --output "%OUT%" --include-modules "%ROMS%" 2>&1 | tee "%TEMP%\ps5rs_cmd.log"
type "%TEMP%\ps5rs_cmd.log" >> "%LOG%"
del "%TEMP%\ps5rs_cmd.log"

if errorlevel 1 goto :error


echo [2/4] Validating dataset + middleware (ps5-signatures, ps5-deps, ps5-sdk-meta, ps5-firmware, ps5-shader)
echo [2/4] Validating dataset + middleware (ps5-signatures, ps5-deps, ps5-sdk-meta, ps5-firmware, ps5-shader) >> "%LOG%"

cargo run -p ps5-cli -- validate dataset "%OUT%" 2>&1 | tee "%TEMP%\ps5rs_cmd.log"
type "%TEMP%\ps5rs_cmd.log" >> "%LOG%"
del "%TEMP%\ps5rs_cmd.log"

if errorlevel 1 goto :error


cargo run -p ps5-cli -- middleware "%ROMS%" --output "%OUT%\reports\middleware.json" 2>&1 | tee "%TEMP%\ps5rs_cmd.log"
type "%TEMP%\ps5rs_cmd.log" >> "%LOG%"
del "%TEMP%\ps5rs_cmd.log"

rem optional - ignore failure


echo [3/4] Deps / graph
echo [3/4] Deps / graph >> "%LOG%"

cargo run -p ps5-cli -- deps --format json --output "%OUT%\reports\deps.json" "%ROMS%" 2>&1 | tee "%TEMP%\ps5rs_cmd.log"
type "%TEMP%\ps5rs_cmd.log" >> "%LOG%"
del "%TEMP%\ps5rs_cmd.log"

if errorlevel 1 goto :error

if not exist "%OUT%\reports" mkdir "%OUT%\reports"

cargo run -p ps5-cli -- analyze graph "%OUT%" --format dot > "%OUT%\reports\graph.dot" 2>> "%LOG%"
if errorlevel 1 goto :error


echo [4/4] Dashboard (ps5-dashboard consumes all above)
echo [4/4] Dashboard (ps5-dashboard consumes all above) >> "%LOG%"

cargo run -p ps5-cli -- dashboard "%OUT%" --output "%OUT%\dashboard\index.html" 2>&1 | tee "%TEMP%\ps5rs_cmd.log"
type "%TEMP%\ps5rs_cmd.log" >> "%LOG%"
del "%TEMP%\ps5rs_cmd.log"

if errorlevel 1 goto :error


echo ============================================================ >> "%LOG%"
echo Done. %date% %time% >> "%LOG%"
echo Dashboard: %OUT%\dashboard\index.html >> "%LOG%"
echo Multi-page: %OUT%\dashboard\ (index/shader/firmware/deps.html) via output dir >> "%LOG%"
echo ============================================================ >> "%LOG%"

echo.
echo Done. Dashboard: %OUT%\dashboard\index.html
echo Multi-page: %OUT%\dashboard\ (index/shader/firmware/deps.html) via output dir
echo Log: %LOG%
exit /b 0


:error
echo Failed with error %errorlevel%
echo ============================================================ >> "%LOG%"
echo FAILED with error %errorlevel% at %date% %time% >> "%LOG%"
echo ============================================================ >> "%LOG%"
exit /b %errorlevel%