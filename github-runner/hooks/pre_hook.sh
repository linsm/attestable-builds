#!/bin/bash
set -e;
set -x;

# TODO: figure out why the RUNNER_WORKSPACE and GITHUB_WORKSPACE are sometimes relative dirs

env
echo "Running PRE_HOOK ($0)"

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
OUTPUT_LOG="$SCRIPT_DIR/../output/output.log"

# Checkout repository using PAT do working dir
echo "TIMESTAMP PRE_CHECKOUT $(date -Ins)" >> "$OUTPUT_LOG"

rm -rf "$RUNNER_WORKSPACE"
mkdir -p "$RUNNER_WORKSPACE"

pushd "$RUNNER_WORKSPACE"
git config --global --add safe.directory "*"

# If no GITHUB_REF_NAME is set, default to main
if [ -z "$GITHUB_REF_NAME" ]; then
  echo "GITHUB_REF_NAME not set, defaulting to main" >> "$OUTPUT_LOG"
  GITHUB_REF_NAME="main"
fi

# Clone the repository without history and only the project specific branch
git clone --depth=1 --shallow-submodules --branch="$GITHUB_REF_NAME" "https://$GITHUB_PAT_TOKEN@github.com/$GITHUB_REPOSITORY.git"

# Initialize submodules (if any; mostly applied for the special libxz target)
pushd "$GITHUB_WORKSPACE"
git submodule init
git submodule update --depth 1
popd

echo "TIMESTAMP POST_CHECKOUT $(date -Ins)" >> "$OUTPUT_LOG"
popd

pushd "$GITHUB_WORKSPACE"
GIT_HASH=$(git rev-parse HEAD)
echo "OUTPUT_LOG=$OUTPUT_LOG"
echo "GIT_HASH=$GIT_HASH" >> "$OUTPUT_LOG"
popd