#!/bin/bash
# UTAM Rust Project - Phase 0 Issues (Project Bootstrap)
# Run: ./02-create-issues-phase0.sh

set -e

REPO="composable-delivery/busbar-sf-utam"
MILESTONE="v0.0.1 - Project Bootstrap"

echo "📋 Creating Phase 0 issues for $REPO..."

# Issue 1: Cargo workspace setup
gh issue create --repo "$REPO" \
  --title "Initialize Cargo workspace with crate structure" \
  --milestone "$MILESTONE" \
  --label "component/core,component/compiler,component/cli,type/chore,priority/critical,size/M,copilot/good-prompt" \
  --body "## Summary
Create the initial Cargo workspace structure with three crates.

## Acceptance Criteria
- [ ] \`Cargo.toml\` at root with workspace configuration
- [ ] \`utam-core/\` crate with basic lib.rs
- [ ] \`utam-compiler/\` crate with basic lib.rs
- [ ] \`utam-cli/\` crate with basic main.rs
- [ ] Shared dependencies configured at workspace level
- [ ] \`cargo build\` succeeds
- [ ] \`cargo test\` runs (even with no tests)

## Technical Details
\`\`\`toml
# Cargo.toml (workspace root)
[workspace]
resolver = \"2\"
members = [\"utam-core\", \"utam-compiler\", \"utam-cli\"]

[workspace.package]
version = \"0.1.0\"
edition = \"2021\"
rust-version = \"1.75\"
license = \"MIT OR Apache-2.0\"
repository = \"https://github.com/composable-delivery/busbar-sf-utam\"

[workspace.dependencies]
thirtyfour = \"0.32\"
tokio = { version = \"1\", features = [\"full\"] }
serde = { version = \"1\", features = [\"derive\"] }
serde_json = \"1\"
thiserror = \"1\"
async-trait = \"0.1\"
\`\`\`

## Copilot Prompt
\`\`\`
Create a Rust workspace with three crates: utam-core (library), utam-compiler (library),
and utam-cli (binary). Use workspace-level dependency management. Include thirtyfour
for WebDriver, tokio for async, and serde for JSON handling.
\`\`\`"

echo "✅ Created: Initialize Cargo workspace"

# Issue 2: CI/CD Pipeline
gh issue create --repo "$REPO" \
  --title "Set up GitHub Actions CI/CD pipeline" \
  --milestone "$MILESTONE" \
  --label "type/chore,priority/critical,size/M,copilot/good-prompt" \
  --body "## Summary
Configure GitHub Actions for continuous integration and deployment.

## Acceptance Criteria
- [ ] \`.github/workflows/ci.yml\` with test, lint, format checks
- [ ] Matrix testing on stable/beta/nightly Rust
- [ ] Caching for faster builds
- [ ] Coverage reporting with codecov
- [ ] Release workflow for publishing to crates.io
- [ ] Dependabot configuration for dependency updates

## Workflow Structure
\`\`\`yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta, nightly]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: \${{ matrix.rust }}
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --all-features --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
\`\`\`

## Copilot Prompt
\`\`\`
Create GitHub Actions workflows for a Rust project: CI workflow with matrix testing
(stable, beta, nightly), clippy linting, rustfmt checking, and codecov coverage.
Include Swatinem/rust-cache for faster builds. Add a release workflow that publishes
to crates.io on tag push.
\`\`\`"

echo "✅ Created: GitHub Actions CI/CD"

# Issue 3: Development documentation
gh issue create --repo "$REPO" \
  --title "Create development documentation and contribution guidelines" \
  --milestone "$MILESTONE" \
  --label "type/docs,priority/high,size/S,copilot/good-prompt" \
  --body "## Summary
Create essential documentation for contributors.

## Acceptance Criteria
- [ ] \`README.md\` with project overview, badges, quick start
- [ ] \`CONTRIBUTING.md\` with development setup, PR process
- [ ] \`CODE_OF_CONDUCT.md\` using Contributor Covenant
- [ ] \`LICENSE\` files (MIT and Apache-2.0)
- [ ] \`ARCHITECTURE.md\` documenting crate structure and design decisions

## README Structure
\`\`\`markdown
# UTAM Rust

Rust implementation of [UTAM](https://utam.dev) - UI Test Automation Model.

[![CI](https://github.com/composable-delivery/busbar-sf-utam/workflows/CI/badge.svg)]
[![codecov](https://codecov.io/gh/composable-delivery/busbar-sf-utam/branch/main/graph/badge.svg)]
[![Crates.io](https://img.shields.io/crates/v/utam-core.svg)]

## Quick Start
...

## Crates
- **utam-core**: Runtime library with WebDriver traits
- **utam-compiler**: JSON to Rust code generator
- **utam-cli**: Command-line interface

## License
Dual-licensed under MIT or Apache-2.0.
\`\`\`

## Copilot Prompt
\`\`\`
Create README.md for a Rust UTAM implementation with: project badges (CI, coverage,
crates.io), feature overview, installation instructions, quick start example, and
links to documentation. Include a crate overview table.
\`\`\`"

echo "✅ Created: Development documentation"

# Issue 4: Copilot configuration
gh issue create --repo "$REPO" \
  --title "Configure GitHub Copilot workspace and custom instructions" \
  --milestone "$MILESTONE" \
  --label "type/chore,priority/high,size/S,copilot/good-prompt" \
  --body "## Summary
Set up GitHub Copilot for optimal AI-assisted development on this project.

## Acceptance Criteria
- [ ] \`.github/copilot-instructions.md\` with project context
- [ ] \`.vscode/settings.json\` with Copilot preferences
- [ ] Custom agent definition for UTAM-specific tasks
- [ ] Prompt templates for common development tasks

## Custom Instructions Content
The instructions should include:
- UTAM JSON grammar overview
- Rust code generation patterns
- Testing conventions
- Error handling patterns (thiserror, UtamResult)
- Async patterns (async-trait, tokio)

## Reference
See the UTAM Rust Implementation Reference document for complete grammar specification.

## Copilot Prompt
\`\`\`
Create .github/copilot-instructions.md for a Rust UTAM compiler project. Include:
project architecture (core/compiler/cli crates), UTAM JSON grammar summary, Rust
idioms to follow (async-trait, thiserror), and testing patterns. Make instructions
specific enough to generate correct UTAM-aware code.
\`\`\`"

echo "✅ Created: Copilot configuration"

# Issue 5: Test infrastructure
gh issue create --repo "$REPO" \
  --title "Set up test infrastructure and fixtures" \
  --milestone "$MILESTONE" \
  --label "type/chore,priority/medium,size/M,copilot/good-prompt" \
  --body "## Summary
Create shared test utilities and example UTAM JSON fixtures.

## Acceptance Criteria
- [ ] \`tests/\` directory with integration test structure
- [ ] \`testdata/\` directory with example .utam.json files
- [ ] Test utilities crate or module for common assertions
- [ ] Mock WebDriver setup for unit testing
- [ ] Snapshot testing configuration (insta crate)

## Test Data Structure
\`\`\`
testdata/
├── basic/
│   ├── simple-element.utam.json
│   ├── clickable-button.utam.json
│   └── editable-input.utam.json
├── shadow-dom/
│   ├── shadow-root.utam.json
│   └── nested-shadow.utam.json
├── compose/
│   ├── simple-method.utam.json
│   ├── chained-method.utam.json
│   └── filter-method.utam.json
├── salesforce/
│   └── (examples from utam-java-recipes)
└── invalid/
    ├── missing-selector.utam.json
    └── invalid-type.utam.json
\`\`\`

## Copilot Prompt
\`\`\`
Create test infrastructure for a UTAM compiler: integration test directory structure,
example .utam.json fixtures covering basic elements, shadow DOM, compose methods,
and error cases. Include insta for snapshot testing of generated Rust code.
\`\`\`"

echo "✅ Created: Test infrastructure"

# Issue 6: Pre-commit hooks
gh issue create --repo "$REPO" \
  --title "Configure pre-commit hooks and development tooling" \
  --milestone "$MILESTONE" \
  --label "type/chore,priority/low,size/S,copilot/good-prompt" \
  --body "## Summary
Set up pre-commit hooks for consistent code quality.

## Acceptance Criteria
- [ ] \`.pre-commit-config.yaml\` with Rust hooks
- [ ] cargo-husky or similar for git hooks
- [ ] Automatic formatting on commit
- [ ] Clippy checks before push
- [ ] Commit message linting (conventional commits)

## Configuration
\`\`\`yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --
        language: system
        types: [rust]
        pass_filenames: false
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --all-targets -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
\`\`\`

## Copilot Prompt
\`\`\`
Create pre-commit configuration for a Rust project: cargo fmt, cargo clippy with
warnings as errors, and conventional commit message validation. Include setup
instructions in CONTRIBUTING.md.
\`\`\`"

echo "✅ Created: Pre-commit hooks"

echo ""
echo "📋 Phase 0 issues created! View at:"
echo "   https://github.com/$REPO/issues?milestone=v0.0.1+-+Project+Bootstrap"
