# Prompt Template: Adding a Compiler Feature

## Task Overview
Add a new feature to the UTAM compiler that enhances the JSON grammar or code generation capabilities.

## Prerequisites
- Understand the compiler pipeline: Parse → Validate → CodeGen
- Know the AST structure
- Understand quote! macro for code generation

## Common Compiler Features

### 1. Adding a New JSON Property

**Example**: Adding `timeout` property to elements

#### Update AST
**File**: `utam-compiler/src/ast/element.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAst {
    pub name: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub selector: SelectorAst,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub timeout: Option<u64>,  // Add new field
    // ... other fields
}
```

#### Update Validation
**File**: `utam-compiler/src/validator.rs`

```rust
fn validate_element(element: &ElementAst) -> Result<(), UtamError> {
    if let Some(timeout) = element.timeout {
        if timeout == 0 {
            return Err(UtamError::InvalidTimeout {
                element: element.name.clone(),
                reason: "Timeout must be greater than 0".to_string(),
            });
        }
    }
    Ok(())
}
```

#### Update Code Generation
**File**: `utam-compiler/src/codegen/element.rs`

```rust
fn generate_element_with_timeout(element: &ElementAst) -> TokenStream {
    let name = &element.name;
    let selector = &element.selector;
    let timeout = element.timeout.unwrap_or(30);
    
    quote! {
        async fn #name(&self) -> UtamResult<BaseElement> {
            let elem = self.root
                .find_with_timeout(By::Css(#selector), Duration::from_secs(#timeout))
                .await?;
            Ok(BaseElement::new(elem))
        }
    }
}
```

#### Add Tests
```rust
#[test]
fn test_parse_element_with_timeout() {
    let json = r#"{
        "name": "elem",
        "type": ["clickable"],
        "selector": {"css": ".elem"},
        "timeout": 10
    }"#;
    let element: ElementAst = serde_json::from_str(json).unwrap();
    assert_eq!(element.timeout, Some(10));
}
```

### 2. Adding a New Selector Type

**Example**: Adding XPath selector support

#### Update AST
**File**: `utam-compiler/src/ast/selector.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorAst {
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub xpath: Option<String>,  // Add XPath
    #[serde(default)]
    pub id: Option<String>,
}
```

#### Update Validation
```rust
fn validate_selector(selector: &SelectorAst) -> Result<(), UtamError> {
    let count = [
        selector.css.is_some(),
        selector.xpath.is_some(),
        selector.id.is_some(),
    ].iter().filter(|&&x| x).count();
    
    if count == 0 {
        return Err(UtamError::MissingSelector);
    }
    if count > 1 {
        return Err(UtamError::MultipleSelectors);
    }
    Ok(())
}
```

#### Update Code Generation
```rust
fn generate_by_selector(selector: &SelectorAst) -> UtamResult<TokenStream> {
    if let Some(css) = &selector.css {
        Ok(quote!(By::Css(#css)))
    } else if let Some(xpath) = &selector.xpath {
        Ok(quote!(By::XPath(#xpath)))
    } else if let Some(id) = &selector.id {
        Ok(quote!(By::Id(#id)))
    } else {
        Err(UtamError::MissingSelector)
    }
}
```

### 3. Adding Method Return Types

**Example**: Supporting methods that return values

#### Update AST
**File**: `utam-compiler/src/ast/method.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodAst {
    pub name: String,
    pub compose: Vec<ComposeStep>,
    #[serde(default)]
    pub returns: Option<ReturnType>,  // Add return type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReturnType {
    Simple(String),  // "string", "boolean", "number"
    Element(String), // Element reference
}
```

#### Update Code Generation
```rust
fn generate_method_with_return(method: &MethodAst) -> TokenStream {
    let name = format_ident!("{}", method.name);
    let statements = generate_compose_statements(&method.compose);
    
    let return_type = if let Some(ret) = &method.returns {
        match ret {
            ReturnType::Simple(s) if s == "string" => quote!(String),
            ReturnType::Simple(s) if s == "boolean" => quote!(bool),
            ReturnType::Element(e) => {
                let elem = format_ident!("{}", e);
                quote!(#elem)
            },
            _ => quote!(()),
        }
    } else {
        quote!(())
    };
    
    quote! {
        pub async fn #name(&self) -> UtamResult<#return_type> {
            #(#statements)*
            // Return appropriate value
        }
    }
}
```

