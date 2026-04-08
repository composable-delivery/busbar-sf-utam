//! Integration tests for utam-runtime
//!
//! These tests exercise the full pipeline: parse UTAM JSON → build element
//! index → introspect methods/elements → verify selector resolution.
//! They don't require a live browser (no WebDriver); they test everything
//! that can be tested without one.

use std::collections::HashMap;
use std::path::PathBuf;

use utam_compiler::ast::*;
use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::*;
use utam_runtime::registry::PageObjectRegistry;

/// Load a test fixture from the testdata directory.
fn load_fixture(path: &str) -> PageObjectAst {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    let json = std::fs::read_to_string(&full)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {}", full.display()));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("Failed to parse {path}: {e}"))
}

// ---------------------------------------------------------------------------
// Fixture parsing + introspection
// ---------------------------------------------------------------------------

#[test]
fn test_simple_compose_method_introspection() {
    let ast = load_fixture("testdata/compose/simple-method.utam.json");

    assert!(ast.root);
    assert_eq!(ast.selector.as_ref().unwrap().css.as_deref(), Some("login-form"));

    // Should have shadow elements
    let shadow = ast.shadow.as_ref().expect("should have shadow");
    assert_eq!(shadow.elements.len(), 3);

    // Method introspection
    assert_eq!(ast.methods.len(), 1);
    let login = &ast.methods[0];
    assert_eq!(login.name, "login");
    assert_eq!(login.compose.len(), 3);

    // First compose step: usernameInput.clearAndType(username)
    let step1 = &login.compose[0];
    assert_eq!(step1.element.as_deref(), Some("usernameInput"));
    assert_eq!(step1.apply.as_deref(), Some("clearAndType"));
    assert_eq!(step1.args.len(), 1);
}

#[test]
fn test_chained_method_introspection() {
    let ast = load_fixture("testdata/compose/chained-method.utam.json");

    assert_eq!(ast.methods.len(), 2);

    let search = &ast.methods[0];
    assert_eq!(search.name, "performSearch");
    assert_eq!(search.compose.len(), 2);

    // Second method calls the first, then returns an element
    let search_and_get = &ast.methods[1];
    assert_eq!(search_and_get.name, "searchAndGetResults");
    assert_eq!(search_and_get.compose.len(), 2);

    // First step is a self-referential call (no element, just apply)
    let step1 = &search_and_get.compose[0];
    assert!(step1.element.is_none());
    assert_eq!(step1.apply.as_deref(), Some("performSearch"));

    // Second step returns an element
    let step2 = &search_and_get.compose[1];
    assert_eq!(step2.element.as_deref(), Some("resultsList"));
    assert!(step2.return_element);
}

#[test]
fn test_filter_method_introspection() {
    let ast = load_fixture("testdata/compose/filter-method.utam.json");

    let method = &ast.methods[0];
    assert_eq!(method.name, "getActiveTodos");

    let step = &method.compose[0];
    assert_eq!(step.element.as_deref(), Some("todoItems"));
    assert!(step.filter.is_some());

    let filters = step.filter.as_ref().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].matcher.matcher_type, "isVisible");
}

#[test]
fn test_salesforce_studio_app_introspection() {
    let ast = load_fixture("testdata/salesforce/salesforceStudioApp.utam.json");

    assert!(ast.root);
    assert!(ast.expose_root_element);

    // Has beforeLoad
    assert!(!ast.before_load.is_empty());
    let bl = &ast.before_load[0];
    assert_eq!(bl.apply.as_deref(), Some("waitFor"));

    // Shadow elements with deep nesting
    let shadow = ast.shadow.as_ref().expect("should have shadow");
    assert!(shadow.elements.len() >= 3);

    // Custom component types
    let studio_mat = &shadow.elements[0];
    assert_eq!(studio_mat.name, "studioMat");
    assert!(matches!(
        &studio_mat.element_type,
        Some(ElementTypeAst::CustomComponent(s)) if s.contains("salesforceStudioWelcomeMat")
    ));

    // Deeply nested elements should be in the flat index
    let index = build_element_index_from_ast(&ast);
    assert!(index.contains_key("studioMat"));
    assert!(index.contains_key("routerContainer"));
    assert!(index.contains_key("routeInterceptor"));
    assert!(index.contains_key("appLayout"));
    assert!(index.contains_key("navigationBar"));
}

#[test]
fn test_shadow_dom_fixture() {
    let ast = load_fixture("testdata/shadow-dom/shadow-root.utam.json");

    assert!(ast.root);
    let shadow = ast.shadow.as_ref().expect("should have shadow section");
    assert!(!shadow.elements.is_empty());

    let index = build_element_index_from_ast(&ast);
    // Shadow elements should be marked as in_shadow
    for (name, (_ast, in_shadow)) in &index {
        if shadow.elements.iter().any(|e| e.name == *name) {
            assert!(*in_shadow, "Element {name} should be in shadow");
        }
    }
}

#[test]
fn test_basic_element_fixtures() {
    for fixture in &[
        "testdata/basic/clickable-button.utam.json",
        "testdata/basic/editable-input.utam.json",
        "testdata/basic/simple-element.utam.json",
    ] {
        let ast = load_fixture(fixture);
        assert!(ast.root, "{fixture} should be a root page object");
        assert!(ast.selector.is_some(), "{fixture} should have a root selector");
    }
}

