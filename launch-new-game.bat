@echo off
setlocal DisableDelayedExpansion
pushd "%~dp0" || exit /b 1

if defined XIANXIA_PYTHON goto custom
py -3 -c "import sys; sys.exit(sys.version_info < (3, 11))" >nul 2>&1
if not errorlevel 1 goto py_launcher
python -c "import sys; sys.exit(sys.version_info < (3, 11))" >nul 2>&1
if not errorlevel 1 goto python_path
echo Python 3.11 or newer is required. Install it or set XIANXIA_PYTHON.
set "result=1"
goto finish

:custom
"%XIANXIA_PYTHON%" tools\dev.py play --fresh %*
goto capture

:py_launcher
py -3 tools\dev.py play --fresh %*
goto capture

:python_path
python tools\dev.py play --fresh %*

:capture
set "result=%errorlevel%"

:finish
popd
if not "%result%"=="0" (
    echo Launch failed. Review the message above and the run evidence folder.
    pause
)
exit /b %result%
