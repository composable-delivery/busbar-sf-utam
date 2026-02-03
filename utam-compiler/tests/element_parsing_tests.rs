//! Integration tests for element parsing and validation

use utam_compiler::ast::*;

#[test]
fn test_parse_element_with_nested_elements() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "shadow": {
            "elements": [{
                "name": "container",
                "type": "container",
                "elements": [{
                    "name": "nestedButton",
                    "type": ["clickable"],
                    "selector": {"css": ".btn"}
                }]
            }]
        }
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    assert!(page.shadow.is_some());

    let shadow = page.shadow.unwrap();
    assert_eq!(shadow.elements.len(), 1);
    
    let container = &shadow.elements[0];
    assert_eq!(container.name, "container");
    assert!(matches!(container.element_kind(), ElementKind::Container));
    
    // Check nested element
    assert_eq!(container.elements.len(), 1);
    let nested = &container.elements[0];
    assert_eq!(nested.name, "nestedButton");
    assert!(matches!(nested.element_kind(), ElementKind::Typed(_)));
}

#[test]
fn test_parse_custom_component_element() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "shadow": {
            "elements": [{
                "name": "customButton",
                "type": "utam-applications/pageObjects/components/button-component",
                "selector": {"css": ".custom-btn"}
            }]
        }
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let shadow = page.shadow.unwrap();
    let element = &shadow.elements[0];
    
    match element.element_kind() {
        ElementKind::Custom(ref comp_ref) => {
            assert_eq!(comp_ref.package, "utam-applications");
            assert_eq!(comp_ref.path, vec!["components"]);
            assert_eq!(comp_ref.name, "button-component");
            assert_eq!(comp_ref.to_rust_type(), "ButtonComponent");
        }
        _ => panic!("Expected Custom element kind"),
    }
}

#[test]
fn test_parse_frame_element() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "elements": [{
            "name": "contentFrame",
            "type": "frame",
            "selector": {"css": "iframe.content"}
        }]
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let element = &page.elements[0];
    
    assert_eq!(element.name, "contentFrame");
    assert!(matches!(element.element_kind(), ElementKind::Frame));
}

#[test]
fn test_validate_frame_no_return_all() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "elements": [{
            "name": "contentFrame",
            "type": "frame",
            "selector": {"css": "iframe"}
        }]
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let element = &page.elements[0];
    
    // Should pass validation (no returnAll)
    assert!(element.validate().is_ok());
}

#[test]
fn test_multiple_element_types() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "shadow": {
            "elements": [
                {
                    "name": "basicElement",
                    "selector": {"css": ".basic"}
                },
                {
                    "name": "clickableButton",
                    "type": ["clickable"],
                    "selector": {"css": ".btn"}
                },
                {
                    "name": "editableInput",
                    "type": ["editable", "actionable"],
                    "selector": {"css": "input"}
                },
                {
                    "name": "customComponent",
                    "type": "pkg/pageObjects/component"
                },
                {
                    "name": "containerDiv",
                    "type": "container",
                    "selector": {"css": ".container"}
                },
                {
                    "name": "iframe",
                    "type": "frame",
                    "selector": {"css": "iframe"}
                }
            ]
        }
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let shadow = page.shadow.unwrap();
    assert_eq!(shadow.elements.len(), 6);

    // Verify each element kind
    assert!(matches!(shadow.elements[0].element_kind(), ElementKind::Basic));
    assert!(matches!(shadow.elements[1].element_kind(), ElementKind::Typed(_)));
    assert!(matches!(shadow.elements[2].element_kind(), ElementKind::Typed(_)));
    assert!(matches!(shadow.elements[3].element_kind(), ElementKind::Custom(_)));
    assert!(matches!(shadow.elements[4].element_kind(), ElementKind::Container));
    assert!(matches!(shadow.elements[5].element_kind(), ElementKind::Frame));
}

#[test]
fn test_validate_all_elements() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "shadow": {
            "elements": [
                {
                    "name": "validButton",
                    "type": ["clickable"],
                    "selector": {"css": ".btn"}
                },
                {
                    "name": "_privateElement",
                    "selector": {"css": ".private"}
                },
                {
                    "name": "element123",
                    "selector": {"css": ".elem"}
                }
            ]
        }
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let shadow = page.shadow.as_ref().unwrap();

    // All elements should pass validation
    for element in &shadow.elements {
        assert!(element.validate().is_ok(), 
            "Element '{}' failed validation", element.name);
    }
}

#[test]
fn test_element_filter_parsing() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "elements": [{
            "name": "filteredElement",
            "selector": {"css": ".items"},
            "filter": {
                "matcher": {
                    "type": "stringEquals",
                    "args": [{"name": "text"}]
                }
            }
        }]
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let element = &page.elements[0];
    
    assert!(element.filter.is_some());
    let filter = element.filter.as_ref().unwrap();
    assert_eq!(filter.matcher.matcher_type, "stringEquals");
}

#[test]
fn test_container_default_behavior() {
    let json = r#"{
        "root": true,
        "selector": {"css": ".app"},
        "elements": [{
            "name": "container",
            "type": "container"
        }]
    }"#;

    let page: PageObjectAst = serde_json::from_str(json).unwrap();
    let element = &page.elements[0];
    
    assert_eq!(element.name, "container");
    assert!(matches!(element.element_kind(), ElementKind::Container));
    // Note: Default selector ":scope > *:first-child" would be applied during code generation
}

#[test]
fn test_custom_component_various_formats() {
    let test_cases = vec![
        ("simple-component", "", vec![], "simple-component"),
        ("pkg/pageObjects/comp", "pkg", vec![], "comp"),
        ("pkg/pageObjects/path/comp", "pkg", vec!["path"], "comp"),
        ("pkg/pageObjects/a/b/c/comp", "pkg", vec!["a", "b", "c"], "comp"),
    ];

    for (input, expected_pkg, expected_path, expected_name) in test_cases {
        let comp_ref = CustomComponentRef::parse(input);
        assert_eq!(comp_ref.package, expected_pkg, "Failed for input: {}", input);
        assert_eq!(comp_ref.path, expected_path, "Failed for input: {}", input);
        assert_eq!(comp_ref.name, expected_name, "Failed for input: {}", input);
    }
}

#[test]
fn test_to_rust_type_conversions() {
    let test_cases = vec![
        ("button", "Button"),
        ("button-component", "ButtonComponent"),
        ("my-custom-button", "MyCustomButton"),
        ("nav-bar-item-link", "NavBarItemLink"),
        ("a-b-c-d", "ABCD"),
    ];

    for (input, expected) in test_cases {
        let comp_ref = CustomComponentRef {
            package: "test".to_string(),
            path: vec![],
            name: input.to_string(),
        };
        assert_eq!(comp_ref.to_rust_type(), expected, 
            "Failed for input: {}", input);
    }
}
