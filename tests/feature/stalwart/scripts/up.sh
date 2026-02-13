#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="$ROOT_DIR/.state/opt-stalwart"

HTTP_PORT=39090
SMTP_PORT=32525

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

port_free() {
  local port="$1"
  if /usr/bin/nc -z 127.0.0.1 "$port" >/dev/null 2>&1; then
    echo "port $port already in use on 127.0.0.1" >&2
    exit 1
  fi
}

need_cmd docker
need_cmd /usr/bin/nc

# Ensure the bind-mounted state directory stays owned by the current user (important on Linux CI).
export STALWART_UID="$(id -u)"
export STALWART_GID="$(id -g)"

port_free "$HTTP_PORT"
port_free "$SMTP_PORT"

mkdir -p "$STATE_DIR/etc"
cp -f "$ROOT_DIR/config.toml" "$STATE_DIR/etc/config.toml"

cd "$ROOT_DIR"
docker compose up -d

echo "Stalwart starting..."
echo "  JMAP/HTTP: http://127.0.0.1:${HTTP_PORT}"
echo "  SMTP:      127.0.0.1:${SMTP_PORT}"
