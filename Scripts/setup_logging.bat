@echo off
echo Setting up C2 logging system...

REM Create logs directory if it doesn't exist
if not exist logs mkdir logs
echo Created logs directory

REM Move existing log files
if exist client.log (
    move client.log logs\client.log
    echo Moved client.log to logs\client.log
)

if exist teamserver.log (
    move teamserver.log logs\teamserver.log
    echo Moved teamserver.log to logs\teamserver.log
)

if exist agent.log (
    move agent.log logs\agent.log
    echo Moved agent.log to logs\agent.log
)

REM Create empty log files for each component
echo. > logs\client.log
echo. > logs\teamserver.log
echo. > logs\agent.log

echo Logging system setup complete!
echo.
echo Log files will be created in the logs\ directory:
echo - logs\client.log
echo - logs\teamserver.log
echo - logs\agent.log
echo.
pause 