#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://127.0.0.1:39090"
API="$BASE_URL/api"
ADMIN_USER="admin"
ADMIN_PASS="xin-admin-pass"
DOMAIN="example.org"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd python3

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
{ "type": "domain", "name": "$DOMAIN" }
JSON
)"
  echo
}

create_user() {
  local user="$1"     # alice
  local pass="$2"     # alice-pass
  local email="$user@$DOMAIN"

  echo "Creating user principal: $email"
  api_json -X POST "$API/principal" -d "$(cat <<JSON
{ "type": "individual", "name": "$user", "emails": ["$email"], "secrets": ["$pass"], "roles": ["user"] }
JSON
)"
  echo
}

wait_ready

# Idempotency: for now, we do best-effort and allow failures to surface.
create_domain || true
create_user "alice" "alice-pass" || true
create_user "bob" "bob-pass" || true

ensure_identity() {
  local user="$1"   # alice
  local pass="$2"   # alice-pass
  local email="$3"  # alice@example.test
  local name="$4"   # Alice

  echo "Ensuring JMAP identity exists for $user ($email)"

  local account_id
  account_id="$(curl -fsSL -u "$user:$pass" "$BASE_URL/.well-known/jmap" | python3 -c 'import json,sys; s=json.load(sys.stdin); print(s["primaryAccounts"]["urn:ietf:params:jmap:mail"])')"

  local resp
  resp="$(curl -fsS -u "$user:$pass" -H 'Content-Type: application/json' \
    -d "{\"using\":[\"urn:ietf:params:jmap:core\",\"urn:ietf:params:jmap:mail\"],\"methodCalls\":[[\"Identity/get\",{\"accountId\":\"$account_id\"},\"0\"]]}" \
    "$BASE_URL/jmap")"

  local has_any
  has_any="$(python3 -c 'import json,sys; resp=json.load(sys.stdin); mr=resp.get("methodResponses", []); obj = mr[0][1] if mr and len(mr[0])>1 else {}; items = obj.get("list", []) if isinstance(obj, dict) else []; print("1" if items else "0")' <<<"$resp")"

  if [ "$has_any" = "1" ]; then
    echo "  Identity already exists."
    return 0
  fi

  echo "  Creating identity via Identity/set"
  curl -fsS -u "$user:$pass" -H 'Content-Type: application/json' \
    -d "{\"using\":[\"urn:ietf:params:jmap:core\",\"urn:ietf:params:jmap:mail\"],\"methodCalls\":[[\"Identity/set\",{\"accountId\":\"$account_id\",\"create\":{\"c0\":{\"name\":\"$name\",\"email\":\"$email\"}}},\"0\"]]}" \
    "$BASE_URL/jmap" >/dev/null

  echo "  Identity created."
}

# Needed for xin send (EmailSubmission requires identityId).
ensure_identity "alice" "alice-pass" "alice@example.org" "Alice"

echo "Seed done."
echo "  alice: alice@$DOMAIN / alice-pass"
echo "  bob:   bob@$DOMAIN / bob-pass"
