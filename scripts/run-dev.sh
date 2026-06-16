#!/bin/bash
# Development launcher for Caw — called from the .desktop file.
set -euo pipefail
cd "$(dirname "$0")/.." && exec cargo tauri dev
