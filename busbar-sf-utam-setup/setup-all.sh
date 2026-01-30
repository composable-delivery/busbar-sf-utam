#!/bin/bash
# UTAM Rust Project - Complete Setup Script
# This script sets up the entire GitHub project infrastructure
#
# Usage:
#   ./setup-all.sh              # Run all setup steps
#   ./setup-all.sh --scaffold   # Also scaffold the repo structure
#   ./setup-all.sh --dry-run    # Show what would be done
#
# Prerequisites:
#   - GitHub CLI (gh) installed and authenticated
#   - Access to composable-delivery/busbar-sf-utam repository

set -e

REPO="composable-delivery/busbar-sf-utam"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRY_RUN=false
SCAFFOLD=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --scaffold)
      SCAFFOLD=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

# Check prerequisites
check_prerequisites() {
  log_info "Checking prerequisites..."

  if ! command -v gh &> /dev/null; then
    log_error "GitHub CLI (gh) is not installed"
    echo "Install it from: https://cli.github.com/"
    exit 1
  fi

  if ! gh auth status &> /dev/null; then
    log_error "GitHub CLI is not authenticated"
    echo "Run: gh auth login"
    exit 1
  fi

  # Check repo access
  if ! gh repo view "$REPO" &> /dev/null; then
    log_error "Cannot access repository: $REPO"
    echo "Make sure you have access to the repository"
    exit 1
  fi

  log_success "Prerequisites check passed"
}

# Run a setup script
run_script() {
  local script="$1"
  local name="$2"

  log_info "Running: $name"

  if [ "$DRY_RUN" = true ]; then
    log_warning "DRY RUN: Would execute $script"
    return
  fi

  if [ -f "$SCRIPT_DIR/scripts/$script" ]; then
    chmod +x "$SCRIPT_DIR/scripts/$script"
    "$SCRIPT_DIR/scripts/$script"
    log_success "Completed: $name"
  else
    log_error "Script not found: $script"
    exit 1
  fi
}

# Main setup flow
main() {
  echo ""
  echo "╔═══════════════════════════════════════════════════════════════╗"
  echo "║          UTAM Rust Project - GitHub Setup                     ║"
  echo "║          Repository: $REPO                    ║"
  echo "╚═══════════════════════════════════════════════════════════════╝"
  echo ""

  check_prerequisites

  echo ""
  log_info "Starting setup process..."
  echo ""

  # Step 1: Labels
  run_script "00-setup-labels.sh" "Setting up labels"
  echo ""

  # Step 2: Milestones
  run_script "01-create-milestones.sh" "Creating milestones"
  echo ""

  # Step 3: Phase 0 Issues (Bootstrap)
  run_script "02-create-issues-phase0.sh" "Creating Phase 0 issues"
  echo ""

  # Step 4: Phase 1 Issues (Core Runtime)
  run_script "03-create-issues-phase1.sh" "Creating Phase 1 issues"
  echo ""

  # Step 5: Phase 2 Issues (Compiler)
  run_script "04-create-issues-phase2.sh" "Creating Phase 2 issues"
  echo ""

  # Step 6: Phase 3 & 4 Issues (CLI, Integration)
  run_script "05-create-issues-phase3-4.sh" "Creating Phase 3 & 4 issues"
  echo ""

  # Step 7: GitHub Project
  run_script "06-create-project.sh" "Creating GitHub Project board"
  echo ""

  # Optional: Scaffold repository
  if [ "$SCAFFOLD" = true ]; then
    log_info "Scaffolding repository structure..."
    if [ "$DRY_RUN" = true ]; then
      log_warning "DRY RUN: Would scaffold repository"
    else
      # Clone repo, scaffold, commit, push
      TEMP_DIR=$(mktemp -d)
      git clone "https://github.com/$REPO.git" "$TEMP_DIR/repo"
      "$SCRIPT_DIR/scripts/07-scaffold-repo.sh" "$TEMP_DIR/repo"

      cd "$TEMP_DIR/repo"
      git add .
      git commit -m "chore: scaffold UTAM Rust project structure

- Initialize Cargo workspace with utam-core, utam-compiler, utam-cli
- Add GitHub Actions CI workflow
- Create initial module structure
- Add test data fixtures

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
      git push

      cd -
      rm -rf "$TEMP_DIR"
      log_success "Repository scaffolded and pushed"
    fi
    echo ""
  fi

  # Copy Copilot instructions to repo
  log_info "Copilot instructions should be copied to the repo:"
  echo "  cp copilot/.github/copilot-instructions.md <repo>/.github/"
  echo "  cp -r copilot/agents <repo>/.github/copilot/agents/"
  echo ""

  # Summary
  echo ""
  echo "╔═══════════════════════════════════════════════════════════════╗"
  echo "║                    Setup Complete! 🎉                          ║"
  echo "╚═══════════════════════════════════════════════════════════════╝"
  echo ""
  echo "Resources created:"
  echo "  📋 Milestones:  https://github.com/$REPO/milestones"
  echo "  🎫 Issues:      https://github.com/$REPO/issues"
  echo "  🏷️  Labels:      https://github.com/$REPO/labels"
  echo "  📊 Project:     https://github.com/orgs/composable-delivery/projects"
  echo ""
  echo "Next steps:"
  echo "  1. Review and adjust milestone due dates"
  echo "  2. Configure Project board views (Roadmap, Board)"
  echo "  3. Assign issues to team members"
  echo "  4. Copy .github/copilot-instructions.md to the repository"
  echo "  5. Start working on Phase 0 issues!"
  echo ""
}

main
