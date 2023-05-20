@echo off

set SCRIPT_DIR=%~dp0
set SCRIPTER_DIR=%SCRIPT_DIR%
set LOG_FOLDER=%SCRIPTER_DIR%\scripter_logs
set LOG_FILE=%LOG_FOLDER%\last_log_%RANDOM%.txt

if not exist "%LOG_FOLDER%" mkdir %LOG_FOLDER%

start %SCRIPTER_DIR%\scripter.exe > %LOG_FILE% 2>&1
