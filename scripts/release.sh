#!/usr/bin/env bash
#
# xin release script
# Usage: ./scripts/release.sh <version>
#
# This script:
# 1. Updates CHANGELOG.md with the new version
# 2. Creates a git tag and pushes to trigger Homebrew release workflow
# 3. Monitors the release until completion
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

usage() {
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
}

# Check arguments
if [[ $# -ne 1 ]]; then
    usage
fi

VERSION="$1"
VERSION_TAG="$VERSION"

# Validate version format (semver)
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    log_error "Invalid version format. Use semver (e.g., 0.1.0 or 0.1.0-beta.1)"
    exit 1
fi

# Get script directory and repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"

cd "$REPO_ROOT"

# Check for uncommitted changes
if [[ -n "$(git status --porcelain)" ]]; then
    log_warn "You have uncommitted changes:"
    git status --short
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_error "Aborted."
        exit 1
    fi
fi

# Check current branch
CURRENT_BRANCH="$(git branch --show-current)"
if [[ "$CURRENT_BRANCH" != "master" && "$CURRENT_BRANCH" != "main" ]]; then
    log_warn "You are on branch '$CURRENT_BRANCH', not master/main."
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_error "Aborted."
        exit 1
    fi
fi

log_info "Releasing xin $VERSION..."

# Update CHANGELOG.md
log_info "Updating CHANGELOG.md..."

# Read current date in ISO format
RELEASE_DATE=$(date +%Y-%m-%d)

# Check if [Unreleased] section exists
if grep -q "^## \[Unreleased\]" CHANGELOG.md; then
    # Replace [Unreleased] with [VERSION] - DATE
    sed -i.bak "s/^## \[Unreleased\]/## [$VERSION_TAG] - $RELEASE_DATE/" CHANGELOG.md
    rm -f CHANGELOG.md.bak
    log_info "Updated CHANGELOG.md: [Unreleased] -> [$VERSION_TAG] - $RELEASE_DATE"
else
    log_warn "No [Unreleased] section found in CHANGELOG.md"
    log_warn "Please update CHANGELOG.md manually before releasing"
fi

# Commit changes
log_info "Committing changes..."
git add CHANGELOG.md
git commit -m "release: $VERSION_TAG"

# Create and push tag
log_info "Creating tag $VERSION_TAG..."
git tag -a "$VERSION_TAG" -m "xin $VERSION"

log_info "Pushing to origin..."
git push origin "$CURRENT_BRANCH"
git push origin "$VERSION_TAG"

log_info "Tag pushed. Homebrew release workflow should start automatically."
log_info "Monitor at: https://github.com/onevcat/xin/actions"
log_info ""
log_info "After the workflow completes, the Homebrew formula PR will be created at:"
log_info "  https://github.com/onevcat/homebrew-tap/pulls"
log_info ""
log_info "To check status:"
echo "  gh run list --repo onevcat/xin"
echo "  gh run list --repo onevcat/homebrew-tap"
