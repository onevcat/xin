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

# Start watch in background, then inject a new email as another user (bob) and
# confirm watch receives it.
OUTFILE="$(mktemp)"
ERRFILE="${OUTFILE}.err"

"$ROOT_DIR/target/debug/xin" watch \
  --since "$STATE" \
  --no-envelope \
  --interval-ms 100 \
  --jitter-ms 0 \
  >"$OUTFILE" 2>"$ERRFILE" &
PID=$!

cleanup() {
  if kill -0 "$PID" >/dev/null 2>&1; then
    kill -INT "$PID" >/dev/null 2>&1 || true
    wait "$PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_grep() {
  local pat="$1"
  local tries="$2"
  local sleep_s="$3"

  for _ in $(seq 1 "$tries"); do
    if /usr/bin/grep -q "$pat" "$OUTFILE" 2>/dev/null; then
      return 0
    fi
    sleep "$sleep_s"
  done
  return 1
}

# Wait for ready event.
if ! wait_for_grep '"type":"ready"' 120 0.05; then
  echo "watch_check failed: ready event not observed" >&2
  echo "stderr:" >&2
  sed -n '1,200p' "$ERRFILE" >&2 || true
  echo "stdout:" >&2
  sed -n '1,200p' "$OUTFILE" >&2 || true
  exit 1
fi

# Inject one mail so watch sees a change.
python3 "$STALWART_DIR/scripts/smtp_inject.py" \
  --auth-user bob \
  --auth-pass bob-pass \
  --mail-from bob@example.org \
  --rcpt-to alice@example.org \
  --eml "$STALWART_DIR/fixtures/simple_subject_space.eml" >/dev/null

# Wait until we observe a change event.
if ! wait_for_grep '"type":"email.change"' 240 0.05; then
  echo "watch_check failed: email.change not observed after injection" >&2
  echo "stderr:" >&2
  sed -n '1,200p' "$ERRFILE" >&2 || true
  echo "stdout:" >&2
  sed -n '1,200p' "$OUTFILE" >&2 || true
  exit 1
fi

# Make sure --no-envelope really means no envelope.
if /usr/bin/grep -q '"schemaVersion"' "$OUTFILE"; then
  echo "watch_check failed: expected no envelope when --no-envelope is set" >&2
  sed -n '1,200p' "$OUTFILE" >&2 || true
  exit 1
fi

# Stop watch.
kill -INT "$PID" >/dev/null 2>&1 || true
wait "$PID" >/dev/null 2>&1 || true

echo "OK: watch e2e observed a new incoming email during watch (NDJSON stream, no envelope)."
