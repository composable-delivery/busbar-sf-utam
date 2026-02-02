# Prompt Template: Adding Validation Rules

## Task Overview
Add validation rules to the UTAM compiler to catch errors in page object definitions before code generation.

## Prerequisites
- Understand the UTAM JSON grammar
- Know the validation pipeline
- Understand semantic vs syntactic validation

## Types of Validation

### 1. Syntactic Validation
Handled by serde during JSON parsing. You don't need to write code for this.

**Examples**:
- JSON structure is well-formed
- Required fields are present
- Field types match schema

### 2. Semantic Validation
Custom validation logic you need to implement.

**Examples**:
- Element names are unique
- Referenced elements exist
- Method arguments match action signatures
- Selector types are compatible

## Validation Architecture

```
PageObjectAst
    ↓
ValidationContext (collects element names, custom types, etc.)
    ↓
validate_page_object()
    ├── validate_elements()
    ├── validate_methods()
    └── validate_custom_types()
        ↓
    Return Result<(), UtamError>
```

## Steps to Add Validation

### 1. Define Error Type
**File**: `utam-compiler/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtamError {
    #[error("Duplicate element name: {name}")]
    DuplicateElement { name: String },
    
    #[error("Element '{element}' references unknown element '{reference}'")]
    UnknownElementReference { element: String, reference: String },
    
    #[error("Invalid selector for element '{element}': {reason}")]
    InvalidSelector { element: String, reason: String },
    
    #[error("Method '{method}' argument mismatch: {reason}")]
    ArgumentMismatch { method: String, reason: String },
    
    // Add your new error variant
    #[error("Your validation error: {message}")]
    YourValidationError { message: String },
}
```

### 2. Create Validation Context
**File**: `utam-compiler/src/validator.rs`

```rust
use std::collections::{HashMap, HashSet};

pub struct ValidationContext {
    /// Map of element name to element definition
    pub elements: HashMap<String, ElementAst>,
    
    /// Set of available custom type paths
    pub custom_types: HashSet<String>,
    
    /// Set of defined method names
    pub methods: HashSet<String>,
    
    /// Current validation path (for error messages)
    pub path: Vec<String>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            custom_types: HashSet::new(),
            methods: HashSet::new(),
            path: Vec::new(),
        }
    }
    
    pub fn add_element(&mut self, element: &ElementAst) -> Result<(), UtamError> {
        if self.elements.contains_key(&element.name) {
            return Err(UtamError::DuplicateElement {
                name: element.name.clone(),
            });
        }
        self.elements.insert(element.name.clone(), element.clone());
        Ok(())
    }
}
```

### 3. Implement Validation Function
**File**: `utam-compiler/src/validator.rs`

```rust
pub fn validate_page_object(ast: &PageObjectAst) -> Result<(), UtamError> {
    let mut ctx = ValidationContext::new();
    
    // Phase 1: Collect all element names
    for element in &ast.elements {
        ctx.add_element(element)?;
    }
    
    // Phase 2: Validate each component
    validate_root_selector(&ast.selector)?;
    
    for element in &ast.elements {
        validate_element(element, &ctx)?;
    }
    
    for method in &ast.methods {
        validate_method(method, &ctx)?;
    }
    
    Ok(())
}
```

### 4. Implement Specific Validators

#### Element Validation
```rust
fn validate_element(element: &ElementAst, ctx: &ValidationContext) -> Result<(), UtamError> {
    // Validate name is a valid identifier
    validate_identifier(&element.name)?;
    
    // Validate selector
    validate_selector(&element.selector, &element.name)?;
    
    // Validate element type
    match &element.element_type {
        ElementType::Basic(actions) => {
            validate_actions(actions, &element.name)?;
        }
        ElementType::Custom(path) => {
            if !ctx.custom_types.contains(path) {
                return Err(UtamError::UnknownCustomType {
                    element: element.name.clone(),
                    type_path: path.clone(),
                });
            }
        }
        ElementType::Container => {
            // Containers must have valid selector
            if element.selector.is_empty() {
                return Err(UtamError::InvalidSelector {
                    element: element.name.clone(),
                    reason: "Container must have a selector".to_string(),
                });
            }
        }
        _ => {}
    }
    
    Ok(())
}
```

