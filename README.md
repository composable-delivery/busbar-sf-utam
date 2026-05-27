# UTAM Rust

[![CI](https://github.com/composable-delivery/busbar-sf-utam/workflows/CI/badge.svg)](https://github.com/composable-delivery/busbar-sf-utam/actions)
[![codecov](https://codecov.io/gh/composable-delivery/busbar-sf-utam/branch/main/graph/badge.svg)](https://codecov.io/gh/composable-delivery/busbar-sf-utam)

Rust implementation of [UTAM](https://utam.dev) — UI Test Automation Model.

UTAM is a declarative page object framework that separates page object
definitions (JSON) from their implementation. This repository provides both
a **compiler** (JSON → Rust code) and a **runtime interpreter** (JSON →
callable interface at runtime) — plus a test harness, CLI, and 1,454
pre-built Salesforce page objects.

## Crates

| Crate | Description |
|-------|-------------|
| **[utam-core](utam-core/)** | Runtime library — WebDriver traits, element wrappers, wait utilities, shadow DOM support |
| **[utam-compiler](utam-compiler/)** | Transforms UTAM JSON page objects into Rust source code |
| **[utam-runtime](utam-runtime/)** | Runtime interpreter — load JSON and call methods dynamically, no compilation needed |
| **[utam-test](utam-test/)** | Test harness with screenshot-on-failure, retry logic, assertion helpers |
| **[utam-cli](utam-cli/)** | Command-line interface for the compiler |

## Quick Start

### Compiled path (traditional)

```bash
# Compile page objects to Rust
cargo run -p utam-cli -- compile src/pageobjects/ --output src/generated/

# Use generated code in tests
cargo test
```

### Runtime path (dynamic / agent-friendly)

```rust
use utam_runtime::prelude::*;
use std::collections::HashMap;

// Load page objects from disk
let mut registry = PageObjectRegistry::new();
registry.add_search_path("./salesforce-pageobjects");
registry.scan()?;

// Discover and introspect
let ast = registry.get("helpers/login")?;
let page = DynamicPageObject::load(driver, ast).await?;
println!("{:?}", page.method_signatures());
// → [MethodInfo { name: "login", args: [userNameStr: string, passwordStr: string] }]

// Execute
let mut args = HashMap::new();
args.insert("userNameStr".into(), RuntimeValue::String("admin@sf.com".into()));
args.insert("passwordStr".into(), RuntimeValue::String("pass".into()));
page.call_method("login", &args).await?;
```

## Example Page Object

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
        { "element": "usernameInput", "apply": "clearAndType",
          "args": [{ "name": "username", "type": "string" }] },
        { "element": "submitButton", "apply": "click" }
      ]
    }
  ]
}
```

## Salesforce Page Objects

The [`salesforce-pageobjects/`](salesforce-pageobjects/) directory contains
1,454 UTAM page object definitions across 74 Salesforce modules (Lightning,
Sales, Service, Experience Cloud, OmniStudio, and more). These parse at
96.5% and compile at 99.8% compatibility.

## Architecture

```
UTAM JSON ──→ utam-compiler ──→ Rust structs (compiled tests)
    │
    └────────→ utam-runtime ──→ DynamicPageObject (runtime interpreter)
                   │
                   ├── UtamDriver trait (protocol-agnostic)
                   │    └── ThirtyfourDriver (WebDriver/Selenium)
                   │
                   ├── DynamicElement (action dispatch by name)
                   └── PageObjectRegistry (1,454 SF objects)
```

See [docs/architecture.md](docs/architecture.md) for details.

## Documentation

- [Architecture Overview](docs/architecture.md)
- [Runtime Interpreter Guide](docs/runtime-guide.md)
- [Page Object Authoring Guide](docs/page-object-guide.md)
- [UTAM Grammar Specification](https://utam.dev/grammar/spec)
- [Contributing Guide](CONTRIBUTING.md)

## License

Dual-licensed under MIT or Apache-2.0.
