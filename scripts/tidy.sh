#!/bin/bash
set -e
cargo clippy --fix --allow-dirty --allow-staged;
cargo fmt;
