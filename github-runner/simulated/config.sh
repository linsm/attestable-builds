#!/bin/bash
set -e;

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
pushd "$SCRIPT_DIR"

OUTPUT_LOG="$SCRIPT_DIR/../output/output.log"
echo "OUTPUT_LOG=$OUTPUT_LOG"

# Sleep a random amount of time
sleep $(( ( RANDOM % 5 )  + 1 ))

export RUNNER_WORKSPACE="$SCRIPT_DIR/simulated_workspace"
rm -rf "$RUNNER_WORKSPACE"
mkdir -p "$RUNNER_WORKSPACE"
