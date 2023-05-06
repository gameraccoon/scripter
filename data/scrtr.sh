#!/usr/bin/env bash

SCRTR_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
LOG_FILE=scripter_logs\\\\last_log_$RANDOM.txt

start $SCRTR_DIR\\\\scripter.exe > LOG_FILE 2>&1
