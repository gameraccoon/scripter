@echo off

set SCRIPT_DIR=%~dp0
set SCRIPTER_DIR=%SCRIPT_DIR\.
set LOG_FILE=scripter_logs\last_log_%RANDOM%.txt

start %SCRIPTER_DIR%\scripter.exe > %LOG_FILE% 2>&1
