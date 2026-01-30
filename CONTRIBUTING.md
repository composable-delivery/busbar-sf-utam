# Contributing to UTAM Rust

Thank you for your interest in contributing to UTAM Rust! This guide will help you get started.

## Table of Contents

- [Development Setup](#development-setup)
- [Pre-commit Hooks](#pre-commit-hooks)
- [Code Quality](#code-quality)
- [Commit Messages](#commit-messages)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)

## Development Setup

### Prerequisites

- Rust 1.75 or later (we recommend using `rustup`)
- Git
- Python 3.7+ (for pre-commit hooks)

### Initial Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/composable-delivery/busbar-sf-utam.git
   cd busbar-sf-utam
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests to ensure everything works:
   ```bash
   cargo test
   ```

## Pre-commit Hooks

We use [pre-commit](https://pre-commit.com/) to ensure code quality and consistency. Pre-commit hooks automatically run checks before commits and pushes.

### Installing Pre-commit

#### Using pip (recommended)

```bash
pip install pre-commit
```

#### Using Homebrew (macOS)

```bash
brew install pre-commit
```

#### Using other package managers

See the [official installation guide](https://pre-commit.com/#install).

### Setting Up Pre-commit Hooks

After installing pre-commit, set up the git hooks:

```bash
# Install the git hooks
pre-commit install

# Install commit-msg hook for conventional commits
pre-commit install --hook-type commit-msg

# (Optional) Run hooks on all files to verify setup
pre-commit run --all-files
```

### What the Hooks Do

Our pre-commit configuration includes:

#### On Every Commit (pre-commit stage)
- **cargo fmt**: Automatically formats Rust code
- **YAML/TOML/JSON validation**: Checks syntax of configuration files
- **File checks**: Prevents large files, merge conflicts, and trailing whitespace

#### On Git Push (pre-push stage)
- **cargo clippy**: Runs linter with warnings as errors
- **cargo check**: Verifies code compiles

#### On Commit Messages (commit-msg stage)
- **Conventional commits**: Validates commit message format

### Bypassing Hooks (Not Recommended)

If you need to bypass hooks (e.g., for work-in-progress commits), use:

```bash
git commit --no-verify -m "wip: work in progress"
```

**Note**: This is discouraged as it may introduce issues that CI will catch later.

## Code Quality

### Formatting

We use `rustfmt` with the configuration in `rustfmt.toml`:

```bash
# Format all code
cargo fmt

# Check formatting without modifying files
cargo fmt --check
```

### Linting

We use `clippy` with warnings treated as errors:

```bash
# Run clippy on all targets
cargo clippy --all-targets --all-features -- -D warnings
```

### Building

```bash
# Build all workspace members
cargo build --all-targets

# Build with all features enabled
cargo build --all-features
```

### Code Style Guidelines

Please follow the guidelines in the [custom instructions](.github/copilot-instructions.md):

- Use `snake_case` for functions and variables
- Use `PascalCase` for types and structs
- Use `SCREAMING_SNAKE_CASE` for constants
- Document public APIs with doc comments (`///`)
- Add `# Examples`, `# Errors`, and `# Panics` sections where appropriate
- Prefer `?` operator for error propagation
- Use `async-trait` for async trait methods

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/) specification:

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- **feat**: New feature
- **fix**: Bug fix
- **docs**: Documentation changes
- **style**: Code style changes (formatting, missing semicolons, etc.)
- **refactor**: Code refactoring without changing functionality
- **perf**: Performance improvements
- **test**: Adding or updating tests
- **build**: Build system or dependency changes
- **ci**: CI/CD configuration changes
- **chore**: Other changes that don't modify src or test files

### Scopes

- `core`: Changes to utam-core
- `compiler`: Changes to utam-compiler
- `cli`: Changes to utam-cli
- `deps`: Dependency updates
- `ci`: CI/CD changes

### Examples

```bash
feat(compiler): add support for frame elements
fix(core): correct shadow DOM element selection
docs(readme): update installation instructions
test(compiler): add tests for validation logic
chore(deps): update thirtyfour to 0.36.2
```

### Breaking Changes

For breaking changes, add `BREAKING CHANGE:` in the footer or append `!` after the type:

```
feat(core)!: remove deprecated ActionElement trait

BREAKING CHANGE: ActionElement trait has been removed in favor of Actionable
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p utam-core

# Run a specific test
cargo test test_parse_selector

# Run tests with output
cargo test -- --nocapture
```

### Writing Tests

- Place unit tests in a `mod tests` block at the bottom of the source file
- Place integration tests in the `tests/` directory
- Use `insta` for snapshot testing of generated code
- Follow existing test patterns in the codebase

### Code Coverage

We use `cargo-llvm-cov` for code coverage:

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --all-features --lcov --output-path lcov.info

# View HTML report
cargo llvm-cov --all-features --html
open target/llvm-cov/html/index.html
```

## Pull Request Process

1. **Fork the repository** and create your branch from `main`:
   ```bash
   git checkout -b feat/my-new-feature
   ```

2. **Make your changes** following the code quality guidelines

3. **Write or update tests** for your changes

4. **Ensure all checks pass**:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

5. **Commit your changes** using conventional commit messages:
   ```bash
   git commit -m "feat(core): add new element trait"
   ```

6. **Push to your fork**:
   ```bash
   git push origin feat/my-new-feature
   ```

7. **Open a Pull Request** with:
   - Clear description of the changes
   - Reference to any related issues
   - Test results and coverage information
   - Screenshots for UI changes (if applicable)

### PR Review Process

- At least one maintainer review is required
- All CI checks must pass
- Code coverage should not decrease significantly
- Breaking changes require special discussion and documentation

## Getting Help

- **Issues**: Check [existing issues](https://github.com/composable-delivery/busbar-sf-utam/issues) or open a new one
- **Discussions**: Use [GitHub Discussions](https://github.com/composable-delivery/busbar-sf-utam/discussions) for questions
- **Documentation**: See the [README](README.md) and inline documentation

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
