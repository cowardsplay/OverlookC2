@echo off
echo Starting C2 System with key: testkey123
echo.

echo Starting Teamserver...
start "Teamserver" cmd /k "cd /d %~dp0 && target\release\teamserver.exe --key testkey123"

echo Waiting 3 seconds for teamserver to start...
timeout /t 3 /nobreak > nul

echo Starting Client...
start "Client" cmd /k "cd /d %~dp0 && target\release\client.exe --key testkey123 start"

echo.
echo C2 System started!
echo - Teamserver: ws://127.0.0.1:8080
echo - Encryption Key: testkey123
echo.
echo To run the agent, use: payload.bat
echo.
pause 