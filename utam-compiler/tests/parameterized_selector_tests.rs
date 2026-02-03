//! Integration tests for parameterized selector parsing

use utam_compiler::ast::*;
use utam_compiler::codegen::generate_selector_code;

#[test]
fn test_parse_string_parameter_selector() {
    let json = include_str!("../../testdata/parameterized/string-parameter.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    assert!(page.root);
    assert!(page.shadow.is_some());

    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 1);
        let element = &shadow.elements[0];
        assert_eq!(element.name, "dynamicButton");

        if let Some(selector) = &element.selector {
            assert_eq!(selector.css, Some("button[data-id='%s']".to_string()));
            assert_eq!(selector.args.len(), 1);
            assert_eq!(selector.args[0].name, "buttonId");
            assert_eq!(selector.args[0].arg_type, "string");

            // Test selector methods
            assert!(selector.has_parameters());
            assert_eq!(selector.count_placeholders(), 1);
            assert!(selector.validate().is_ok());
        } else {
            panic!("Element should have a selector");
        }
    }
}

#[test]
fn test_parse_number_parameter_selector() {
    let json = include_str!("../../testdata/parameterized/number-parameter.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    assert!(page.root);
    assert!(page.shadow.is_some());

    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 1);
        let element = &shadow.elements[0];
        assert_eq!(element.name, "nthItem");

        if let Some(selector) = &element.selector {
            assert_eq!(selector.css, Some("li:nth-child(%d)".to_string()));
            assert_eq!(selector.args.len(), 1);
            assert_eq!(selector.args[0].name, "index");
            assert_eq!(selector.args[0].arg_type, "number");

            // Test selector methods
            assert!(selector.has_parameters());
            assert_eq!(selector.count_placeholders(), 1);
            assert!(selector.validate().is_ok());
        } else {
            panic!("Element should have a selector");
        }
    }
}

#[test]
fn test_parse_multiple_parameters_selector() {
    let json = include_str!("../../testdata/parameterized/multiple-parameters.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    assert!(page.root);
    assert!(page.shadow.is_some());

    if let Some(shadow) = page.shadow {
        assert_eq!(shadow.elements.len(), 1);
        let element = &shadow.elements[0];
        assert_eq!(element.name, "dynamicInput");

        if let Some(selector) = &element.selector {
            assert_eq!(
                selector.css,
                Some("input[data-type='%s'][data-index='%d']".to_string())
            );
            assert_eq!(selector.args.len(), 2);
            assert_eq!(selector.args[0].name, "inputType");
            assert_eq!(selector.args[0].arg_type, "string");
            assert_eq!(selector.args[1].name, "position");
            assert_eq!(selector.args[1].arg_type, "number");

            // Test selector methods
            assert!(selector.has_parameters());
            assert_eq!(selector.count_placeholders(), 2);
            assert!(selector.validate().is_ok());
        } else {
            panic!("Element should have a selector");
        }
    }
}

#[test]
fn test_codegen_for_parameterized_selector() {
    let json = include_str!("../../testdata/parameterized/string-parameter.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    if let Some(shadow) = page.shadow {
        let element = &shadow.elements[0];
        if let Some(selector) = &element.selector {
            let code = generate_selector_code(selector);
            let code_str = code.to_string();

            // Verify the generated code contains the expected elements
            assert!(code_str.contains("thirtyfour :: By :: Css"));
            assert!(code_str.contains("format !"));
            assert!(code_str.contains("buttonId"));
            assert!(code_str.contains("{}"));
        }
    }
}

#[test]
fn test_validate_parameter_mismatch() {
    // Create a selector with mismatched parameter count
    let selector = SelectorAst {
        css: Some("button[data-id='%s']".to_string()),
        accessid: None,
        classchain: None,
        uiautomator: None,
        args: vec![
            SelectorArgAst {
                name: "id1".to_string(),
                arg_type: "string".to_string(),
            },
            SelectorArgAst {
                name: "id2".to_string(),
                arg_type: "string".to_string(),
            },
        ],
        return_all: false,
    };

    let result = selector.validate();
    assert!(result.is_err());
}
