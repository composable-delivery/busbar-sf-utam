# Prompt Template: Debugging Code Generation Issues

## Task Overview
Debug and fix issues with the UTAM compiler's code generation output.

## Common Code Generation Issues

### 1. Invalid Rust Syntax Generated

**Symptoms**: Generated code doesn't compile, syn::parse_file fails

**Debug Steps**:

1. **Capture the generated output**:
```rust
#[test]
fn debug_generated_code() {
    let ast = create_problematic_ast();
    let code = generate_page_object(&ast);
    
    // Print raw output
    println!("Generated code:\n{}", code.to_string());
    
    // Try to parse
    match syn::parse_file(&code.to_string()) {
        Ok(_) => println!("✓ Valid syntax"),
        Err(e) => println!("✗ Parse error: {}", e),
    }
}
```

2. **Format and inspect**:
```rust
let formatted = prettyplease::unparse(&syn::parse_file(&code.to_string()).unwrap());
std::fs::write("/tmp/generated.rs", formatted).unwrap();
```

3. **Run rustc directly**:
```bash
rustc --crate-type lib /tmp/generated.rs 2>&1 | head -20
```

**Common Fixes**:
- Missing commas in quote! macros
- Incorrect token spacing
- Forgetting semicolons
- Wrong quote! delimiters

**Example Fix**:
```rust
// ❌ Wrong
quote! {
    fn method() {
        statement1()
        statement2()
    }
}

// ✓ Correct
quote! {
    fn method() {
        statement1();
        statement2();
    }
}
```

### 2. Incorrect Trait Bounds

**Symptoms**: Generated code has trait resolution errors

**Debug Steps**:

1. **Check trait hierarchy**:
```rust
fn verify_trait_bounds(element_type: &ElementType) {
    let bounds = generate_trait_bounds(element_type);
    println!("Generated bounds: {}", quote!(#(#bounds)+*));
}
```

2. **Verify trait requirements**:
```rust
// Clickable extends Actionable, so both should be included
let bounds = vec![quote!(Actionable), quote!(Clickable)];
```

**Common Fixes**:
- Include parent trait bounds
- Add Send + Sync bounds
- Use correct lifetime parameters

**Example Fix**:
```rust
// ❌ Wrong
quote! {
    impl Clickable for Element { }
}

// ✓ Correct
quote! {
    impl Clickable for Element where Element: Actionable + Send + Sync { }
}
```

### 3. Incorrect Async Code Generation

**Symptoms**: Future not implemented, .await syntax errors

**Debug Steps**:

1. **Check async-trait usage**:
```rust
// Must use #[async_trait] for trait impls
#[async_trait]
impl PageObject for GeneratedPage {
    async fn load(driver: &WebDriver) -> UtamResult<Self> {
        // ...
    }
}
```

2. **Verify async blocks**:
```rust
// Ensure all async operations have .await
quote! {
    async fn method(&self) -> UtamResult<()> {
        self.element.click().await?;  // Don't forget .await
        Ok(())
    }
}
```

**Common Fixes**:
- Add #[async_trait] attribute
- Include .await on all async calls
- Return UtamResult instead of Result

### 4. Wrong Variable Scoping

**Symptoms**: Variables not in scope, lifetime issues

**Debug Steps**:

1. **Trace variable generation**:
```rust
fn debug_variable_scope() {
    let mut vars = Vec::new();
    
    for step in compose_steps {
        let var_name = generate_temp_var();
        vars.push(var_name.clone());
        println!("Step {} uses var: {}", step.element, var_name);
    }
}
```

2. **Check variable usage**:
```rust
quote! {
    let elem = self.get_element().await?;
    elem.click().await?;  // elem must still be in scope
}
```

**Common Fixes**:
- Avoid unnecessary nested scopes
- Use correct lifetime annotations
- Store intermediate results appropriately

### 5. Missing Imports

**Symptoms**: Unresolved imports in generated code

**Debug Steps**:

1. **List required imports**:
```rust
fn collect_required_imports(ast: &PageObjectAst) -> Vec<TokenStream> {
    let mut imports = vec![
        quote!(use utam_core::prelude::*),
    ];
    
    for element in &ast.elements {
        if let ElementType::Custom(path) = &element.element_type {
            let import = generate_custom_import(path);
            imports.push(import);
        }
    }
    
    imports
}
```

2. **Verify import paths**:
```rust
// For custom type "package/pageObjects/component"
// Should generate: use crate::package::page_objects::Component;
```

**Common Fixes**:
- Convert JSON paths to Rust module paths
- Add necessary re-exports
- Include trait imports

### 6. Incorrect Method Signatures

**Symptoms**: Method signature doesn't match trait/usage

**Debug Steps**:

