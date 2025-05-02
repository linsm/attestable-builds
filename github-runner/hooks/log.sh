#!/bin/bash
set -e;

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
OUTPUT_LOG="$SCRIPT_DIR/../output/output.log"

echo "$@" >> "$OUTPUT_LOG"