#### Selector Validation
```rust
fn validate_selector(selector: &SelectorAst, element_name: &str) -> Result<(), UtamError> {
    // At least one selector must be present
    if selector.css.is_none() && selector.xpath.is_none() && selector.id.is_none() {
        return Err(UtamError::InvalidSelector {
            element: element_name.to_string(),
            reason: "No selector provided (css, xpath, or id required)".to_string(),
        });
    }
    
    // Only one selector type should be present
    let count = [
        selector.css.is_some(),
        selector.xpath.is_some(),
        selector.id.is_some(),
    ].iter().filter(|&&x| x).count();
    
    if count > 1 {
        return Err(UtamError::InvalidSelector {
            element: element_name.to_string(),
            reason: "Multiple selector types provided (use only one of css, xpath, or id)".to_string(),
        });
    }
    
    // Validate CSS selector syntax (basic check)
    if let Some(css) = &selector.css {
        if css.is_empty() {
            return Err(UtamError::InvalidSelector {
                element: element_name.to_string(),
                reason: "CSS selector cannot be empty".to_string(),
            });
        }
    }
    
    Ok(())
}
```

#### Method Validation
```rust
fn validate_method(method: &MethodAst, ctx: &ValidationContext) -> Result<(), UtamError> {
    // Validate method name
    validate_identifier(&method.name)?;
    
    // Validate compose steps
    for step in &method.compose {
        validate_compose_step(step, ctx, &method.name)?;
    }
    
    // Validate arguments
    for arg in &method.args {
        validate_method_argument(arg, &method.name)?;
    }
    
    Ok(())
}

fn validate_compose_step(
    step: &ComposeStep,
    ctx: &ValidationContext,
    method_name: &str,
) -> Result<(), UtamError> {
    // Element must exist
    let element = ctx.elements.get(&step.element)
        .ok_or_else(|| UtamError::UnknownElementReference {
            element: method_name.to_string(),
            reference: step.element.clone(),
        })?;
    
    // Action must be valid for element type
    validate_action_for_element(&step.apply, element, method_name)?;
    
    // Arguments must match action signature
    validate_action_arguments(&step.apply, &step.args, method_name)?;
    
    Ok(())
}
```

#### Identifier Validation
```rust
fn validate_identifier(name: &str) -> Result<(), UtamError> {
    if name.is_empty() {
        return Err(UtamError::InvalidIdentifier {
            name: name.to_string(),
            reason: "Identifier cannot be empty".to_string(),
        });
    }
    
    // Must start with letter or underscore
    if !name.chars().next().unwrap().is_alphabetic() && name.chars().next().unwrap() != '_' {
        return Err(UtamError::InvalidIdentifier {
            name: name.to_string(),
            reason: "Identifier must start with letter or underscore".to_string(),
        });
    }
    
    // Must contain only alphanumeric or underscore
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(UtamError::InvalidIdentifier {
            name: name.to_string(),
            reason: "Identifier can only contain letters, numbers, and underscores".to_string(),
        });
    }
    
    // Cannot be a Rust keyword
    const RUST_KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
        "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
        "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];
    
    if RUST_KEYWORDS.contains(&name) {
        return Err(UtamError::InvalidIdentifier {
            name: name.to_string(),
            reason: format!("'{}' is a Rust keyword", name),
        });
    }
    
    Ok(())
}
```

### 5. Add Tests

#### Valid Case Tests
```rust
#[test]
fn test_validate_valid_page_object() {
    let ast = PageObjectAst {
        root: true,
        selector: Some(SelectorAst {
            css: Some("app-root".to_string()),
            ..Default::default()
        }),
        elements: vec![
            ElementAst {
                name: "button".to_string(),
                element_type: ElementType::Basic(vec!["clickable".to_string()]),
                selector: SelectorAst {
                    css: Some(".button".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    
    assert!(validate_page_object(&ast).is_ok());
}
```

