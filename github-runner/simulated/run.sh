#!/bin/bash
set -e;
set -x;

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
pushd "$SCRIPT_DIR"

OUTPUT_LOG="$SCRIPT_DIR/../output/output.log"
echo "OUTPUT_LOG=$OUTPUT_LOG"

export RUNNER_WORKSPACE="$SCRIPT_DIR/simulated_workspace"

REPOSITORY_NAME=$(echo "$GITHUB_REPOSITORY" | cut -d'/' -f2)
export GITHUB_WORKSPACE="$RUNNER_WORKSPACE/$REPOSITORY_NAME"

/bin/bash "$ACTIONS_RUNNER_HOOK_JOB_STARTED"
pushd "$GITHUB_WORKSPACE"

# If SUBPROJECT_DIR is set, cd into it
if [ -n "$SUBPROJECT_DIR" ]; then
  pushd "$SUBPROJECT_DIR"
else
  echo "SUBPROJECT_DIR is not set"
fi

# Keep in sync with the action.yml files
ls -la
chmod +x build.sh
/bin/bash build.sh

rm -rvf "$RUNNER_WORKSPACE"