### 4. Adding Conditional Logic in Compose

**Example**: Supporting if-else in compose methods

#### Update AST
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeStep {
    Action(ActionStep),
    Conditional(ConditionalStep),  // Add conditional
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStep {
    pub condition: ConditionAst,
    pub then: Vec<ComposeStep>,
    pub otherwise: Option<Vec<ComposeStep>>,
}
```

#### Update Code Generation
```rust
fn generate_conditional(cond: &ConditionalStep) -> TokenStream {
    let condition = generate_condition(&cond.condition);
    let then_steps = cond.then.iter().map(generate_compose_step);
    let else_steps = cond.otherwise.as_ref().map(|steps| {
        let stmts = steps.iter().map(generate_compose_step);
        quote! { #(#stmts)* }
    });
    
    if let Some(else_block) = else_steps {
        quote! {
            if #condition {
                #(#then_steps)*
            } else {
                #else_block
            }
        }
    } else {
        quote! {
            if #condition {
                #(#then_steps)*
            }
        }
    }
}
```

## Testing Strategy

### 1. Parser Tests
```rust
#[test]
fn test_parse_new_feature() {
    let json = include_str!("../testdata/new-feature.utam.json");
    let ast: PageObjectAst = serde_json::from_str(json).unwrap();
    // Verify AST structure
}
```

### 2. Validation Tests
```rust
#[test]
fn test_validate_new_feature() {
    let ast = create_test_ast_with_feature();
    assert!(validate_page_object(&ast).is_ok());
}

#[test]
fn test_reject_invalid_feature() {
    let ast = create_invalid_ast();
    assert!(validate_page_object(&ast).is_err());
}
```

### 3. Code Generation Tests
```rust
#[test]
fn test_generate_with_new_feature() {
    let ast = create_test_ast();
    let code = generate_page_object(&ast);
    
    // Verify it parses
    let syntax = syn::parse_file(&code.to_string()).unwrap();
    
    // Snapshot test
    insta::assert_snapshot!("new_feature", prettyplease::unparse(&syntax));
}
```

### 4. Integration Tests
```rust
#[test]
fn test_compile_with_new_feature() {
    let input = include_str!("../testdata/new-feature.utam.json");
    let output = compile(input).unwrap();
    
    // Verify generated code compiles
    assert!(syn::parse_file(&output).is_ok());
}
```

## Documentation Requirements

### 1. Update Grammar Docs
Document the new feature in `.github/copilot-instructions.md`:

```markdown
### New Feature

Description of what the feature does.

**Syntax**:
```json
{
  "propertyName": "value"
}
```

**Example**:
```json
{
  "name": "example",
  "newProperty": "value"
}
```
```

### 2. Add Examples
Create examples in `testdata/examples/`:

```json
{
  "description": "Example using new feature",
  "root": true,
  "selector": {"css": "root"},
  "newFeature": {
    "property": "value"
  }
}
```

### 3. Update README
Add usage examples to README.md showing the new feature in action.

## Common Pitfalls

1. **Forgetting serde attributes**: Use `#[serde(default)]`, `#[serde(rename = "...")]`
2. **Not validating edge cases**: Test boundary conditions
3. **Breaking backward compatibility**: Ensure old JSON still works
4. **Missing error messages**: Provide helpful validation errors
5. **Not testing generated code**: Always verify code compiles

## Steps Checklist

- [ ] Update AST types
- [ ] Add serde attributes
- [ ] Implement validation
- [ ] Update code generation
- [ ] Add parser tests
- [ ] Add validation tests
- [ ] Add codegen tests
- [ ] Add integration tests
- [ ] Create test data files
- [ ] Update documentation
- [ ] Add examples
- [ ] Test backward compatibility
- [ ] Update changelog

## Example Commit Message

```
feat(compiler): add support for [feature name]

- Add [property] to ElementAst/MethodAst
- Implement validation for [feature]
- Generate code with [capability]
- Add comprehensive tests
- Update documentation with examples

Closes #XXX
```

## Useful Commands

```bash
# Run parser tests
cargo test -p utam-compiler --lib

# Run integration tests
cargo test -p utam-compiler --test '*'

# Update snapshots
cargo insta test
cargo insta review

# Check generated code compiles
cargo check --all-targets

# Verify examples
cargo run -p utam-cli -- compile testdata/examples/
```
