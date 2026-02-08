#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"

BASE_URL="http://127.0.0.1:39090"
USER="alice"
PASS="alice-pass"

# Build xin if needed
cd "$ROOT_DIR"
cargo build -q

export XIN_BASE_URL="$BASE_URL"
export XIN_BASIC_USER="$USER"
export XIN_BASIC_PASS="$PASS"

# Minimal connectivity check: should succeed even with empty inbox.
OUT="$("$ROOT_DIR/target/debug/xin" messages search --filter-json '{"text":"xin-smoke"}' --max 1)"

echo "$OUT" | /usr/bin/grep -q '"ok": true'
echo "$OUT" | /usr/bin/grep -q '"command": "messages.search"'

echo "OK: xin can connect to Stalwart via Basic auth (JMAP)."