//! Integration tests for AST deserialization
//!
//! Tests that all example UTAM JSON files can be successfully parsed into AST types.

use utam_compiler::ast::*;

#[test]
fn test_deserialize_simple_element() {
    let json = include_str!("../../testdata/basic/simple-element.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.selector.is_some());
    assert!(page.expose_root_element);
    assert_eq!(page.action_types.len(), 1);
    assert_eq!(page.action_types[0], "clickable");
}

#[test]
fn test_deserialize_clickable_button() {
    let json = include_str!("../../testdata/basic/clickable-button.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.selector.is_some());
    if let Some(selector) = page.selector {
        assert_eq!(selector.css, Some("button.submit-btn".to_string()));
    }
}

#[test]
fn test_deserialize_editable_input() {
    let json = include_str!("../../testdata/basic/editable-input.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert_eq!(page.action_types.len(), 1);
    assert_eq!(page.action_types[0], "editable");
}

#[test]
fn test_deserialize_simple_method() {
    let json = include_str!("../../testdata/compose/simple-method.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.shadow.is_some());
    
    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 3);
        assert_eq!(shadow.elements[0].name, "usernameInput");
        assert_eq!(shadow.elements[2].name, "submitButton");
        assert!(shadow.elements[2].public);
    }
    
    assert_eq!(page.methods.len(), 1);
    assert_eq!(page.methods[0].name, "login");
    assert_eq!(page.methods[0].compose.len(), 3);
}

#[test]
fn test_deserialize_chained_method() {
    let json = include_str!("../../testdata/compose/chained-method.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert_eq!(page.methods.len(), 2);
    assert_eq!(page.methods[0].name, "performSearch");
    assert_eq!(page.methods[1].name, "searchAndGetResults");
    
    // Verify the resultsList is correctly parsed as Container
    if let Some(shadow) = page.shadow {
        if let Some(results) = shadow.elements.iter().find(|e| e.name == "resultsList") {
            match &results.element_type {
                Some(ElementTypeAst::Container) => {
                    // Correct!
                }
                _ => panic!("Expected Container type for resultsList"),
            }
        } else {
            panic!("resultsList element not found");
        }
    } else {
        panic!("Shadow not found");
    }
}

#[test]
fn test_deserialize_filter_method() {
    let json = include_str!("../../testdata/compose/filter-method.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.shadow.is_some());
    
    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 2);
        assert!(shadow.elements[0].list);
    }
    
    assert_eq!(page.methods.len(), 1);
    assert_eq!(page.methods[0].name, "getActiveTodos");
}

#[test]
fn test_deserialize_shadow_root() {
    let json = include_str!("../../testdata/shadow-dom/shadow-root.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.shadow.is_some());
    
    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 1);
        assert_eq!(shadow.elements[0].name, "innerButton");
        assert!(shadow.elements[0].public);
    }
}

#[test]
fn test_deserialize_nested_shadow() {
    let json = include_str!("../../testdata/shadow-dom/nested-shadow.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.shadow.is_some());
    
    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 2);
        
        // Check custom component type
        if let Some(ElementTypeAst::CustomComponent(path)) = &shadow.elements[0].element_type {
            assert_eq!(path, "inner-component");
        } else {
            panic!("Expected CustomComponent type");
        }
        
        // Check action types
        if let Some(ElementTypeAst::ActionTypes(types)) = &shadow.elements[1].element_type {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], "clickable");
        } else {
            panic!("Expected ActionTypes");
        }
    }
}

#[test]
fn test_deserialize_salesforce_app() {
    let json = include_str!("../../testdata/salesforce/salesforceStudioApp.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(page.root);
    assert!(page.expose_root_element);
    
    // Check detailed description
    match page.description {
        Some(DescriptionAst::Detailed { text, author, .. }) => {
            assert_eq!(author, Some("Salesforce".to_string()));
            assert!(!text.is_empty());
        }
        _ => panic!("Expected Detailed description"),
    }
    
    // Check beforeLoad
    assert!(!page.before_load.is_empty());
    
    // Check shadow elements
    assert!(page.shadow.is_some());
    if let Some(shadow) = page.shadow {
        assert!(!shadow.elements.is_empty());
        
        // Verify closeButton type is correctly parsed as ActionTypes (single action type)
        if let Some(close_btn) = shadow.elements.iter().find(|e| e.name == "closeButton") {
            match &close_btn.element_type {
                Some(ElementTypeAst::ActionTypes(types)) => {
                    assert_eq!(types.len(), 1);
                    assert_eq!(types[0], "clickable");
                }
                _ => panic!("Expected ActionTypes for closeButton"),
            }
        } else {
            panic!("closeButton element not found");
        }
    }
    
    // Check methods
    assert!(!page.methods.is_empty());
}

#[test]
fn test_round_trip_simple_element() {
    let original = include_str!("../../testdata/basic/simple-element.utam.json");
    let page: PageObjectAst = serde_json::from_str(original).unwrap();
    
    // Serialize back to JSON
    let serialized = serde_json::to_string_pretty(&page).unwrap();
    
    // Deserialize again
    let page2: PageObjectAst = serde_json::from_str(&serialized).unwrap();
    
    // Verify key properties are preserved
    assert_eq!(page.root, page2.root);
    assert_eq!(page.expose_root_element, page2.expose_root_element);
    assert_eq!(page.action_types.len(), page2.action_types.len());
}

#[test]
fn test_optional_fields_default() {
    // Minimal JSON with only required fields
    let json = r#"{
        "selector": {"css": ".test"}
    }"#;
    
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    
    assert!(!page.root);
    assert!(!page.expose_root_element);
    assert_eq!(page.elements.len(), 0);
    assert_eq!(page.methods.len(), 0);
    assert_eq!(page.action_types.len(), 0);
}

#[test]
fn test_ignore_unknown_fields() {
    // JSON with extra unknown field
    let json = r#"{
        "root": true,
        "selector": {"css": ".test"},
        "unknownField": "should be ignored"
    }"#;
    
    // Should not fail with unknown field
    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    assert!(page.root);
}
