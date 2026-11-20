@echo off
setlocal DisableDelayedExpansion
call "%~dp0launch-new-game.bat" --scenario frontier-x4
exit /b %errorlevel%
