# UTAM Rust

[![CI](https://github.com/composable-delivery/busbar-sf-utam/workflows/CI/badge.svg)](https://github.com/composable-delivery/busbar-sf-utam/actions)
[![codecov](https://codecov.io/gh/composable-delivery/busbar-sf-utam/branch/main/graph/badge.svg)](https://codecov.io/gh/composable-delivery/busbar-sf-utam)

Rust implementation of [UTAM](https://utam.dev) - UI Test Automation Model.

## Overview

UTAM is a declarative page object framework that separates page object definitions (JSON) from their implementation. This repository provides:

- **utam-core**: Runtime library with WebDriver traits and element wrappers
- **utam-compiler**: Transforms UTAM JSON to Rust source code
- **utam-cli**: Command-line interface for the compiler

## Quick Start

```bash
# Install the CLI
cargo install utam-cli

# Initialize a project
utam init

# Compile page objects
utam compile src/pageobjects/

# Run tests
cargo test
```

## Example

Define a page object in JSON:

```json
{
  "description": "Login form",
  "root": true,
  "selector": { "css": "login-form" },
  "shadow": {
    "elements": [
      {
        "name": "usernameInput",
        "type": ["editable"],
        "selector": { "css": "input[name='username']" }
      },
      {
        "name": "submitButton",
        "type": ["clickable"],
        "selector": { "css": "button[type='submit']" },
        "public": true
      }
    ]
  },
  "methods": [
    {
      "name": "login",
      "compose": [
        { "element": "usernameInput", "apply": "clearAndType", "args": [{ "name": "username", "type": "string" }] },
        { "element": "submitButton", "apply": "click" }
      ]
    }
  ]
}
```

Use the generated Rust code:

```rust
use utam_core::prelude::*;

#[tokio::test]
async fn test_login() -> UtamResult<()> {
    let driver = setup_driver().await?;
    let login = LoginForm::load(&driver).await?;

    login.login("testuser").await?;
    login.get_submit_button().await?.click().await?;

    Ok(())
}
```

## Documentation

- [UTAM Grammar Specification](https://utam.dev/grammar/spec)
- [API Reference](https://docs.rs/utam-core)
- [Contributing Guide](CONTRIBUTING.md)

## License

Dual-licensed under MIT or Apache-2.0.
