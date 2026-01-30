# UTAM Rust

[![CI](https://github.com/composable-delivery/busbar-sf-utam/workflows/CI/badge.svg)](https://github.com/composable-delivery/busbar-sf-utam/actions)
[![codecov](https://codecov.io/gh/composable-delivery/busbar-sf-utam/branch/main/graph/badge.svg)](https://codecov.io/gh/composable-delivery/busbar-sf-utam)
[![Crates.io - utam-core](https://img.shields.io/crates/v/utam-core.svg)](https://crates.io/crates/utam-core)
[![Crates.io - utam-compiler](https://img.shields.io/crates/v/utam-compiler.svg)](https://crates.io/crates/utam-compiler)
[![Crates.io - utam-cli](https://img.shields.io/crates/v/utam-cli.svg)](https://crates.io/crates/utam-cli)
[![docs.rs](https://img.shields.io/docsrs/utam-core)](https://docs.rs/utam-core)

Rust implementation of [UTAM](https://utam.dev) - UI Test Automation Model.

## Overview

UTAM is a declarative page object framework that separates page object definitions (JSON) from their implementation. This repository provides:

| Crate | Description | Version | Docs |
|-------|-------------|---------|------|
| **utam-core** | Runtime library with WebDriver traits and element wrappers | [![Crates.io](https://img.shields.io/crates/v/utam-core.svg)](https://crates.io/crates/utam-core) | [![docs.rs](https://img.shields.io/docsrs/utam-core)](https://docs.rs/utam-core) |
| **utam-compiler** | Transforms UTAM JSON to Rust source code | [![Crates.io](https://img.shields.io/crates/v/utam-compiler.svg)](https://crates.io/crates/utam-compiler) | [![docs.rs](https://img.shields.io/docsrs/utam-compiler)](https://docs.rs/utam-compiler) |
| **utam-cli** | Command-line interface for the compiler | [![Crates.io](https://img.shields.io/crates/v/utam-cli.svg)](https://crates.io/crates/utam-cli) | - |

## Features

- 🚀 **Type-safe**: Generated Rust code with full type safety
- ⚡ **Async/await**: Modern async Rust with Tokio runtime
- 🎯 **Shadow DOM**: First-class support for Shadow DOM encapsulation
- 🔧 **Modular**: Separate runtime, compiler, and CLI crates
- 📦 **WebDriver**: Built on the robust [thirtyfour](https://crates.io/crates/thirtyfour) WebDriver library
- 🛠️ **Extensible**: Custom element types and compose methods

## Quick Start

### Installation

Add the runtime library to your project:

```bash
cargo add utam-core
```

Or install the CLI tool globally:

```bash
cargo install utam-cli
```

### Using the CLI

```bash
# Initialize a project
utam init

# Compile page objects
utam compile src/pageobjects/

# Watch mode for development
utam watch src/pageobjects/

# Run tests
cargo test
```

### Using as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
utam-core = "0.1"
utam-compiler = "0.1"
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

- [Architecture Guide](ARCHITECTURE.md) - System design and crate structure
- [Contributing Guide](CONTRIBUTING.md) - How to contribute to this project
- [UTAM Grammar Specification](https://utam.dev/grammar/spec) - Official UTAM spec
- [API Reference - utam-core](https://docs.rs/utam-core) - Runtime API docs
- [API Reference - utam-compiler](https://docs.rs/utam-compiler) - Compiler API docs

## Community

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Issue Tracker](https://github.com/composable-delivery/busbar-sf-utam/issues)
- [Discussions](https://github.com/composable-delivery/busbar-sf-utam/discussions)

## License

Dual-licensed under MIT or Apache-2.0.
