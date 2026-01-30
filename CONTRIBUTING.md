# Contributing to UTAM Rust

Thank you for your interest in contributing to UTAM Rust! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up the development environment (see below)
4. Create a feature branch
5. Make your changes
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75 or later (MSRV)
- Cargo
- Git

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/busbar-sf-utam.git
cd busbar-sf-utam

# Build all crates
cargo build --all-targets

# Run tests
cargo test --all-features

# Run lints
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### IDE Setup

We recommend using an IDE with Rust Language Server (rust-analyzer) support:

- **VS Code**: Install the `rust-analyzer` extension
- **IntelliJ IDEA**: Install the Rust plugin
- **Vim/Neovim**: Configure with `coc-rust-analyzer` or native LSP

## Project Structure

```
busbar-sf-utam/
├── utam-core/          # Runtime library
│   ├── src/
│   │   ├── elements.rs  # Element wrappers
│   │   ├── traits.rs    # Actionable, Clickable, etc.
│   │   ├── error.rs     # Error types
│   │   ├── shadow.rs    # Shadow DOM support
│   │   └── wait.rs      # Wait utilities
│   └── Cargo.toml
├── utam-compiler/      # JSON to Rust compiler
│   ├── src/
│   │   ├── ast.rs       # AST definitions
│   │   ├── parser.rs    # JSON parsing
│   │   ├── validator.rs # Semantic validation
│   │   ├── codegen.rs   # Code generation
│   │   └── error.rs     # Compiler errors
│   └── Cargo.toml
├── utam-cli/           # Command-line interface
│   ├── src/
│   │   └── main.rs
│   └── Cargo.toml
├── testdata/           # Test fixtures
└── Cargo.toml          # Workspace configuration
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for more details on the system design.

## Development Workflow

### Creating a Feature Branch

```bash
# Update main branch
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name
```

### Making Changes

1. **Write tests first** (TDD approach recommended)
2. Implement your changes
3. Ensure tests pass: `cargo test`
4. Run lints: `cargo clippy --all-targets -- -D warnings`
5. Format code: `cargo fmt`
6. Update documentation if needed

### Commit Messages

Use clear, descriptive commit messages:

```
feat: Add support for custom element types
fix: Resolve shadow DOM selector bug
docs: Update ARCHITECTURE.md with new diagrams
test: Add integration tests for compiler
refactor: Extract validation logic into separate module
```

Follow [Conventional Commits](https://www.conventionalcommits.org/) format:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `test:` - Test additions or modifications
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Build process or auxiliary tool changes

## Testing

### Running Tests

```bash
# Run all tests
cargo test --all-features

# Run tests for a specific crate
cargo test -p utam-core
cargo test -p utam-compiler
cargo test -p utam-cli

# Run with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Writing Tests

#### Unit Tests

Place unit tests in a `tests` module within the source file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_parsing() {
        let selector = Selector::parse(".button").unwrap();
        assert_eq!(selector.css, Some(".button".to_string()));
    }
}
```

#### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/compile_page_object.rs
use utam_compiler::compile;

#[test]
fn compile_basic_page_object() {
    let input = include_str!("testdata/basic.utam.json");
    let result = compile(input);
    assert!(result.is_ok());
}
```

#### Snapshot Tests

Use `insta` for snapshot testing generated code:

```rust
#[test]
fn test_code_generation() {
    let output = generate_code(&ast);
    insta::assert_snapshot!(output);
}
```

Update snapshots with: `cargo insta review`

### Coverage

Generate coverage reports:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --lcov --output-path lcov.info
cargo llvm-cov --all-features --html
```

## Code Style

### Rust Style Guide

Follow the [Rust Style Guide](https://rust-lang.github.io/api-guidelines/):

- Use `snake_case` for functions, methods, variables
- Use `PascalCase` for types, traits, enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Maximum line length: 100 characters
- Use descriptive names over abbreviations

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt
```

The CI will fail if code is not properly formatted.

### Linting

Code must pass Clippy without warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

Fix common issues automatically:

```bash
cargo clippy --fix
```

### Documentation

- Add doc comments to all public APIs
- Include examples in doc comments
- Document error conditions
- Keep documentation up to date with code changes

Example:

```rust
/// Clicks the element and waits for it to become stable.
///
/// # Arguments
///
/// * `timeout` - Maximum time to wait for the element
///
/// # Errors
///
/// Returns `UtamError::ElementNotFound` if the element doesn't exist.
///
/// # Examples
///
/// ```rust
/// let button = page.get_submit_button().await?;
/// button.click().await?;
/// ```
pub async fn click(&self) -> UtamResult<()> {
    // Implementation
}
```

## Pull Request Process

### Before Submitting

1. Ensure all tests pass: `cargo test --all-features`
2. Run lints: `cargo clippy --all-targets -- -D warnings`
3. Format code: `cargo fmt`
4. Update documentation
5. Add/update tests for your changes
6. Rebase on latest main branch

### Submitting a PR

1. Push your branch to your fork
2. Open a pull request against `main`
3. Fill out the PR template
4. Link any related issues
5. Request review from maintainers

### PR Title Format

Use conventional commit format:

```
feat: Add support for custom selectors
fix: Resolve memory leak in element cache
docs: Update installation instructions
```

### PR Description

Include:

- **Summary**: What does this PR do?
- **Motivation**: Why is this change needed?
- **Changes**: List of key changes
- **Testing**: How was this tested?
- **Breaking Changes**: Any breaking changes?
- **Related Issues**: Links to related issues

### Review Process

- At least one maintainer approval required
- All CI checks must pass
- Address review feedback promptly
- Keep PRs focused and reasonably sized

### After Approval

Maintainers will merge your PR using squash merge.

## Release Process

*Note: Only maintainers can perform releases.*

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create release tag: `git tag -a v0.x.x -m "Release v0.x.x"`
5. Push tag: `git push origin v0.x.x`
6. Publish to crates.io:
   ```bash
   cargo publish -p utam-core
   cargo publish -p utam-compiler
   cargo publish -p utam-cli
   ```
7. Create GitHub release with changelog

## Getting Help

- **Questions**: Open a [Discussion](https://github.com/composable-delivery/busbar-sf-utam/discussions)
- **Bug Reports**: Open an [Issue](https://github.com/composable-delivery/busbar-sf-utam/issues)
- **Security Issues**: See [SECURITY.md](SECURITY.md) (if available)

## Recognition

Contributors are recognized in:
- GitHub contributors page
- Release notes
- Project README (for significant contributions)

Thank you for contributing! 🎉
