# Prompt Template: Adding a New Action Trait

## Task Overview
Add a new action trait to utam-core that elements can implement (e.g., Scrollable, Hoverable, Resizable).

## Prerequisites
- Understand async Rust and async-trait
- Know WebDriver capabilities
- Determine which element types should support this action

## Steps to Complete

### 1. Define the Trait
**File**: `utam-core/src/traits/your_action.rs`

```rust
use async_trait::async_trait;
use crate::error::UtamResult;
use thirtyfour::WebElement;

/// Provides actions for [describe what this trait does].
///
/// # Examples
///
/// ```rust
/// use utam_core::prelude::*;
///
/// #[tokio::test]
/// async fn example() -> UtamResult<()> {
///     let driver = setup_driver().await?;
///     let element = BaseElement::new(driver.find(By::Css(".item")).await?);
///     element.your_action().await?;
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait YourAction: Send + Sync {
    /// Get the underlying WebElement
    fn inner(&self) -> &WebElement;

    /// Performs [describe action].
    ///
    /// # Errors
    ///
    /// Returns error if [describe failure conditions].
    async fn your_action(&self) -> UtamResult<()> {
        self.inner()
            .some_webdriver_method()
            .await
            .map_err(|e| e.into())
    }

    /// Another action method with parameters
    async fn action_with_params(&self, param: i32) -> UtamResult<String> {
        // Implementation
        Ok("result".to_string())
    }
}
```

### 2. Export from Module
**File**: `utam-core/src/traits/mod.rs`

```rust
mod your_action;
pub use your_action::YourAction;
```

Add to prelude:
**File**: `utam-core/src/lib.rs`

```rust
pub mod prelude {
    pub use crate::traits::YourAction;
    // ... other exports
}
```

### 3. Implement for BaseElement
**File**: `utam-core/src/elements/base_element.rs`

```rust
use crate::traits::YourAction;

#[async_trait]
impl YourAction for BaseElement {
    fn inner(&self) -> &WebElement {
        &self.element
    }
    
    // Optional: override default implementation if needed
    async fn your_action(&self) -> UtamResult<()> {
        // Custom implementation
        Ok(())
    }
}
```

### 4. Update Compiler to Recognize Action
**File**: `utam-compiler/src/ast/element.rs`

Add to recognized action types:
```rust
const KNOWN_ACTIONS: &[&str] = &[
    "actionable",
    "clickable",
    "editable",
    "draggable",
    "your_action",  // Add here
];

fn parse_action_type(s: &str) -> Option<ActionType> {
    match s {
        "actionable" => Some(ActionType::Actionable),
        "clickable" => Some(ActionType::Clickable),
        "editable" => Some(ActionType::Editable),
        "draggable" => Some(ActionType::Draggable),
        "your_action" => Some(ActionType::YourAction),
        _ => None,
    }
}
```

### 5. Update Code Generation
**File**: `utam-compiler/src/codegen/element.rs`

```rust
fn generate_trait_bounds(actions: &[String]) -> Vec<TokenStream> {
    actions.iter().map(|action| {
        match action.as_str() {
            "actionable" => quote!(Actionable),
            "clickable" => quote!(Clickable),
            "editable" => quote!(Editable),
            "draggable" => quote!(Draggable),
            "your_action" => quote!(YourAction),  // Add here
            _ => quote!(),
        }
    }).collect()
}
```

### 6. Add Validation Rules
**File**: `utam-compiler/src/validator.rs`

```rust
fn validate_action_compatibility(element: &ElementAst) -> Result<(), UtamError> {
    // Ensure your_action is only used with compatible element types
    if element.actions.contains(&"your_action".to_string()) {
        if element.element_type == ElementType::Frame {
            return Err(UtamError::IncompatibleAction {
                element: element.name.clone(),
                action: "your_action".to_string(),
                reason: "Frames cannot use your_action".to_string(),
            });
        }
    }
    Ok(())
}
```

### 7. Update Compose Methods Support
**File**: `utam-compiler/src/ast/method.rs`

Add to recognized method applications:
```rust
fn is_valid_apply(apply: &str) -> bool {
    matches!(apply, 
        "click" | "doubleClick" | "rightClick" |
        "clear" | "setText" | "clearAndType" |
        "focus" | "blur" | "scroll" | "moveTo" |
        "dragAndDrop" | "dragAndDropByOffset" |
        "yourAction" | "actionWithParams"  // Add here
    )
}
```

### 8. Add Unit Tests
**File**: `utam-core/tests/your_action_tests.rs`

```rust
use utam_core::prelude::*;
use thirtyfour::prelude::*;

