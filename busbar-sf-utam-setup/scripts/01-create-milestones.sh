#!/bin/bash
# UTAM Rust Project - GitHub Milestones Setup
# Run: ./01-create-milestones.sh

set -e

REPO="composable-delivery/busbar-sf-utam"

echo "🎯 Creating milestones for $REPO..."

# Calculate dates (adjust as needed)
TODAY=$(date +%Y-%m-%d)
PHASE1_DUE=$(date -d "+4 weeks" +%Y-%m-%d 2>/dev/null || date -v+4w +%Y-%m-%d)
PHASE2_DUE=$(date -d "+8 weeks" +%Y-%m-%d 2>/dev/null || date -v+8w +%Y-%m-%d)
PHASE3_DUE=$(date -d "+10 weeks" +%Y-%m-%d 2>/dev/null || date -v+10w +%Y-%m-%d)
PHASE4_DUE=$(date -d "+12 weeks" +%Y-%m-%d 2>/dev/null || date -v+12w +%Y-%m-%d)
V1_DUE=$(date -d "+14 weeks" +%Y-%m-%d 2>/dev/null || date -v+14w +%Y-%m-%d)

# Phase 0: Project Setup (1 week)
gh api repos/$REPO/milestones -f title="v0.0.1 - Project Bootstrap" \
  -f description="Initial project setup, repository structure, CI/CD pipeline, and development environment configuration.

### Goals
- [ ] Cargo workspace structure with utam-core, utam-compiler, utam-cli crates
- [ ] CI/CD with GitHub Actions (test, lint, format, coverage)
- [ ] Development documentation and contribution guidelines
- [ ] Copilot workspace configuration and custom instructions
- [ ] Initial test infrastructure" \
  -f due_on="${TODAY}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v0.0.1 - Project Bootstrap"

# Phase 1: Core Runtime (4 weeks)
gh api repos/$REPO/milestones -f title="v0.1.0 - Core Runtime" \
  -f description="Implement the utam-core runtime library with all fundamental traits and types.

### Deliverables
- [ ] Base element wrapper types (ActionableElement, ClickableElement, EditableElement, DraggableElement)
- [ ] Action trait implementations with thirtyfour WebDriver integration
- [ ] PageObject and RootPageObject traits
- [ ] Shadow DOM support (ShadowRoot wrapper)
- [ ] Container element support with generics
- [ ] Frame element support and context switching
- [ ] Wait utilities, predicates, and timeout configuration
- [ ] Comprehensive error types (UtamError, UtamResult)

### Success Criteria
- All traits compile and have basic test coverage
- Can manually construct and use page objects
- Shadow DOM traversal works correctly
- Frame switching works correctly" \
  -f due_on="${PHASE1_DUE}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v0.1.0 - Core Runtime"

# Phase 2: Compiler (4 weeks)
gh api repos/$REPO/milestones -f title="v0.2.0 - Compiler" \
  -f description="Implement the utam-compiler for transforming JSON page objects to Rust code.

### Deliverables
- [ ] JSON schema validation using jsonschema crate
- [ ] Complete AST data structures mirroring UTAM grammar
- [ ] Element parsing (basic, custom, container, frame types)
- [ ] Selector parsing (CSS with parameters, mobile selectors)
- [ ] Method/compose statement parsing with chaining support
- [ ] Filter and matcher parsing
- [ ] beforeLoad configuration parsing
- [ ] Rust code generation using quote/proc-macro2
- [ ] Error reporting with source locations and helpful messages

### Success Criteria
- Can parse all example UTAM JSON files from salesforce/utam-java-recipes
- Generated Rust code compiles successfully
- Code generation is deterministic
- Errors include file/line information" \
  -f due_on="${PHASE2_DUE}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v0.2.0 - Compiler"

# Phase 3: Tooling (2 weeks)
gh api repos/$REPO/milestones -f title="v0.3.0 - CLI & Tooling" \
  -f description="Implement the utam-cli command-line tool and developer tooling.

### Deliverables
- [ ] CLI tool with clap (compile, validate, init, lint commands)
- [ ] Config file support (utam.config.json parsing)
- [ ] SARIF linting output for IDE integration
- [ ] Watch mode for development workflow
- [ ] VS Code extension recommendations
- [ ] Copilot workspace prompts for common tasks

### Success Criteria
- \`utam compile\` works on a directory of .utam.json files
- \`utam validate\` provides helpful error messages
- \`utam init\` creates a valid config file
- SARIF output integrates with GitHub Code Scanning" \
  -f due_on="${PHASE3_DUE}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v0.3.0 - CLI & Tooling"

# Phase 4: Integration (2 weeks)
gh api repos/$REPO/milestones -f title="v0.4.0 - Integration" \
  -f description="Integration testing, Salesforce compatibility, and documentation.

### Deliverables
- [ ] Test harness utilities for page object testing
- [ ] Assertion helpers and custom matchers
- [ ] Salesforce page objects compatibility verification
- [ ] Documentation generation from JSON
- [ ] Example project with real-world usage
- [ ] Performance benchmarks

### Success Criteria
- Can compile Salesforce page objects from utam-java
- Example tests pass against a real browser
- Documentation is generated and published
- Benchmarks show acceptable performance" \
  -f due_on="${PHASE4_DUE}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v0.4.0 - Integration"

# v1.0.0 Release
gh api repos/$REPO/milestones -f title="v1.0.0 - Initial Release" \
  -f description="First stable release of UTAM Rust implementation.

### Release Checklist
- [ ] All Phase 1-4 milestones complete
- [ ] API stability review
- [ ] Security audit
- [ ] crates.io publishing preparation
- [ ] Release notes and changelog
- [ ] Announcement blog post draft

### Quality Gates
- Test coverage > 80%
- No critical or high-priority bugs
- Documentation complete
- Examples tested on CI" \
  -f due_on="${V1_DUE}T00:00:00Z" \
  -f state="open"

echo "✅ Created: v1.0.0 - Initial Release"

echo ""
echo "🎯 All milestones created! View at:"
echo "   https://github.com/$REPO/milestones"
