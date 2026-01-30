# UTAM Test Data

This directory contains test fixtures for UTAM compiler and runtime testing.

## Directory Structure

```
testdata/
├── basic/              # Simple element type fixtures
├── shadow-dom/         # Shadow DOM test cases
├── compose/            # Compose method examples
├── salesforce/         # Real-world Salesforce examples
└── invalid/            # Invalid UTAM JSON for error testing
```

## Test Fixtures

### Basic Elements (`basic/`)

- **simple-element.utam.json** - Minimal clickable button with root element exposure
- **clickable-button.utam.json** - Basic clickable button element
- **editable-input.utam.json** - Text input with editable type

### Shadow DOM (`shadow-dom/`)

- **shadow-root.utam.json** - Component with shadow DOM and inner elements
- **nested-shadow.utam.json** - Nested shadow DOM with custom components

### Compose Methods (`compose/`)

- **simple-method.utam.json** - Login form with simple compose method using clearAndType and click
- **chained-method.utam.json** - Search form with chained methods that call other methods
- **filter-method.utam.json** - Todo list with filter method using matchers

### Salesforce Examples (`salesforce/`)

- **salesforceStudioApp.utam.json** - Complex real-world Salesforce Studio application page object

### Invalid Test Cases (`invalid/`)

- **missing-selector.utam.json** - Missing required selector field
- **invalid-type.utam.json** - Unknown element type

## Usage

### In Integration Tests

```rust
use common::*;

#[test]
fn test_compile_fixture() {
    assert_compiles("basic/simple-element.utam.json");
}
```

### In Snapshot Tests

```rust
#[test]
fn snapshot_simple_element() {
    let code = compile_fixture("basic/simple-element.utam.json")
        .expect("Failed to compile");
    insta::assert_snapshot!("simple_element", code);
}
```

## Adding New Fixtures

1. Create a `.utam.json` file in the appropriate subdirectory
2. Add a test case in `utam-compiler/tests/compile_fixtures.rs`
3. Optionally add a snapshot test in `utam-compiler/tests/snapshot_tests.rs`
4. Update this README with the fixture description

## UTAM JSON Schema

All fixtures must conform to the [UTAM JSON grammar specification](https://utam.dev/grammar/spec).

Key elements:
- `root`: Boolean indicating if this is a root page object
- `selector`: CSS or other selector for locating the element
- `type`: Element type (clickable, editable, actionable, draggable, or custom)
- `shadow`: Shadow DOM configuration with nested elements
- `elements`: Child elements within the page object
- `methods`: Compose methods that combine element actions
