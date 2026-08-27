@echo off
setlocal EnableExtensions

set "ROMS=C:\Users\claimoar\Documents\ROMS\PS5"
set "OUT=analysis"
set "LOG=update.log"

rem ============================================================
rem START LOG
rem ============================================================

> "%LOG%" echo ============================================================
>> "%LOG%" echo ps5rs analysis update
>> "%LOG%" echo Started: %date% %time%
>> "%LOG%" echo ROMS:    %ROMS%
>> "%LOG%" echo OUTPUT:  %OUT%
>> "%LOG%" echo ============================================================
>> "%LOG%" echo.


rem ============================================================
rem STEP 1 - SCAN
rem ============================================================

call :header "1/4 - Scan PS5 ROMs"

call :run "cargo run -p ps5-cli -- scan --output %OUT% --include-modules %ROMS%"

if errorlevel 1 goto :error


rem ============================================================
rem STEP 2 - VALIDATE
rem ============================================================

call :header "2/4 - Validate dataset"

call :run "cargo run -p ps5-cli -- validate dataset %OUT%"

if errorlevel 1 goto :error


rem ============================================================
rem STEP 2b - MIDDLEWARE
rem ============================================================

call :header "2/4 - Middleware analysis"

call :run_optional "cargo run -p ps5-cli -- middleware %ROMS% --output %OUT%\reports\middleware.json"


rem ============================================================
rem STEP 3 - DEPS
rem ============================================================

if not exist "%OUT%\reports" (
    mkdir "%OUT%\reports"
)

call :header "3/4 - Dependency analysis"

call :run "cargo run -p ps5-cli -- deps --format json --output %OUT%\reports\deps.json %ROMS%"

if errorlevel 1 goto :error


rem ============================================================
rem STEP 3b - GRAPH
rem ============================================================

call :header "3/4 - Generate dependency graph"

cargo run -p ps5-cli -- analyze graph "%OUT%" --format dot > "%OUT%\reports\graph.dot" 2>> "%LOG%"

set "ERR=%errorlevel%"

if not "%ERR%"=="0" (
    echo [FAILED] Dependency graph generation
    >> "%LOG%" echo [FAILED] Dependency graph generation
    >> "%LOG%" echo Exit code: %ERR%
    goto :error
)

echo [OK] Dependency graph generated
>> "%LOG%" echo [OK] Dependency graph generated


rem ============================================================
rem STEP 4 - DASHBOARD
rem ============================================================

call :header "4/4 - Dashboard generation"

call :run "cargo run -p ps5-cli -- dashboard %OUT% --output %OUT%\dashboard\index.html"

if errorlevel 1 goto :error


rem ============================================================
rem SUCCESS
rem ============================================================

echo.
echo ============================================================
echo DONE
echo ============================================================
echo Finished: %date% %time%
echo Dashboard: %OUT%\dashboard\index.html
echo Log: %LOG%
echo ============================================================

>> "%LOG%" echo.
>> "%LOG%" echo ============================================================
>> "%LOG%" echo UPDATE COMPLETED SUCCESSFULLY
>> "%LOG%" echo Finished: %date% %time%
>> "%LOG%" echo Dashboard: %OUT%\dashboard\index.html
>> "%LOG%" echo ============================================================

exit /b 0


rem ============================================================
rem HEADER
rem ============================================================

:header

echo.
echo ============================================================
echo [%~1]
echo ============================================================

>> "%LOG%" echo.
>> "%LOG%" echo ============================================================
>> "%LOG%" echo [%~1]
>> "%LOG%" echo Started: %date% %time%
>> "%LOG%" echo ============================================================

exit /b 0


rem ============================================================
rem RUN COMMAND
rem ============================================================

:run

set "COMMAND=%~1"

echo COMMAND: %COMMAND%
>> "%LOG%" echo COMMAND: %COMMAND%
>> "%LOG%" echo.

powershell.exe -NoProfile -Command ^
    "& { Invoke-Expression '%COMMAND%' 2>&1 | Tee-Object -FilePath '%LOG%' -Append; exit $LASTEXITCODE }"

set "ERR=%errorlevel%"

if not "%ERR%"=="0" (
    echo.
    echo [FAILED] Exit code: %ERR%

    >> "%LOG%" echo.
    >> "%LOG%" echo [FAILED] Exit code: %ERR%
    >> "%LOG%" echo Failed: %date% %time%

    exit /b %ERR%
)

echo.
echo [OK]

>> "%LOG%" echo.
>> "%LOG%" echo [OK]
>> "%LOG%" echo Finished: %date% %time%

exit /b 0


rem ============================================================
rem OPTIONAL RUN
rem ============================================================

:run_optional

set "COMMAND=%~1"

echo COMMAND: %COMMAND%
>> "%LOG%" echo COMMAND: %COMMAND%
>> "%LOG%" echo.

powershell.exe -NoProfile -Command ^
    "& { Invoke-Expression '%COMMAND%' 2>&1 | Tee-Object -FilePath '%LOG%' -Append; exit $LASTEXITCODE }"

set "ERR=%errorlevel%"

if not "%ERR%"=="0" (
    echo.
    echo [WARNING] Optional step failed: %ERR%

    >> "%LOG%" echo.
    >> "%LOG%" echo [WARNING] Optional step failed: %ERR%
    >> "%LOG%" echo Continued because this step is optional.
) else (
    echo.
    echo [OK]

    >> "%LOG%" echo.
    >> "%LOG%" echo [OK]
)

exit /b 0


rem ============================================================
rem ERROR
rem ============================================================

:error

set "ERR=%errorlevel%"

echo.
echo ============================================================
echo UPDATE FAILED
echo ============================================================
echo Error code: %ERR%
echo Failed: %date% %time%
echo See: %LOG%
echo ============================================================

>> "%LOG%" echo.
>> "%LOG%" echo ============================================================
>> "%LOG%" echo UPDATE FAILED
>> "%LOG%" echo Error code: %ERR%
>> "%LOG%" echo Failed: %date% %time%
>> "%LOG%" echo ============================================================

exit /b %ERR%