async fn setup_driver() -> UtamResult<WebDriver> {
    let caps = DesiredCapabilities::chrome();
    WebDriver::new("http://localhost:4444", caps)
        .await
        .map_err(|e| e.into())
}

#[tokio::test]
async fn test_your_action() -> UtamResult<()> {
    let driver = setup_driver().await?;
    driver.goto("http://localhost:8080/test.html").await?;
    
    let element = BaseElement::new(
        driver.find(By::Css(".test-element")).await?
    );
    
    element.your_action().await?;
    
    // Verify state change
    assert!(some_condition);
    Ok(())
}

#[tokio::test]
async fn test_action_with_params() -> UtamResult<()> {
    let driver = setup_driver().await?;
    let element = BaseElement::new(
        driver.find(By::Css(".test")).await?
    );
    
    let result = element.action_with_params(42).await?;
    assert_eq!(result, "expected");
    Ok(())
}
```

### 9. Add Compiler Tests
**File**: `utam-compiler/tests/compile_your_action.rs`

```rust
use utam_compiler::compile;

#[test]
fn test_compile_element_with_your_action() {
    let input = r#"{
        "root": true,
        "selector": {"css": "app-root"},
        "elements": [{
            "name": "actionableElement",
            "type": ["your_action"],
            "selector": {"css": ".element"}
        }]
    }"#;
    
    let output = compile(input).unwrap();
    assert!(output.contains("YourAction"));
}

#[test]
fn test_compile_compose_with_your_action() {
    let input = r#"{
        "root": true,
        "selector": {"css": "root"},
        "elements": [{
            "name": "elem",
            "type": ["your_action"],
            "selector": {"css": ".elem"}
        }],
        "methods": [{
            "name": "doAction",
            "compose": [{
                "element": "elem",
                "apply": "yourAction"
            }]
        }]
    }"#;
    
    let output = compile(input).unwrap();
    assert!(output.contains("yourAction"));
}
```

### 10. Create Test Data
**File**: `testdata/your-action.utam.json`

```json
{
  "description": "Test page with YourAction elements",
  "root": true,
  "selector": {"css": "test-page"},
  "elements": [
    {
      "name": "actionElement",
      "type": ["your_action"],
      "selector": {"css": ".action-elem"},
      "public": true
    }
  ],
  "methods": [
    {
      "name": "performAction",
      "compose": [
        {
          "element": "actionElement",
          "apply": "yourAction"
        }
      ]
    }
  ]
}
```

### 11. Update Documentation

**File**: `.github/copilot-instructions.md`

Add to the Action Types section:
```markdown
### Action Types
- `actionable`: focus, blur, scroll, moveTo
- `clickable`: click, doubleClick, rightClick (extends actionable)
- `editable`: clear, setText, clearAndType (extends actionable)
- `draggable`: dragAndDrop, dragAndDropByOffset (extends actionable)
- `your_action`: yourAction, actionWithParams (extends actionable)
```

**Add example to README.md**:
```rust
// Using the new action
let element = page.get_actionable_element().await?;
element.your_action().await?;
element.action_with_params(42).await?;
```

### 12. Add API Documentation
Ensure the trait has comprehensive rustdoc:

```rust
/// Provides custom actions for elements.
///
/// This trait extends the basic element capabilities with specialized
/// actions for [describe use case].
///
/// # Element Types
///
/// This trait can be applied to:
/// - Basic elements
/// - Custom components
/// - Containers
///
/// # Examples
///
/// ## Basic Usage
/// ```rust
/// # use utam_core::prelude::*;
/// # async fn example() -> UtamResult<()> {
/// let element = page.get_element().await?;
/// element.your_action().await?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Parameters
/// ```rust
/// # use utam_core::prelude::*;
/// # async fn example() -> UtamResult<()> {
/// let element = page.get_element().await?;
/// let result = element.action_with_params(10).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait YourAction: Send + Sync {
    // ...
}
```

## Testing Checklist
- [ ] Trait compiles without warnings
- [ ] BaseElement implements trait correctly
- [ ] Compiler recognizes action in element types
- [ ] Code generation includes trait bounds
- [ ] Compose methods work with new actions
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] E2E tests with real browser pass
- [ ] Documentation complete

## Example Commit Message
```
feat(core): add YourAction trait

- Define YourAction trait with async methods
- Implement for BaseElement
- Add compiler support for "your_action" type
- Add codegen for trait bounds
- Add comprehensive tests
- Update documentation and examples

Closes #XXX
```

## Checklist Before PR
- [ ] All tests pass (`cargo test --all`)
- [ ] No clippy warnings (`cargo clippy --all-targets -- -D warnings`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] Examples in docs tested
- [ ] Changelog updated
- [ ] Breaking changes documented (if any)