1. **Compare signatures**:
```rust
// Expected trait signature
async fn click(&self) -> UtamResult<()>;

// Generated implementation
async fn click(&self) -> UtamResult<()> { /* ... */ }  // Must match exactly
```

2. **Check parameter types**:
```rust
fn verify_method_params(method: &MethodAst) {
    for arg in &method.args {
        let rust_type = map_json_type_to_rust(&arg.type_name);
        println!("Param {} -> {}", arg.name, quote!(#rust_type));
    }
}
```

**Common Fixes**:
- Match trait signature exactly
- Convert JSON types correctly
- Use proper reference types (&self vs self)

## Debugging Techniques

### 1. Pretty Print TokenStream
```rust
fn pretty_print_tokens(tokens: &TokenStream) {
    let file = syn::parse_file(&tokens.to_string())
        .expect("Should parse as file");
    let formatted = prettyplease::unparse(&file);
    println!("{}", formatted);
}
```

### 2. Write Generated Code to File
```rust
#[test]
fn debug_to_file() {
    let code = generate_page_object(&ast);
    std::fs::write("/tmp/debug.rs", code.to_string()).unwrap();
    
    // Then: rustc --crate-type lib /tmp/debug.rs
}
```

### 3. Use Snapshot Diffs
```rust
#[test]
fn debug_with_snapshot() {
    let code = generate_page_object(&ast);
    insta::assert_snapshot!("debug", prettyplease::unparse(
        &syn::parse_file(&code.to_string()).unwrap()
    ));
}
// Run: cargo insta review
```

### 4. Add Debug Prints in Generator
```rust
fn generate_element_getter(element: &ElementAst) -> TokenStream {
    eprintln!("Generating getter for: {}", element.name);
    eprintln!("  Type: {:?}", element.element_type);
    eprintln!("  Selector: {:?}", element.selector);
    
    let tokens = quote! { /* ... */ };
    eprintln!("  Generated: {}", tokens);
    tokens
}
```

### 5. Test Individual Generators
```rust
#[test]
fn test_selector_generation() {
    let selector = SelectorAst {
        css: Some(".button".to_string()),
        ..Default::default()
    };
    let code = generate_selector(&selector);
    assert_eq!(code.to_string(), "By :: Css ( \".button\" )");
}
```

## Common quote! Macro Patterns

### Interpolating Values
```rust
let name = format_ident!("my_method");
let value = "string value";
let number = 42;

quote! {
    fn #name() -> i32 {
        println!(#value);
        #number
    }
}
```

### Iterating with #()*
```rust
let items = vec!["a", "b", "c"];
quote! {
    vec![#(#items),*]
}
// Generates: vec!["a", "b", "c"]
```

### Conditional Generation
```rust
let is_public = true;
let visibility = if is_public { quote!(pub) } else { quote!() };

quote! {
    #visibility fn method() {}
}
```

### Nested Quotes
```rust
let methods = elements.iter().map(|e| {
    let name = format_ident!("get_{}", e.name);
    quote! {
        pub async fn #name(&self) -> UtamResult<Element> {
            // ...
        }
    }
});

quote! {
    impl Page {
        #(#methods)*
    }
}
```

## Regression Testing

After fixing a bug, add a regression test:

```rust
#[test]
fn test_fix_issue_123() {
    // Reproduce the bug scenario
    let ast = create_problematic_ast_from_issue_123();
    
    // Should not panic or generate invalid code
    let code = generate_page_object(&ast);
    
    // Verify the fix
    assert!(syn::parse_file(&code.to_string()).is_ok());
    
    // Snapshot the expected output
    insta::assert_snapshot!("issue_123_fix", 
        prettyplease::unparse(&syn::parse_file(&code.to_string()).unwrap())
    );
}
```

## Checklist for Code Generation Bugs

- [ ] Captured generated code output
- [ ] Tested parsing with syn::parse_file
- [ ] Formatted with prettyplease for readability
- [ ] Ran rustc on generated code
- [ ] Checked trait bounds and constraints
- [ ] Verified async/await usage
- [ ] Validated imports are correct
- [ ] Tested with edge cases
- [ ] Added regression test
- [ ] Updated snapshots if needed

## Example Commit Message

```
fix(compiler): correct trait bounds for custom elements

Previously, custom elements weren't including parent trait bounds,
causing trait resolution errors in generated code.

- Add parent trait bounds to generated impls
- Fix import path generation for custom types
- Add regression test for issue #123
- Update snapshots

Fixes #123
```

## Useful Commands

```bash
# Test code generation
cargo test -p utam-compiler generate

# Update all snapshots
cargo insta test --review

# Check generated code compiles
cargo run -p utam-cli -- compile --check testdata/

# Debug specific test with output
cargo test test_name -- --nocapture

# Format generated code for inspection
echo 'code' | rustfmt
```
