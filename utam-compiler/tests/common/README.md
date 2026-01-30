# UTAM Compiler Test Utilities

Common test utilities for UTAM compiler integration tests.

## Overview

This module provides helper functions for loading test fixtures and asserting compilation results.

## Functions

### `load_fixture(path: &str) -> String`

Loads a test fixture from the testdata directory.

**Example:**
```rust
let json = load_fixture("basic/simple-element.utam.json");
```

### `compile_fixture(path: &str) -> CompilerResult<String>`

Compiles a test fixture and returns the generated Rust code.

**Example:**
```rust
let code = compile_fixture("basic/simple-element.utam.json")?;
```

### `assert_compiles(path: &str)`

Asserts that a fixture compiles successfully. Panics if compilation fails.

**Example:**
```rust
#[test]
fn test_simple_element() {
    assert_compiles("basic/simple-element.utam.json");
}
```

### `assert_fails_to_compile(path: &str)`

Asserts that a fixture fails to compile. Panics if compilation succeeds.

**Example:**
```rust
#[test]
fn test_invalid_fixture() {
    assert_fails_to_compile("invalid/missing-selector.utam.json");
}
```

### `assert_compile_error_contains(path: &str, expected_msg: &str)`

Asserts that compilation fails with an error message containing the expected string.

**Example:**
```rust
#[test]
fn test_missing_selector_error() {
    assert_compile_error_contains(
        "invalid/missing-selector.utam.json",
        "missing selector"
    );
}
```

## Usage

Import the utilities in your test files:

```rust
mod common;
use common::*;
```

## Test Fixtures

All test fixtures are located in the `testdata/` directory at the workspace root.
See `testdata/README.md` for details on available fixtures.
