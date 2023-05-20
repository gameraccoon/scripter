#!/usr/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
SCRIPTER_DIR=$SCRIPT_DIR/.
LOG_FOLDER=$SCRIPTER_DIR/scripter_logs
LOG_FILE=$LOG_FOLDER/last_log_$RANDOM.txt

mkdir -p "$LOG_FOLDER"

"$SCRIPTER_DIR"/scripter > "$LOG_FILE" 2>&1 &
