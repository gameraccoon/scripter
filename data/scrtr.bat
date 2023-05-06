@echo off

set SCRTR_DIR=%~dp0
set LOG_FILE=scripter_logs\last_log_%RANDOM%.txt

start %SCRTR_DIR%\scripter.exe > %LOG_FILE% 2>&1
