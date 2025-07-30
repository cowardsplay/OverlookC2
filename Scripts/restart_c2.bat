@echo off
echo Stopping any existing C2 processes...
taskkill /f /im teamserver.exe 2>nul
taskkill /f /im client.exe 2>nul
taskkill /f /im agent.exe 2>nul

echo.
echo Starting C2 System with key: testkey123
echo.

echo Starting Teamserver...
start "Teamserver" cmd /k "cd /d %~dp0 && target\release\teamserver.exe --key testkey123"

echo Waiting 5 seconds for teamserver to start...
timeout /t 5 /nobreak > nul

echo Starting Client...
start "Client" cmd /k "cd /d %~dp0 && target\release\client.exe --key testkey123 start"

echo.
echo C2 System restarted!
echo - Teamserver: ws://127.0.0.1:8080
echo - Encryption Key: testkey123
echo.
echo To run the agent, use: payload.bat
echo.
pause 