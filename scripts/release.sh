#!/usr/bin/env bash
#
# Prax Release Script
#
# Prepares a new release by updating versions, CHANGELOG, and creating git tags.
#
# Usage:
#   ./scripts/release.sh <VERSION> [OPTIONS]
#
# Options:
#   --no-tag        Skip creating git tag
#   --no-commit     Skip creating commit
#   --no-push       Skip pushing to remote
#   --help          Show this help message
#
# Examples:
#   ./scripts/release.sh 0.2.0
#   ./scripts/release.sh 0.2.0 --no-push

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Options
NO_TAG=false
NO_COMMIT=false
NO_PUSH=false
VERSION=""

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
Prax Release Script

Prepares a new release by updating versions, CHANGELOG, and creating git tags.

Usage:
    ./scripts/release.sh <VERSION> [OPTIONS]

Arguments:
    VERSION         Semantic version (e.g., 0.2.0, 1.0.0-beta.1)

Options:
    --no-tag        Skip creating git tag
    --no-commit     Skip creating commit
    --no-push       Skip pushing to remote
    --help          Show this help message

Examples:
    ./scripts/release.sh 0.2.0
    ./scripts/release.sh 0.2.0 --no-push
    ./scripts/release.sh 1.0.0-rc.1 --no-tag
EOF
}

parse_args() {
    if [[ $# -lt 1 ]]; then
        log_error "Version argument required"
        show_help
        exit 1
    fi

    while [[ $# -gt 0 ]]; do
        case $1 in
            --no-tag)
                NO_TAG=true
                shift
                ;;
            --no-commit)
                NO_COMMIT=true
                shift
                ;;
            --no-push)
                NO_PUSH=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
            *)
                if [[ -z "$VERSION" ]]; then
                    VERSION="$1"
                else
                    log_error "Unexpected argument: $1"
                    show_help
                    exit 1
                fi
                shift
                ;;
        esac
    done

    if [[ -z "$VERSION" ]]; then
        log_error "Version argument required"
        show_help
        exit 1
    fi

    # Validate version format
    if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
        log_error "Invalid version format: $VERSION"
        log_error "Expected format: MAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH-PRERELEASE"
        exit 1
    fi
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check we're on develop or release branch
    local branch
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [[ "$branch" != "develop" && ! "$branch" =~ ^release/ ]]; then
        log_warn "Not on develop or release branch (current: $branch)"
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    # Check for uncommitted changes
    if ! git diff --quiet HEAD; then
        log_error "Working directory has uncommitted changes"
        exit 1
    fi

    # Run tests
    log_info "Running tests..."
    if ! cargo test --workspace --lib; then
        log_error "Tests failed"
        exit 1
    fi

    log_success "Prerequisites check passed"
}

get_current_version() {
    grep -m1 '^version = "' Cargo.toml | sed 's/version = "\(.*\)"/\1/'
}

update_versions() {
    local old_version
    old_version=$(get_current_version)

    log_info "Updating version from $old_version to $VERSION..."

    # Update workspace version
    sed -i "s/^version = \"$old_version\"/version = \"$VERSION\"/" Cargo.toml

    # Update workspace.dependencies version references
    sed -i "s/version = \"$old_version\"/version = \"$VERSION\"/g" Cargo.toml

    # Update individual crate Cargo.toml files (non-workspace versions)
    for dir in prax-*/; do
        if [[ -f "$dir/Cargo.toml" ]]; then
            # Only update if it has its own version line (not workspace = true)
            if grep -q "^version = \"" "$dir/Cargo.toml"; then
                sed -i "s/^version = \"$old_version\"/version = \"$VERSION\"/" "$dir/Cargo.toml"
            fi
        fi
    done

    log_success "Versions updated"
}

update_changelog() {
    log_info "Updating CHANGELOG.md..."

    local today
    today=$(date +%Y-%m-%d)

    # Check if CHANGELOG has Unreleased section
    if grep -q "## \[Unreleased\]" CHANGELOG.md; then
        # Replace [Unreleased] with version and date, add new Unreleased section
        sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [$VERSION] - $today/" CHANGELOG.md
        log_success "CHANGELOG.md updated"
    else
        log_warn "No [Unreleased] section found in CHANGELOG.md - skipping"
    fi
}

create_commit() {
    if [[ "$NO_COMMIT" == "true" ]]; then
        log_warn "Skipping commit (--no-commit)"
        return
    fi

    log_info "Creating release commit..."

    git add -A
    git commit -m "chore(release): prepare v$VERSION"

    log_success "Commit created"
}

create_tag() {
    if [[ "$NO_TAG" == "true" ]]; then
        log_warn "Skipping tag (--no-tag)"
        return
    fi

    log_info "Creating git tag v$VERSION..."

    git tag -a "v$VERSION" -m "Release v$VERSION"

    log_success "Tag v$VERSION created"
}

push_changes() {
    if [[ "$NO_PUSH" == "true" ]]; then
        log_warn "Skipping push (--no-push)"
        return
    fi

    log_info "Pushing to remote..."

    git push
    if [[ "$NO_TAG" == "false" ]]; then
        git push --tags
    fi

    log_success "Pushed to remote"
}

main() {
    parse_args "$@"

    echo ""
    echo "================================================"
    echo "  Prax Release Script - v$VERSION"
    echo "================================================"
    echo ""

    cd "$(dirname "$0")/.."

    check_prerequisites
    update_versions
    update_changelog
    create_commit
    create_tag
    push_changes

    echo ""
    log_success "================================================"
    log_success "  Release v$VERSION prepared!"
    log_success "================================================"
    echo ""

    echo "Next steps:"
    echo "  1. Create PR from release branch to main"
    echo "  2. After merge, run: ./scripts/publish.sh"
    echo "  3. Create GitHub release at https://github.com/pegasusheavy/prax/releases/new"
    echo ""
}

main "$@"

