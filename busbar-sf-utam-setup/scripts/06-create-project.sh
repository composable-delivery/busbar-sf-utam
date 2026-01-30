#!/bin/bash
# UTAM Rust Project - GitHub Projects Board Setup
# Run: ./06-create-project.sh

set -euo pipefail

REPO="composable-delivery/busbar-sf-utam"
ORG="composable-delivery"
PROJECT_TITLE="UTAM Rust Implementation"

if [[ -n "${1:-}" ]]; then
  PROJECT_NUMBER="$1"
  echo "📊 Using existing GitHub Project #$PROJECT_NUMBER for $REPO..."
else
  echo "📊 Creating GitHub Project for $REPO..."

  # Reuse existing project if it already exists
  EXISTING_NUMBER=$(gh project list --owner "$ORG" --format json | jq -r ".projects[] | select(.title == \"$PROJECT_TITLE\") | .number" | head -n 1)

  if [[ -n "$EXISTING_NUMBER" && "$EXISTING_NUMBER" != "null" ]]; then
    PROJECT_NUMBER="$EXISTING_NUMBER"
    echo "✅ Found existing project: $PROJECT_TITLE (#$PROJECT_NUMBER)"
  else
    PROJECT_JSON=$(gh project create --owner "$ORG" --title "$PROJECT_TITLE" --format json)
    PROJECT_ID=$(echo "$PROJECT_JSON" | jq -r '.id')
    PROJECT_NUMBER=$(echo "$PROJECT_JSON" | jq -r '.number')

    echo "✅ Created project: $PROJECT_ID"
  fi
fi

echo "📋 Project number: $PROJECT_NUMBER"

# Add custom fields
echo "Adding custom fields..."

create_field() {
  local name="$1"
  shift
  if gh project field-create "$PROJECT_NUMBER" --owner "$ORG" --name "$name" "$@" 2>/dev/null; then
    echo "  ✅ Created field: $name"
  else
    echo "  ⚠️  Field already exists or cannot be created: $name"
  fi
}

# Priority field (single select)
create_field "Priority" \
  --data-type "SINGLE_SELECT" \
  --single-select-options "🔴 Critical,🟠 High,🟡 Medium,🟢 Low"

# Size field (single select)
create_field "Size" \
  --data-type "SINGLE_SELECT" \
  --single-select-options "XS (< 1h),S (1-4h),M (4-8h),L (1-2d),XL (3-5d)"

# Component field (single select)
create_field "Component" \
  --data-type "SINGLE_SELECT" \
  --single-select-options "utam-core,utam-compiler,utam-cli,integration"

# Sprint field (number)
create_field "Sprint" \
  --data-type "NUMBER"

echo "✅ Custom fields created"

# Create views
echo "Creating project views..."

# Board view (default)
# Note: Views are typically created via the UI, but we can set up the structure

# Link issues to project
echo "Linking issues to project..."

# Get all issues and add them to the project
gh issue list --repo "$REPO" --json number --jq '.[].number' | while read -r issue_num; do
  gh project item-add "$PROJECT_NUMBER" --owner "$ORG" --url "https://github.com/$REPO/issues/$issue_num" 2>/dev/null || true
  echo "  Added issue #$issue_num"
done

echo ""
echo "📊 Project setup complete!"
echo "   View at: https://github.com/orgs/$ORG/projects/$PROJECT_NUMBER"
echo ""
echo "📝 Manual setup required:"
echo "   1. Create Board view with columns: Backlog, Ready, In Progress, Review, Done"
echo "   2. Create Roadmap view grouped by Milestone"
echo "   3. Set up automation rules for status changes"
echo "   4. Configure sprint iterations"
