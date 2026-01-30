# GitHub Copilot Instructions for UTAM Rust

## Project Overview

This repository contains the Rust implementation of UTAM (UI Test Automation Model), a declarative page object framework. The project consists of three crates:

- **utam-core**: Runtime library providing traits and types for WebDriver automation
- **utam-compiler**: Transforms UTAM JSON page object definitions into Rust code
- **utam-cli**: Command-line interface for the compiler

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        UTAM Architecture                             │
├─────────────────────────────────────────────────────────────────────┤
│  .utam.json → Parser → Validator → CodeGen → .rs files → Runtime    │
└─────────────────────────────────────────────────────────────────────┘
```

## Development Workflow

### Before Committing - REQUIRED CHECKS

**ALWAYS run these commands before committing code:**

1. **Format code**: `cargo fmt --all`
2. **Run linter**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **Run tests**: `cargo test --all-features`
4. **Build all targets**: `cargo build --all-targets --all-features`

These checks are enforced by CI and pre-commit hooks. **Do not skip these steps.**

### Pre-commit Hooks

The repository uses pre-commit hooks (`.pre-commit-config.yaml`). Install with:
```bash
pip install pre-commit
pre-commit install
```

This ensures formatting and linting are automatically checked before commits.

## Code Style Guidelines

### Naming Conventions
- Structs: `PascalCase` (e.g., `PageObjectAst`, `ClickableElement`)
- Functions/methods: `snake_case` (e.g., `get_element`, `wait_for_visible`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `ROOT_SELECTOR`)
- Type parameters: Single uppercase letters or descriptive `PascalCase` (e.g., `T`, `PageObject`)

### Error Handling
Always use the project's error types:
```rust
use crate::error::{UtamError, UtamResult};

// Prefer ? operator for propagation
async fn get_text(&self) -> UtamResult<String> {
    Ok(self.inner.text().await?)
}

// Use context for better errors
element.click().await.map_err(|e| UtamError::ElementNotFound {
    name: "submitButton".to_string(),
    selector: "button[type='submit']".to_string(),
})?;
```

### Async Patterns
```rust
use async_trait::async_trait;

#[async_trait]
pub trait SomeTrait: Send + Sync {
    async fn some_method(&self) -> UtamResult<()>;
}
```

### Documentation
```rust
/// Brief description of the item.
///
/// Longer description if needed, explaining behavior,
/// edge cases, and usage examples.
///
/// # Arguments
///
/// * `name` - Description of the argument
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// * `UtamError::ElementNotFound` - When the element doesn't exist
///
/// # Examples
///
/// ```rust
/// let element = page.get_button().await?;
/// element.click().await?;
/// ```
pub async fn method(&self, name: &str) -> UtamResult<Element> {
    // ...
}
```

## UTAM JSON Grammar Quick Reference

### Root Page Object
```json
{
  "root": true,
  "selector": { "css": "app-root" },
  "shadow": { "elements": [...] },
  "elements": [...],
  "methods": [...]
}
```

### Element Types
- **Basic**: `"type": ["clickable", "editable"]`
- **Custom**: `"type": "package/pageObjects/component"`
- **Container**: `"type": "container"`
- **Frame**: `"type": "frame"`

### Action Types
- `actionable`: focus, blur, scroll, moveTo
- `clickable`: click, doubleClick, rightClick (extends actionable)
- `editable`: clear, setText, clearAndType (extends actionable)
- `draggable`: dragAndDrop, dragAndDropByOffset (extends actionable)

### Compose Methods
```json
{
  "name": "login",
  "compose": [
    { "element": "usernameInput", "apply": "clearAndType", "args": [...] },
    { "element": "submitButton", "apply": "click" }
  ]
}
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `thirtyfour` | WebDriver client for browser automation |
| `tokio` | Async runtime |
| `serde` | JSON serialization/deserialization |
| `quote` | Rust code generation via quasi-quoting |
| `syn` | Rust syntax parsing |
| `thiserror` | Derive macro for error types |
| `async-trait` | Async methods in traits |
| `clap` | CLI argument parsing |

## Testing Conventions

### Unit Tests
Place in `mod tests` at the bottom of source files:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selector() {
        let json = r#"{"css": ".button"}"#;
        let selector: SelectorAst = serde_json::from_str(json).unwrap();
        assert!(selector.css.is_some());
    }
}
```

### Integration Tests
Place in `tests/` directory:
```rust
// tests/compile_page_object.rs
use utam_compiler::compile;

#[test]
fn compile_basic_page_object() {
    let input = include_str!("testdata/basic.utam.json");
    let output = compile(input).unwrap();
    insta::assert_snapshot!(output);
}
```

### Snapshot Testing
Use `insta` for snapshot testing generated code:
```rust
#[test]
fn generate_element_getter() {
    let code = generator.generate(&ast);
    insta::assert_snapshot!("element_getter", code);
}
```

## Common Tasks

### Adding a New Action Type
1. Define trait in `utam-core/src/traits/`
2. Implement for element wrapper in `utam-core/src/elements/`
3. Add codegen support in `utam-compiler/src/codegen/`
4. Update JSON schema if needed
5. Add tests

### Adding a CLI Command
1. Add variant to `Commands` enum in `utam-cli/src/main.rs`
2. Implement command handler
3. Update help text
4. Add integration tests

### Parsing New JSON Feature
1. Add fields to AST types in `utam-compiler/src/ast/`
2. Update validation if needed
3. Add codegen support
4. Add test fixtures in `testdata/`

## File Organization

```
utam-core/
├── src/
│   ├── lib.rs           # Re-exports and prelude
│   ├── error.rs         # UtamError, UtamResult
│   ├── traits/          # Actionable, Clickable, etc.
│   ├── elements/        # Element wrapper types
│   ├── wait.rs          # Wait utilities
│   └── shadow.rs        # Shadow DOM support

utam-compiler/
├── src/
│   ├── lib.rs           # Public API
│   ├── ast/             # AST type definitions
│   ├── parser.rs        # JSON parsing
│   ├── validator.rs     # Semantic validation
│   ├── codegen/         # Rust code generation
│   └── error.rs         # Compiler errors

utam-cli/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── commands/        # Subcommand implementations
│   └── config.rs        # Configuration handling
```

## Performance Considerations

- Compile JSON schema once and reuse
- Use `Cow<'_, str>` for zero-copy string handling where possible
- Prefer `&str` over `String` in function parameters
- Use `Vec::with_capacity` when size is known
- Profile with `cargo flamegraph` before optimizing

## Security Notes

- Never execute JavaScript from untrusted UTAM JSON
- Validate all selectors before use
- Sanitize file paths in CLI
- Don't expose internal errors to users