// ---------------------------------------------------------------------------
// Selector resolution
// ---------------------------------------------------------------------------

#[test]
fn test_selector_resolution_from_fixture() {
    let ast = load_fixture("testdata/compose/simple-method.utam.json");

    // Root selector
    let root_sel = resolve_selector(ast.selector.as_ref().unwrap(), &HashMap::new()).unwrap();
    assert!(matches!(root_sel, utam_runtime::Selector::Css(s) if s == "login-form"));

    // Element selector (from shadow)
    let shadow = ast.shadow.as_ref().unwrap();
    let username = &shadow.elements[0];
    assert_eq!(username.name, "usernameInput");
    let sel = resolve_selector(username.selector.as_ref().unwrap(), &HashMap::new()).unwrap();
    assert!(matches!(sel, utam_runtime::Selector::Css(s) if s == "input[name='username']"));
}

#[test]
fn test_parameterized_selector_resolution() {
    let ast = load_fixture("testdata/parameterized/string-parameter.utam.json");

    // Find the parameterized element
    let elem = ast
        .elements
        .iter()
        .chain(ast.shadow.iter().flat_map(|s| s.elements.iter()))
        .find(|e| e.selector.as_ref().map_or(false, |s| s.has_parameters()))
        .expect("Should have a parameterized element");

    let sel_ast = elem.selector.as_ref().unwrap();
    assert!(sel_ast.has_parameters());

    // Resolve with an argument
    let mut args = HashMap::new();
    let param_name = &sel_ast.args[0].name;
    args.insert(param_name.clone(), RuntimeValue::String("test-value".into()));

    let resolved = resolve_selector(sel_ast, &args).unwrap();
    match resolved {
        utam_runtime::Selector::Css(s) => {
            assert!(!s.contains("%s"), "Resolved selector should not contain %%s: {s}");
            assert!(s.contains("test-value"), "Should contain the argument value: {s}");
        }
        _ => panic!("Expected CSS selector"),
    }
}

// ---------------------------------------------------------------------------
// Matcher evaluation
// ---------------------------------------------------------------------------

#[test]
fn test_matcher_evaluation() {
    let val = RuntimeValue::String("Hello World".into());

    // These are internal functions tested via the module's test infrastructure
    // Just verify the RuntimeValue accessors work for matcher patterns
    assert_eq!(val.as_str().unwrap(), "Hello World");
    assert!(val.as_bool().is_err());
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_registry_loads_testdata() {
    let mut registry = PageObjectRegistry::new();
    let testdata = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../testdata");
    registry.add_search_path(&testdata);
    let count = registry.scan().unwrap();

    assert!(count >= 6, "Should load at least 6 test fixtures, got {count}");

    // Should find compose fixtures
    let compose = registry.search("compose");
    assert!(!compose.is_empty(), "Should find compose fixtures");

    // Should find shadow fixtures
    let shadow = registry.search("shadow");
    assert!(!shadow.is_empty(), "Should find shadow fixtures");
}

#[test]
fn test_registry_loads_salesforce_pageobjects() {
    let mut registry = PageObjectRegistry::new();
    let sf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../salesforce-pageobjects");
    if !sf_path.exists() {
        return; // Skip if not available
    }
    registry.add_search_path(&sf_path);
    let count = registry.scan().unwrap();

    assert!(count > 1000, "Should load 1000+ SF page objects, got {count}");

    // Verify the login page object loads and introspects correctly
    let login_matches = registry.search("login");
    assert!(!login_matches.is_empty());

    let login_name =
        login_matches.iter().find(|n| n.contains("helpers")).expect("Should find helpers/login");
    let login_ast = registry.get(login_name).unwrap();

    assert!(login_ast.root);
    assert_eq!(login_ast.methods.len(), 2);
    assert_eq!(login_ast.methods[0].name, "login");
    assert_eq!(login_ast.methods[0].compose.len(), 6);
    assert_eq!(login_ast.elements.len(), 3);

    // Verify header page object
    let header_matches = registry.search("header");
    assert!(!header_matches.is_empty(), "Should find header page objects");
    // Just verify we can load at least one header-related page object
    let header_ast = registry.get(&header_matches[0]).unwrap();
    assert!(
        header_ast.selector.is_some() || !header_ast.elements.is_empty(),
        "Header page object should have a selector or elements"
    );
}

// ---------------------------------------------------------------------------
// Helpers (re-implementing build_element_index for test visibility)
// ---------------------------------------------------------------------------

fn build_element_index_from_ast(ast: &PageObjectAst) -> HashMap<String, (ElementAst, bool)> {
    let mut index = HashMap::new();
    for elem in &ast.elements {
        collect_elements_recursive(elem, false, &mut index);
    }
    if let Some(shadow) = &ast.shadow {
        for elem in &shadow.elements {
            collect_elements_recursive(elem, true, &mut index);
        }
    }
    index
}

fn collect_elements_recursive(
    elem: &ElementAst,
    in_shadow: bool,
    index: &mut HashMap<String, (ElementAst, bool)>,
) {
    index.insert(elem.name.clone(), (elem.clone(), in_shadow));
    for child in &elem.elements {
        collect_elements_recursive(child, in_shadow, index);
    }
}
