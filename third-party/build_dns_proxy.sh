#!/bin/bash
set -e

SCRIPT_PATH=$(dirname `which $0`)
cd "$SCRIPT_PATH"

mkdir -p build
cd build

echo "[ ] Cloning dnsproxy repo"
rm -rvf dnsproxy
git clone https://github.com/AdguardTeam/dnsproxy.git
cd dnsproxy

echo "[ ] Building (this will download Go)"
make build

echo "[ ] Retrieving artifacts"
cp -v dnsproxy ../../

echo "[+] All done"
