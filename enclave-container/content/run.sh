#!/bin/bash
set -e;

id;
env;

cd /app;

./enclave-client "ANY:11000";
cat github-runner/output/output.log || true;
cat tmp/output/output.log || true;
