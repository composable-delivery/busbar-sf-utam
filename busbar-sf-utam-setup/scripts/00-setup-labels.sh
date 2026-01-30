#!/bin/bash
# UTAM Rust Project - GitHub Labels Setup
# Run: ./00-setup-labels.sh

set -e

REPO="composable-delivery/busbar-sf-utam"

echo "🏷️  Setting up GitHub labels for $REPO..."

# Delete default labels we won't use
echo "Removing default labels..."
gh label delete "good first issue" --repo "$REPO" --yes 2>/dev/null || true
gh label delete "help wanted" --repo "$REPO" --yes 2>/dev/null || true
gh label delete "invalid" --repo "$REPO" --yes 2>/dev/null || true
gh label delete "question" --repo "$REPO" --yes 2>/dev/null || true
gh label delete "wontfix" --repo "$REPO" --yes 2>/dev/null || true

# Component labels (blue family)
echo "Creating component labels..."
gh label create "component/core" --color "0052CC" --description "utam-core runtime library" --repo "$REPO" --force
gh label create "component/compiler" --color "0066FF" --description "utam-compiler code generator" --repo "$REPO" --force
gh label create "component/cli" --color "0080FF" --description "utam-cli command line tool" --repo "$REPO" --force
gh label create "component/integration" --color "0099FF" --description "Integration and testing utilities" --repo "$REPO" --force

# Type labels (green/yellow family)
echo "Creating type labels..."
gh label create "type/feature" --color "1D7A43" --description "New feature or capability" --repo "$REPO" --force
gh label create "type/enhancement" --color "2EA44F" --description "Improvement to existing feature" --repo "$REPO" --force
gh label create "type/bug" --color "D73A4A" --description "Something isn't working" --repo "$REPO" --force
gh label create "type/docs" --color "0075CA" --description "Documentation improvements" --repo "$REPO" --force
gh label create "type/chore" --color "7057FF" --description "Maintenance and tooling" --repo "$REPO" --force
gh label create "type/research" --color "D4C5F9" --description "Investigation or spike" --repo "$REPO" --force

# Priority labels (warm colors)
echo "Creating priority labels..."
gh label create "priority/critical" --color "B60205" --description "Blocking - needs immediate attention" --repo "$REPO" --force
gh label create "priority/high" --color "D93F0B" --description "Important - address soon" --repo "$REPO" --force
gh label create "priority/medium" --color "FBCA04" --description "Normal priority" --repo "$REPO" --force
gh label create "priority/low" --color "C2E0C6" --description "Nice to have" --repo "$REPO" --force

# Status labels (process)
echo "Creating status labels..."
gh label create "status/needs-design" --color "E4E669" --description "Needs architecture/design discussion" --repo "$REPO" --force
gh label create "status/ready" --color "0E8A16" --description "Ready for implementation" --repo "$REPO" --force
gh label create "status/blocked" --color "B60205" --description "Blocked by external dependency" --repo "$REPO" --force
gh label create "status/in-review" --color "6F42C1" --description "In code review" --repo "$REPO" --force

# Copilot-optimized labels
echo "Creating Copilot-optimized labels..."
gh label create "copilot/good-prompt" --color "238636" --description "Well-defined for AI assistance" --repo "$REPO" --force
gh label create "copilot/needs-context" --color "FFA657" --description "Needs more context for AI" --repo "$REPO" --force
gh label create "copilot/generated" --color "A371F7" --description "Code generated with AI assistance" --repo "$REPO" --force

# Size labels (for estimation)
echo "Creating size labels..."
gh label create "size/XS" --color "C2E0C6" --description "< 1 hour" --repo "$REPO" --force
gh label create "size/S" --color "5DC07F" --description "1-4 hours" --repo "$REPO" --force
gh label create "size/M" --color "FBCA04" --description "4-8 hours" --repo "$REPO" --force
gh label create "size/L" --color "F9A825" --description "1-2 days" --repo "$REPO" --force
gh label create "size/XL" --color "D93F0B" --description "3-5 days" --repo "$REPO" --force

echo "✅ Labels setup complete!"
