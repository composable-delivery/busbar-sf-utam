# UTAM Core Test Utilities

Common test utilities for UTAM runtime integration tests.

## Overview

This module provides WebDriver setup and element assertion helpers for testing UTAM runtime behavior with real browsers.

## Test Types

### Unit Tests
- Located in `src/` directories as `#[cfg(test)] mod tests`
- Run with: `cargo test`
- Do not require external services

### Integration Tests
- Located in `tests/` directory
- Require WebDriver (ChromeDriver) running on port 9515
- Run with: `cargo test --test <test_name> -- --ignored`

## WebDriver Setup

### Prerequisites

1. **Install Chrome/Chromium**
2. **Install ChromeDriver**:
   ```bash
   # On macOS
   brew install chromedriver
   
   # On Ubuntu
   sudo apt-get install chromium-chromedriver
   
   # Or download from: https://chromedriver.chromium.org/
   ```

3. **Start ChromeDriver**:
   ```bash
   chromedriver --port=9515
   ```

### `TestDriverConfig`

Configuration for WebDriver setup.

**Fields:**
- `headless: bool` - Run browser in headless mode (default: true)
- `implicit_wait_ms: u64` - Implicit wait timeout in milliseconds (default: 5000)

### `setup_test_driver(config: TestDriverConfig) -> UtamResult<WebDriver>`

Sets up a test WebDriver connected to ChromeDriver.

**Example:**
```rust
#[tokio::test]
#[ignore = "Requires ChromeDriver"]
async fn test_with_driver() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    driver.goto("http://example.com").await?;
    // ... test logic ...
    driver.quit().await?;
    Ok(())
}
```

## Test HTML Files

Test HTML files are located in `tests/testdata/`:
- `frame_test.html` - Main test page with iframes
- `frame_content.html` - Simple iframe content
- `frame_nested.html` - Outer frame for nested tests
- `frame_inner.html` - Inner frame for nested tests

### `get_test_file_url(filename: &str) -> String`

Returns a `file://` URL for a test HTML file.

**Example:**
```rust
driver.goto(get_test_file_url("frame_test.html")).await?;
```

## Element Assertions

### `assert_element_visible(element: &WebElement) -> UtamResult<()>`

Asserts that an element is displayed on the page.

**Example:**
```rust
let button = driver.find(By::Id("button")).await?;
assert_element_visible(&button).await?;
```

### `assert_element_not_visible(element: &WebElement) -> UtamResult<()>`

Asserts that an element is not displayed on the page.

### `assert_element_text(element: &WebElement, expected: &str) -> UtamResult<()>`

Asserts that an element has the expected text content.

**Example:**
```rust
let button = driver.find(By::Id("button")).await?;
assert_element_text(&button, "Submit").await?;
```

### `assert_element_attribute(element: &WebElement, attr: &str, expected: &str) -> UtamResult<()>`

Asserts that an element has the expected attribute value.

**Example:**
```rust
let input = driver.find(By::Id("input")).await?;
assert_element_attribute(&input, "type", "text").await?;
```

## Running Tests

### Run all unit tests:
```bash
cargo test
```

### Run specific integration test:
```bash
# Start ChromeDriver first
chromedriver --port=9515

# In another terminal
cargo test --test frame_integration_tests -- --ignored
```

### Run all integration tests:
```bash
cargo test -- --ignored
```

## Writing New Tests

### Integration Test Template

```rust
mod common;
use common::*;
use utam_core::prelude::*;

#[tokio::test]
#[ignore = "Requires ChromeDriver running on port 9515"]
async fn test_my_feature() -> UtamResult<()> {
    let driver = setup_test_driver(TestDriverConfig::default()).await?;
    
    // Load test page
    driver.goto(get_test_file_url("test_page.html")).await?;
    
    // Your test logic here
    let element = driver.find(By::Id("my-element")).await?;
    assert_element_visible(&element).await?;
    
    driver.quit().await?;
    Ok(())
}
```

## Coverage

To run tests with coverage:
```bash
cargo llvm-cov --package utam-core --lcov --output-path lcov.info
```

## See Also

- [thirtyfour documentation](https://docs.rs/thirtyfour/)
- [UTAM core API](../../src/lib.rs)
- [WebDriver Protocol](https://www.w3.org/TR/webdriver/)
