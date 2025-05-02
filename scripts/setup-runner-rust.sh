#!/bin/bash
set -e;

# Install rust for the runner user
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo -u runner sh -s -- --profile minimal -y;
