#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
STALWART_DIR="$ROOT_DIR/tests/feature/stalwart"

BASE_URL="http://127.0.0.1:39090"
USER="alice"
PASS="alice-pass"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

need_cmd python3
need_cmd /usr/bin/nc

# If Stalwart isn't running, start it.
if ! /usr/bin/nc -z 127.0.0.1 39090 >/dev/null 2>&1; then
  echo "Stalwart not running; starting harness..." >&2
  (cd "$STALWART_DIR" && ./scripts/up.sh)
fi

# Ensure users + identity exist (idempotent).
(cd "$STALWART_DIR" && ./scripts/seed.sh >/dev/null)

cd "$ROOT_DIR"
cargo build -q

export XIN_BASE_URL="$BASE_URL"
export XIN_BASIC_USER="$USER"
export XIN_BASIC_PASS="$PASS"

# Get current cursor (S0).
STATE="$($ROOT_DIR/target/debug/xin history | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["sinceState"])')"
if [ -z "$STATE" ]; then
  echo "watch_check: failed to obtain sinceState" >&2
  exit 1
fi

# Inject one mail so watch sees a change.
python3 "$STALWART_DIR/scripts/smtp_inject.py" \
  --auth-user bob \
  --auth-pass bob-pass \
  --mail-from bob@example.org \
  --rcpt-to alice@example.org \
  --eml "$STALWART_DIR/fixtures/simple_subject_space.eml" >/dev/null

OUT="$($ROOT_DIR/target/debug/xin watch --since "$STATE" --once --no-envelope --interval-ms 50 --jitter-ms 0)"

if ! echo "$OUT" | /usr/bin/grep -q '"type":"ready"'; then
  echo "watch_check failed: missing ready event. Output:" >&2
  echo "$OUT" >&2
  exit 1
fi

if echo "$OUT" | /usr/bin/grep -q '"schemaVersion"'; then
  echo "watch_check failed: expected no envelope when --no-envelope is set. Output:" >&2
  echo "$OUT" >&2
  exit 1
fi

if ! echo "$OUT" | /usr/bin/grep -q '"type":"email.change"'; then
  echo "watch_check failed: missing email.change event. Output:" >&2
  echo "$OUT" >&2
  exit 1
fi

echo "OK: watch e2e produced NDJSON change events (no envelope)."
