@echo off
echo ========================================
echo C2 Framework - Cleanup Script
echo ========================================
echo.

echo [1/3] Killing all C2 processes...
taskkill /F /IM teamserver.exe 2>nul
taskkill /F /IM agent.exe 2>nul
taskkill /F /IM client.exe 2>nul
echo ✓ Processes cleaned up

echo.
echo [2/3] Cleaning up sessions.json...
if exist "%~dp0..\sessions.json" (
    echo Clearing all sessions...
    echo [] > "%~dp0..\sessions.json"
    echo ✓ All sessions cleared
) else (
    echo No sessions.json file found
)

echo.
echo [3/3] Cleaning up temporary files...
if exist target\debug rmdir /s /q target\debug 2>nul
if exist target\release\*.pdb del /q target\release\*.pdb 2>nul
echo ✓ Temporary files cleaned up

echo.
echo ========================================
echo Cleanup completed!
echo ========================================
echo.
echo What was cleaned:
echo - C2 processes (teamserver.exe, agent.exe, client.exe)
echo - Sessions (based on your choice)
echo - Temporary build files
echo.
pause 