# UTAM Core Test Utilities

Common test utilities for UTAM runtime integration tests.

## Overview

This module provides mock WebDriver setup and element assertion helpers for testing UTAM runtime behavior.

## Mock WebDriver

### `MockDriverConfig`

Configuration for mock WebDriver setup.

**Fields:**
- `headless: bool` - Run browser in headless mode (default: true)
- `implicit_wait_ms: u64` - Implicit wait timeout in milliseconds (default: 5000)

### `setup_mock_driver() -> UtamResult<()>`

Sets up a mock WebDriver for testing.

**Note:** This is currently a placeholder. In production tests, you would either:
1. Use a real WebDriver with a test browser (e.g., ChromeDriver with Chrome)
2. Use a mock WebDriver implementation
3. Use dependency injection to provide test doubles

## Element Assertions

### `assert_element_visible(element: &WebElement) -> UtamResult<()>`

Asserts that an element is displayed on the page.

**Example:**
```rust
#[tokio::test]
async fn test_button_visible() {
    let element = driver.find_element(By::Css("button")).await?;
    assert_element_visible(&element).await?;
}
```

### `assert_element_not_visible(element: &WebElement) -> UtamResult<()>`

Asserts that an element is not displayed on the page.

### `assert_element_text(element: &WebElement, expected: &str) -> UtamResult<()>`

Asserts that an element has the expected text content.

**Example:**
```rust
#[tokio::test]
async fn test_button_text() {
    let button = driver.find_element(By::Css("button")).await?;
    assert_element_text(&button, "Submit").await?;
}
```

### `assert_element_attribute(element: &WebElement, attr: &str, expected: &str) -> UtamResult<()>`

Asserts that an element has the expected attribute value.

**Example:**
```rust
#[tokio::test]
async fn test_input_type() {
    let input = driver.find_element(By::Css("input")).await?;
    assert_element_attribute(&input, "type", "text").await?;
}
```

## Usage

Import the utilities in your test files:

```rust
mod common;
use common::*;
```

## Integration with Real WebDriver

For full integration tests with a real browser:

```rust
use thirtyfour::prelude::*;

#[tokio::test]
async fn test_with_real_driver() -> UtamResult<()> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;
    
    driver.goto("http://example.com").await?;
    let button = driver.find_element(By::Css("button")).await?;
    assert_element_visible(&button).await?;
    
    driver.quit().await?;
    Ok(())
}
```

## See Also

- [thirtyfour documentation](https://docs.rs/thirtyfour/)
- [UTAM core API](../../src/lib.rs)
