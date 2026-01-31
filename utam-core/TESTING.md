# UTAM Core Testing Guide

This document describes how to run tests for the UTAM Core library, including both unit tests and integration tests with WebDriver.

## Running Unit Tests

Unit tests don't require any external dependencies and can be run directly:

```bash
cargo test --package utam-core
```

These tests verify:
- Type signatures and trait bounds
- Struct constructors and methods
- Send/Sync requirements
- Trait implementations

## Running Integration Tests

Integration tests for the Actionable trait require a running WebDriver server. These tests are marked with `#[ignore]` by default.

### Prerequisites

You need either Chrome + ChromeDriver or Firefox + GeckoDriver:

**Option 1: ChromeDriver**
```bash
# Install ChromeDriver (version should match your Chrome version)
# On macOS:
brew install chromedriver

# On Linux:
wget https://chromedriver.storage.googleapis.com/LATEST_RELEASE
# Download the appropriate version for your system

# Run ChromeDriver on port 4444
chromedriver --port=4444
```

**Option 2: GeckoDriver (Firefox)**
```bash
# Install GeckoDriver
# On macOS:
brew install geckodriver

# On Linux:
# Download from https://github.com/mozilla/geckodriver/releases

# Run GeckoDriver on port 4444
geckodriver --port=4444
```

### Running the Integration Tests

With ChromeDriver or GeckoDriver running on port 4444, execute:

```bash
cargo test --package utam-core -- --ignored --test-threads=1
```

The `--test-threads=1` flag ensures tests run sequentially (important for browser tests).

## Test Coverage

To generate test coverage reports:

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage for unit tests only
cargo llvm-cov --package utam-core --summary-only

# Generate coverage with integration tests (requires WebDriver)
cargo llvm-cov --package utam-core -- --ignored --test-threads=1
```

### Coverage Details

**Current Coverage** (unit tests only):
- `elements.rs`: ~61% - Covers struct creation and trait implementations
- `traits.rs`: ~27% - Limited because actual method execution requires WebDriver

**With Integration Tests** (requires WebDriver):
- Coverage increases to ~90%+ as all Actionable trait methods are exercised

## Test Organization

### Unit Tests (`src/**/*.rs`)

Located in `#[cfg(test)] mod tests` blocks within source files:
- `elements.rs`: Tests for BaseElement and ActionableElement
- `traits.rs`: Tests for Actionable trait bounds and signatures

### Integration Tests (`tests/runtime_tests.rs`)

- **Basic tests**: Run without WebDriver, verify types and API
- **Actionable tests**: Require WebDriver, test actual browser interactions
  - `test_actionable_focus`: Verifies focus() sets document.activeElement
  - `test_actionable_blur`: Verifies blur() removes focus
  - `test_actionable_scroll_to_center`: Verifies scrollIntoView centering
  - `test_actionable_scroll_to_top`: Verifies scrollIntoView top alignment
  - `test_actionable_move_to`: Verifies mouse hover action

## Continuous Integration

The CI workflow automatically:
1. Runs unit tests on all commits
2. Generates coverage reports using cargo-llvm-cov
3. Uploads coverage to Codecov

Integration tests requiring WebDriver are not run in CI but can be run locally for validation.
