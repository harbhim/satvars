#!/usr/bin/env bash

set -e

cargo fmt --all
cargo clippy --workspace --all-targets --all-features --fix
