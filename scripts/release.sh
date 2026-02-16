#!/usr/bin/env bash
#
# xin release script
# Usage: ./scripts/release.sh <version>
#
# Release invariant:
# - The *tagged commit* must be self-consistent: version/changelog/generated docs/tests all match.
# - Do NOT tag first and “fix docs later” (CI will (correctly) fail).
#
# This script:
# 1) Bumps Cargo.toml version
# 2) Moves CHANGELOG [Unreleased] body into a new release section and resets [Unreleased]
# 3) Regenerates generated docs (skills/xin/*)
# 4) Runs tests (serialized to reduce OOM/SIGKILL flakes)
# 5) Commits, tags, pushes
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

auto_tag() {
  # normalize: always tag as vX.Y.Z
  local v="$1"
  if [[ "$v" == v* ]]; then
    echo "$v"
  else
    echo "v$v"
  fi
}

usage() {
  echo "Usage: $0 <version>"
  echo "Example: $0 0.2.0"
  exit 1
}

if [[ $# -ne 1 ]]; then
  usage
fi

VERSION="$1"

# Validate version format (semver)
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  log_error "Invalid version format. Use semver (e.g., 0.1.0 or 0.1.0-beta.1)"
  exit 1
fi

VERSION_TAG="$(auto_tag "$VERSION")"
CHANGELOG_VERSION="$VERSION" # changelog entries use bare version (no 'v')
RELEASE_DATE="$(date +%Y-%m-%d)"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CURRENT_BRANCH="$(git branch --show-current)"
if [[ "$CURRENT_BRANCH" != "master" && "$CURRENT_BRANCH" != "main" ]]; then
  log_error "You are on branch '$CURRENT_BRANCH', not master/main. Aborting."
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  log_error "You have uncommitted changes. Aborting."
  git status --short
  exit 1
fi

if grep -q "^## \\[$CHANGELOG_VERSION\\]" CHANGELOG.md; then
  log_error "CHANGELOG already contains version $CHANGELOG_VERSION. Aborting."
  exit 1
fi

log_info "Releasing xin $VERSION (tag: $VERSION_TAG)"

# 1) Bump Cargo.toml version
log_info "Bumping Cargo.toml version -> $VERSION"
perl -i.bak -pe 'BEGIN{$in=0} if(/^\[package\]/){$in=1} elsif(/^\[/){$in=0} if($in && /^version = "/){s/^version = "[^"]+"/version = "'"$VERSION"'"/}' Cargo.toml
rm -f Cargo.toml.bak

# 2) Update CHANGELOG.md by moving Unreleased body into the release section
log_info "Updating CHANGELOG.md (move [Unreleased] -> [$CHANGELOG_VERSION] - $RELEASE_DATE)"
if grep -q "^## \\[Unreleased\\]" CHANGELOG.md; then
  perl -0777 -i.bak -pe 'my $v="'"$CHANGELOG_VERSION"'"; my $d="'"$RELEASE_DATE"'"; s/^## \[Unreleased\]\n(.*?)(?=\n## \[|\z)/"## [Unreleased]\n\n## [$v] - $d\n\n$1"/sme;' CHANGELOG.md
  rm -f CHANGELOG.md.bak
else
  log_error "No [Unreleased] section found in CHANGELOG.md. Aborting."
  exit 1
fi

# 3) Regenerate docs (this is what CI checks)
log_info "Regenerating generated docs (skills/xin/*)"
command -v deno >/dev/null 2>&1 || { log_error "deno not found (required for docs generation)"; exit 1; }

cargo build -q

deno run --allow-run --allow-read --allow-write skills/xin/scripts/generate-docs.ts

# Ensure docs are committed (matches CI behavior)
if ! git diff --exit-code >/dev/null; then
  log_info "Docs/version/changelog updated; changes detected (expected)."
fi

# 4) Run tests (serialized to reduce resource flakes)
log_info "Running tests (CARGO_BUILD_JOBS=1, RUST_TEST_THREADS=1)"
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --jobs 1

# 5) Commit
log_info "Creating release commit"
git add -A
git commit -m "release: $VERSION_TAG"

# 6) Tag
log_info "Tagging $VERSION_TAG"
git tag -a "$VERSION_TAG" -m "xin $VERSION"

# 7) Push
log_info "Pushing branch + tag"
git push origin "$CURRENT_BRANCH"
git push origin "$VERSION_TAG"

# 8) Create GitHub Release page (so we don't forget)
if command -v gh >/dev/null 2>&1; then
  log_info "Creating GitHub Release page for $VERSION_TAG"

  REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)"
  if [[ -z "${REPO:-}" ]]; then
    ORIGIN_URL="$(git remote get-url origin)"
    REPO="$(echo "$ORIGIN_URL" | sed -E 's#(git@github.com:|https://github.com/)##; s#\\.git$##')"
  fi

  if [[ -z "${REPO:-}" ]]; then
    log_warn "Could not determine GitHub repo; skipped creating release page."
  elif gh release view "$VERSION_TAG" --repo "$REPO" >/dev/null 2>&1; then
    log_warn "GitHub release $VERSION_TAG already exists; skipping."
  else
    NOTES_FILE="$(mktemp)"

    # Extract this version's section from CHANGELOG.md
    perl -0777 -ne 'if(/## \\['"$CHANGELOG_VERSION"'\\] - [^\\n]*\\n(.*?)(?=\\n## \\[|\\z)/s){print $1}' \
      CHANGELOG.md > "$NOTES_FILE"

    if [[ ! -s "$NOTES_FILE" ]]; then
      cat <<EOF > "$NOTES_FILE"
See CHANGELOG.md for details.
EOF
    fi

    gh release create "$VERSION_TAG" \
      --repo "$REPO" \
      --title "xin $VERSION_TAG" \
      --notes-file "$NOTES_FILE"

    rm -f "$NOTES_FILE"
  fi
else
  log_warn "gh CLI not found; skipped creating GitHub release."
fi

log_info "Done. Monitor: https://github.com/onevcat/xin/actions"
