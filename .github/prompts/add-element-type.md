# Prompt Template: Adding a New Element Type

## Task Overview
Add a new element type to the UTAM grammar that can be used in `.utam.json` files.

## Prerequisites
- Understand the UTAM JSON grammar
- Know which actions this element type supports
- Determine if it extends an existing type

## Steps to Complete

### 1. Define the Element Type in AST
**File**: `utam-compiler/src/ast/element.rs`

Add the new variant to the `ElementType` enum:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElementType {
    Basic(Vec<String>),
    Custom(String),
    Container,
    Frame,
    NewType,  // Add your new type here
}
```

### 2. Create Runtime Trait (if needed)
**File**: `utam-core/src/traits/your_new_type.rs`

Define the trait with required methods:
```rust
use async_trait::async_trait;
use crate::error::UtamResult;
use thirtyfour::WebElement;

#[async_trait]
pub trait YourNewType: Send + Sync {
    fn inner(&self) -> &WebElement;

    async fn new_action(&self) -> UtamResult<()> {
        // Implementation
        Ok(())
    }
}
```

Export the trait:
**File**: `utam-core/src/traits/mod.rs`
```rust
mod your_new_type;
pub use your_new_type::YourNewType;
```

### 3. Implement for Element Wrapper
**File**: `utam-core/src/elements/base_element.rs`

```rust
#[async_trait]
impl YourNewType for BaseElement {
    fn inner(&self) -> &WebElement {
        &self.element
    }

    async fn new_action(&self) -> UtamResult<()> {
        // Call WebDriver methods
        self.inner().some_webdriver_method().await?;
        Ok(())
    }
}
```

### 4. Add Code Generation Support
**File**: `utam-compiler/src/codegen/element.rs`

Update the code generator to handle the new type:
```rust
fn generate_element_type_bounds(element_type: &ElementType) -> Vec<TokenStream> {
    match element_type {
        ElementType::NewType => vec![quote!(YourNewType)],
        // ... other cases
    }
}
```

### 5. Update Validation
**File**: `utam-compiler/src/validator.rs`

Add validation rules for the new type:
```rust
fn validate_element_type(element: &ElementAst) -> Result<(), UtamError> {
    match &element.element_type {
        ElementType::NewType => {
            // Validate that required fields are present
            // Validate that incompatible fields are not present
            Ok(())
        }
        // ... other cases
    }
}
```

### 6. Update JSON Schema (if separate)
If you maintain a JSON schema file, update it:
```json
{
  "type": {
    "oneOf": [
      {"type": "array", "items": {"type": "string"}},
      {"type": "string"},
      {"const": "container"},
      {"const": "frame"},
      {"const": "newtype"}
    ]
  }
}
```

### 7. Add Tests

**Unit test** (`utam-compiler/src/ast/element.rs`):
```rust
#[test]
fn test_parse_new_type_element() {
    let json = r#"{
        "name": "example",
        "type": "newtype",
        "selector": {"css": ".example"}
    }"#;
    let element: ElementAst = serde_json::from_str(json).unwrap();
    assert!(matches!(element.element_type, ElementType::NewType));
}
```

**Integration test** (`utam-compiler/tests/compile_new_type.rs`):
```rust
#[test]
fn test_compile_new_type_element() {
    let input = include_str!("../testdata/new-type.utam.json");
    let output = compile(input).unwrap();
    assert!(output.contains("impl YourNewType"));
}
```

**Runtime test** (`utam-core/tests/new_type_actions.rs`):
```rust
#[tokio::test]
async fn test_new_type_action() -> UtamResult<()> {
    let driver = setup_driver().await?;
    let element = BaseElement::new(driver.find(By::Css(".test")).await?);
    element.new_action().await?;
    Ok(())
}
```

### 8. Create Test Data
**File**: `testdata/new-type.utam.json`
```json
{
  "root": true,
  "selector": {"css": "app-root"},
  "elements": [
    {
      "name": "exampleElement",
      "type": "newtype",
      "selector": {"css": ".example"},
      "public": true
    }
  ]
}
```

### 9. Update Documentation
- Add to `.github/copilot-instructions.md` in "Element Types" section
- Update README.md with examples
- Add JSDoc/inline docs

## Testing Checklist
- [ ] Element type parses from JSON
- [ ] Validation accepts valid configurations
- [ ] Validation rejects invalid configurations
- [ ] Code generation produces valid Rust
- [ ] Generated code compiles
- [ ] Runtime actions work as expected
- [ ] Documentation updated

## Example Commit Message
```
feat(core): add NewType element type

- Add NewType variant to ElementType enum
- Implement YourNewType trait with new_action
- Add codegen support for new type
- Add comprehensive tests
- Update documentation

Closes #XXX
```

## Checklist Before PR
- [ ] All tests pass
- [ ] Clippy warnings addressed
- [ ] Code formatted with rustfmt
- [ ] Documentation updated
- [ ] Changelog entry added (if applicable)
- [ ] Examples provided
