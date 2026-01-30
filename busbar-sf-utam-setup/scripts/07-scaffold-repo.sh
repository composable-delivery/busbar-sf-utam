#!/bin/bash
# UTAM Rust Project - Repository Scaffolding
# Run: ./07-scaffold-repo.sh [target-directory]

set -e

TARGET_DIR="${1:-.}"

echo "🏗️  Scaffolding UTAM Rust project in $TARGET_DIR..."

# Create directory structure
mkdir -p "$TARGET_DIR"/{utam-core,utam-compiler,utam-cli}/{src,tests}
mkdir -p "$TARGET_DIR"/.github/{workflows,ISSUE_TEMPLATE}
mkdir -p "$TARGET_DIR"/testdata/{basic,shadow-dom,compose,invalid}
mkdir -p "$TARGET_DIR"/examples/todomvc

# Root Cargo.toml
cat > "$TARGET_DIR/Cargo.toml" << 'EOF'
[workspace]
resolver = "2"
members = ["utam-core", "utam-compiler", "utam-cli"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
license = "MIT OR Apache-2.0"
repository = "https://github.com/composable-delivery/busbar-sf-utam"
keywords = ["testing", "webdriver", "page-object", "automation"]
categories = ["development-tools::testing", "web-programming"]

[workspace.dependencies]
# Runtime
thirtyfour = "0.32"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"
miette = { version = "7", features = ["fancy"] }

# Code generation
quote = "1"
proc-macro2 = "1"
syn = { version = "2", features = ["full", "parsing"] }
prettyplease = "0.2"

# CLI
clap = { version = "4", features = ["derive"] }
console = "0.15"
glob = "0.3"
notify = "6"

# Schema validation
jsonschema = "0.18"

# Testing
insta = { version = "1", features = ["json"] }

# Workspace crates
utam-core = { path = "utam-core" }
utam-compiler = { path = "utam-compiler" }
EOF

echo "✅ Created root Cargo.toml"

# utam-core/Cargo.toml
cat > "$TARGET_DIR/utam-core/Cargo.toml" << 'EOF'
[package]
name = "utam-core"
description = "Runtime library for UTAM page object framework"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
keywords.workspace = true
categories.workspace = true

[dependencies]
thirtyfour.workspace = true
tokio.workspace = true
async-trait.workspace = true
thiserror.workspace = true

[dev-dependencies]
insta.workspace = true
EOF

# utam-core/src/lib.rs
cat > "$TARGET_DIR/utam-core/src/lib.rs" << 'EOF'
//! UTAM Core Runtime Library
//!
//! This crate provides the runtime traits and types for the UTAM
//! (UI Test Automation Model) framework.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_core::prelude::*;
//!
//! // Generated page object
//! let login = LoginForm::load(&driver).await?;
//! login.login("user", "pass").await?;
//! ```

mod error;
mod traits;
mod elements;
mod wait;
mod shadow;

pub mod prelude {
    pub use crate::error::{UtamError, UtamResult};
    pub use crate::traits::*;
    pub use crate::elements::*;
    pub use crate::wait::*;
    pub use crate::shadow::*;
    pub use thirtyfour::prelude::*;
}

pub use error::{UtamError, UtamResult};
pub use traits::*;
pub use elements::*;
pub use wait::*;
pub use shadow::*;
EOF

# utam-core/src/error.rs
cat > "$TARGET_DIR/utam-core/src/error.rs" << 'EOF'
//! Error types for UTAM operations

use thiserror::Error;

/// Errors that can occur during UTAM operations
#[derive(Debug, Error)]
pub enum UtamError {
    /// Element was not found with the given selector
    #[error("Element '{name}' not found with selector: {selector}")]
    ElementNotFound { name: String, selector: String },

    /// Operation timed out
    #[error("Timeout waiting for condition: {condition}")]
    Timeout { condition: String },

    /// WebDriver operation failed
    #[error("WebDriver error: {0}")]
    WebDriver(#[from] thirtyfour::error::WebDriverError),

    /// Shadow root not found
    #[error("Shadow root not found for element: {element}")]
    ShadowRootNotFound { element: String },

    /// Invalid selector
    #[error("Invalid selector: {selector}")]
    InvalidSelector { selector: String },

    /// Frame not found
    #[error("Frame not found: {name}")]
    FrameNotFound { name: String },

    /// Assertion failed
    #[error("Assertion failed: expected {expected}, got {actual}")]
    AssertionFailed { expected: String, actual: String },
}

/// Result type for UTAM operations
pub type UtamResult<T> = Result<T, UtamError>;
EOF

# Create placeholder files for other modules
for module in traits elements wait shadow; do
  cat > "$TARGET_DIR/utam-core/src/$module.rs" << EOF
//! ${module^} module - TODO: implement

// TODO: Implement $module
EOF
done

echo "✅ Created utam-core crate"

# utam-compiler/Cargo.toml
cat > "$TARGET_DIR/utam-compiler/Cargo.toml" << 'EOF'
[package]
name = "utam-compiler"
description = "Compiler for UTAM page object JSON to Rust"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
keywords.workspace = true
categories.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
quote.workspace = true
proc-macro2.workspace = true
syn.workspace = true
prettyplease.workspace = true
thiserror.workspace = true
miette.workspace = true
jsonschema.workspace = true

[dev-dependencies]
insta.workspace = true
EOF

# utam-compiler/src/lib.rs
cat > "$TARGET_DIR/utam-compiler/src/lib.rs" << 'EOF'
//! UTAM Compiler
//!
//! Transforms UTAM JSON page object definitions into Rust source code.
//!
//! # Example
//!
//! ```rust,ignore
//! use utam_compiler::compile;
//!
//! let json = include_str!("login-form.utam.json");
//! let rust_code = compile(json)?;
//! ```

mod ast;
mod parser;
mod validator;
mod codegen;
mod error;

pub use ast::*;
pub use parser::parse;
pub use validator::validate;
pub use codegen::generate;
pub use error::{CompilerError, CompilerResult};

/// Compile UTAM JSON to Rust source code
pub fn compile(json: &str) -> CompilerResult<String> {
    let ast = parse(json)?;
    validate(&ast)?;
    let code = generate(&ast)?;
    Ok(code)
}
EOF

# Create placeholder files
for module in ast parser validator codegen error; do
  cat > "$TARGET_DIR/utam-compiler/src/$module.rs" << EOF
//! ${module^} module - TODO: implement

// TODO: Implement $module
EOF
done

echo "✅ Created utam-compiler crate"

# utam-cli/Cargo.toml
cat > "$TARGET_DIR/utam-cli/Cargo.toml" << 'EOF'
[package]
name = "utam-cli"
description = "Command-line interface for UTAM compiler"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "utam"
path = "src/main.rs"

[dependencies]
utam-compiler.workspace = true
clap.workspace = true
console.workspace = true
serde.workspace = true
serde_json.workspace = true
glob.workspace = true
notify.workspace = true
tokio.workspace = true
miette.workspace = true
EOF

# utam-cli/src/main.rs
cat > "$TARGET_DIR/utam-cli/src/main.rs" << 'EOF'
//! UTAM CLI
//!
//! Command-line interface for compiling UTAM page objects to Rust.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "utam")]
#[command(author, version, about = "UTAM Rust Compiler")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "utam.config.json")]
    config: PathBuf,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile UTAM JSON files to Rust
    Compile {
        /// Input files or directories
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Watch for changes
        #[arg(short, long)]
        watch: bool,
    },

    /// Validate UTAM JSON files
    Validate {
        /// Files to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format (text, json, sarif)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Initialize configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Lint UTAM JSON files
    Lint {
        /// Files to lint
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output SARIF report
        #[arg(long)]
        sarif: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output, watch } => {
            println!("Compiling {:?} -> {:?} (watch: {})", input, output, watch);
            // TODO: Implement
        }
        Commands::Validate { files, format } => {
            println!("Validating {:?} (format: {})", files, format);
            // TODO: Implement
        }
        Commands::Init { force } => {
            println!("Initializing config (force: {})", force);
            // TODO: Implement
        }
        Commands::Lint { files, sarif } => {
            println!("Linting {:?} (sarif: {:?})", files, sarif);
            // TODO: Implement
        }
    }
}
EOF

echo "✅ Created utam-cli crate"

# GitHub Actions CI
cat > "$TARGET_DIR/.github/workflows/ci.yml" << 'EOF'
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Build
        run: cargo build --all-targets

      - name: Test
        run: cargo test --all-features

      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Format
        run: cargo fmt --check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --lcov --output-path lcov.info

      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true

  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@1.75

      - name: Check MSRV
        run: cargo check --all-targets
EOF

echo "✅ Created CI workflow"

# README.md
cat > "$TARGET_DIR/README.md" << 'EOF'
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
EOF

echo "✅ Created README.md"

# .gitignore
cat > "$TARGET_DIR/.gitignore" << 'EOF'
/target/
Cargo.lock
*.swp
*.swo
.idea/
.vscode/
*.iml
.DS_Store
*.log
coverage/
lcov.info
EOF

# rustfmt.toml
cat > "$TARGET_DIR/rustfmt.toml" << 'EOF'
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
EOF

# Create test data examples
cat > "$TARGET_DIR/testdata/basic/simple-element.utam.json" << 'EOF'
{
  "description": "Simple clickable button",
  "root": true,
  "selector": { "css": ".simple-button" },
  "type": ["clickable"],
  "exposeRootElement": true
}
EOF

cat > "$TARGET_DIR/testdata/shadow-dom/shadow-root.utam.json" << 'EOF'
{
  "description": "Component with shadow DOM",
  "root": true,
  "selector": { "css": "my-component" },
  "shadow": {
    "elements": [
      {
        "name": "innerButton",
        "type": ["clickable"],
        "selector": { "css": ".inner-btn" },
        "public": true
      }
    ]
  }
}
EOF

echo "✅ Created test data"

echo ""
echo "🎉 Repository scaffolded successfully!"
echo ""
echo "Next steps:"
echo "  1. cd $TARGET_DIR"
echo "  2. cargo build"
echo "  3. cargo test"
echo ""
echo "Project structure:"
find "$TARGET_DIR" -type f -name "*.toml" -o -name "*.rs" -o -name "*.md" -o -name "*.yml" | head -30