#### Error Case Tests
```rust
#[test]
fn test_reject_duplicate_element_names() {
    let ast = PageObjectAst {
        elements: vec![
            create_element("button"),
            create_element("button"),  // Duplicate!
        ],
        ..Default::default()
    };
    
    let result = validate_page_object(&ast);
    assert!(matches!(result, Err(UtamError::DuplicateElement { .. })));
}

#[test]
fn test_reject_unknown_element_reference() {
    let ast = PageObjectAst {
        elements: vec![
            create_element("input"),
        ],
        methods: vec![
            MethodAst {
                name: "submit".to_string(),
                compose: vec![
                    ComposeStep {
                        element: "button".to_string(),  // Doesn't exist!
                        apply: "click".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    
    let result = validate_page_object(&ast);
    assert!(matches!(result, Err(UtamError::UnknownElementReference { .. })));
}

#[test]
fn test_reject_invalid_identifier() {
    let ast = PageObjectAst {
        elements: vec![
            ElementAst {
                name: "123-invalid".to_string(),  // Invalid!
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    
    let result = validate_page_object(&ast);
    assert!(result.is_err());
}

#[test]
fn test_reject_rust_keyword_as_name() {
    let ast = PageObjectAst {
        elements: vec![
            ElementAst {
                name: "return".to_string(),  // Rust keyword!
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    
    let result = validate_page_object(&ast);
    assert!(result.is_err());
}
```

### 6. Integration with Compiler

Ensure validation runs before code generation:

**File**: `utam-compiler/src/lib.rs`

```rust
pub fn compile(input: &str) -> Result<String, UtamError> {
    // Parse JSON
    let ast: PageObjectAst = serde_json::from_str(input)
        .map_err(|e| UtamError::ParseError { message: e.to_string() })?;
    
    // Validate before generating
    validate_page_object(&ast)?;
    
    // Generate code
    let code = generate_page_object(&ast);
    
    Ok(code.to_string())
}
```

## Advanced Validation Patterns

### Cross-Reference Validation
```rust
fn validate_cross_references(ast: &PageObjectAst) -> Result<(), UtamError> {
    let mut graph = HashMap::new();
    
    // Build dependency graph
    for element in &ast.elements {
        if let ElementType::Custom(ref path) = element.element_type {
            graph.entry(&element.name)
                .or_insert_with(Vec::new)
                .push(path);
        }
    }
    
    // Check for cycles
    detect_cycles(&graph)?;
    
    Ok(())
}
```

### Contextual Validation
```rust
fn validate_in_shadow_context(element: &ElementAst, is_shadow: bool) -> Result<(), UtamError> {
    // Some features only work in shadow DOM
    if element.pierce_shadow && !is_shadow {
        return Err(UtamError::InvalidContext {
            element: element.name.clone(),
            reason: "pierce_shadow only valid in shadow DOM".to_string(),
        });
    }
    Ok(())
}
```

## Error Message Best Practices

1. **Be specific**: Explain exactly what's wrong
2. **Suggest fix**: Tell user how to fix it
3. **Show location**: Include element/method name in context

```rust
// ❌ Bad
#[error("Invalid element")]
InvalidElement,

// ✓ Good
#[error("Element '{element}' has invalid selector: {reason}. Did you mean to use 'css' instead of 'selector'?")]
InvalidSelector { element: String, reason: String },
```

## Testing Checklist

- [ ] Test valid cases pass
- [ ] Test each error case
- [ ] Test edge cases (empty strings, special chars)
- [ ] Test error messages are helpful
- [ ] Test validation performance with large files
- [ ] Add integration tests with real JSON files

## Example Commit Message

```
feat(compiler): add validation for element references

- Validate that compose methods only reference defined elements
- Check for duplicate element names
- Ensure element types are compatible with actions
- Add comprehensive validation tests
- Improve error messages with suggestions

Closes #XXX
```

## Useful Commands

```bash
# Run validation tests
cargo test -p utam-compiler validate

# Test with sample files
cargo run -p utam-cli -- compile --validate-only testdata/

# Check error messages
cargo test test_error_message -- --nocapture
```
