#!/bin/bash
# Development launcher for Caw (pure Rust, gpui-kit).
set -euo pipefail
cd "$(dirname "$0")/.." && exec cargo run "$@"
