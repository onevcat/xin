#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://127.0.0.1:39090"
API="$BASE_URL/api"
ADMIN_USER="admin"
ADMIN_PASS="xin-admin-pass"
DOMAIN="example.test"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

need_cmd curl

api() {
  # Basic-auth is the most likely scheme for local fallback-admin.
  curl -fsS -u "$ADMIN_USER:$ADMIN_PASS" \
    -H 'Accept: application/json' \
    "$@"
}

api_json() {
  curl -fsS -u "$ADMIN_USER:$ADMIN_PASS" \
    -H 'Accept: application/json' \
    -H 'Content-Type: application/json' \
    "$@"
}

wait_ready() {
  echo "Waiting for management API at $API ..."
  for i in $(seq 1 60); do
    if api "$API/principal?limit=1" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
  done
  echo "Management API did not become ready. Check: docker logs xin-stalwart" >&2
  exit 1
}

create_domain() {
  echo "Creating domain principal: $DOMAIN"
  api_json -X POST "$API/principal" -d "$(cat <<JSON
{
  \"type\": \"domain\",
  \"name\": \"$DOMAIN\",
  \"description\": \"xin local test domain\",
  \"quota\": 0,
  \"secrets\": [],
  \"emails\": [],
  \"memberOf\": [],
  \"roles\": [],
  \"lists\": [],
  \"members\": []
}
JSON
)" >/dev/null
}

create_user() {
  local user="$1"     # alice
  local pass="$2"     # alice-pass
  local email="$user@$DOMAIN"

  echo "Creating user principal: $email"
  api_json -X POST "$API/principal" -d "$(cat <<JSON
{
  \"type\": \"individual\",
  \"name\": \"$user\",
  \"description\": \"xin local test user\",
  \"quota\": 0,
  \"secrets\": [\"$pass\"],
  \"emails\": [\"$email\"],
  \"memberOf\": [],
  \"roles\": [\"user\"],
  \"lists\": [\"all\"],
  \"members\": []
}
JSON
)" >/dev/null
}

wait_ready

# Idempotency: for now, we do best-effort and allow failures to surface.
create_domain || true
create_user "alice" "alice-pass" || true
create_user "bob" "bob-pass" || true

echo "Seed done."
echo "  alice: alice@$DOMAIN / alice-pass"
echo "  bob:   bob@$DOMAIN / bob-pass"
