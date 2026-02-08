#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://127.0.0.1:39090"
USER="alice@example.test"
PASS="alice-pass"

# Build xin if needed
cargo build -q

export XIN_BASE_URL="$BASE_URL"
export XIN_BASIC_USER="$USER"
export XIN_BASIC_PASS="$PASS"

# Minimal connectivity check: should succeed even with empty inbox.
./target/debug/xin messages search --filter-json '{"text":"xin-smoke"}' --max 1 >/dev/null

echo "OK: xin can connect to Stalwart via Basic auth (JMAP)."