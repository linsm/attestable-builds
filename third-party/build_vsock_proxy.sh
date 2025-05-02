#!/bin/bash
set -e

SCRIPT_PATH=$(dirname `which $0`)
cd "$SCRIPT_PATH"

mkdir -p build
cd build

echo "[ ] Cloning TCP-VSOCK proxy repo"
rm -rvf oyster-tcp-proxy
git clone https://github.com/marlinprotocol/oyster-tcp-proxy.git
cd oyster-tcp-proxy

echo "[ ] Patching using local oyster-tcp-proxy.patch"
git apply ../../build_vsock_proxy.patch

echo "[ ] Building"
cargo build --release

echo "[ ] Retrieving artifacts"
cp -v ./target/release/ip-to-vsock-transparent ./target/release/vsock-to-ip-transparent ../../

echo "[+] All done"
