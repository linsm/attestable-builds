#!/bin/bash
set -e;

echo "Running ATTESTATION_HOOK ($0) with ARTIFACT_PATH=$1"

SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")

OUTPUT_LOG="$SCRIPT_DIR/../output/output.log"
# echo "OUTPUT_LOG=$OUTPUT_LOG"

INPUT_LOG="$SCRIPT_DIR/../output/input.log"
# echo "INPUT_LOG=$INPUT_LOG"

# Output the artifact name and hash to the output log (which then gets picked up by the Enclave Client)
ARTIFACT_PATH=$1
echo "ARTIFACT_NAME_AND_HASH=$(basename $ARTIFACT_PATH);$(sha256sum $ARTIFACT_PATH | cut -d ' ' -f 1)" >> "$OUTPUT_LOG"

# Wait for the input log to contain at least one line and then write it into a .cert file based on the artifact path
CERT_PATH="$ARTIFACT_PATH.cert"
while [ ! -s "$INPUT_LOG" ]; do
  echo "Waiting for attestation result..."
  sleep 1
done
cp "$INPUT_LOG" "$CERT_PATH"

echo "Content of $CERT_PATH:"
cat "$CERT_PATH" | jq
echo "---end---"
