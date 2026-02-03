//! Tests for compose method and statement parsing
//!
//! These tests validate the compose method parsing functionality including:
//! - Method signature parsing (name, args, return type)
//! - Compose statement parsing
//! - Statement chaining logic
//! - Element reference resolution
//! - Argument reference handling
//! - Matcher parsing and validation

use utam_compiler::ast::*;
use utam_compiler::codegen::*;

#[test]
fn test_parse_simple_compose_method() {
    let json = include_str!("../../testdata/compose/simple-method.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    assert_eq!(page.methods.len(), 1);
    let method = &page.methods[0];
    assert_eq!(method.name, "login");
    assert_eq!(method.compose.len(), 3);
}

#[test]
fn test_parse_chained_compose_method() {
    let json = include_str!("../../testdata/compose/chained-method.utam.json");
    let page: PageObjectAst = serde_json::from_str(json).unwrap();

    assert_eq!(page.methods.len(), 2);
    assert_eq!(page.methods[0].name, "performSearch");
    assert_eq!(page.methods[1].name, "searchAndGetResults");
}

#[test]
fn test_method_signature_with_return_type() {
    let method = MethodAst {
        name: "getTitle".to_string(),
        description: None,
        args: vec![],
        compose: vec![],
        return_type: Some("string".to_string()),
        return_all: false,
    };

    let sig = method.rust_signature();
    assert_eq!(sig.name, "get_title");
    assert_eq!(sig.return_type, "String");
    assert!(sig.is_async);
}

#[test]
fn test_method_signature_with_multiple_args() {
    let method = MethodAst {
        name: "submitForm".to_string(),
        description: None,
        args: vec![
            MethodArgAst {
                name: "firstName".to_string(),
                arg_type: "string".to_string(),
            },
            MethodArgAst {
                name: "lastName".to_string(),
                arg_type: "string".to_string(),
            },
            MethodArgAst {
                name: "age".to_string(),
                arg_type: "number".to_string(),
            },
            MethodArgAst {
                name: "isActive".to_string(),
                arg_type: "boolean".to_string(),
            },
        ],
        compose: vec![],
        return_type: None,
        return_all: false,
    };

    let sig = method.rust_signature();
    assert_eq!(sig.name, "submit_form");
    assert_eq!(sig.args.len(), 4);
    assert_eq!(sig.args[0].rust_type, "String");
    assert_eq!(sig.args[1].rust_type, "String");
    assert_eq!(sig.args[2].rust_type, "i64");
    assert_eq!(sig.args[3].rust_type, "bool");
}

#[test]
fn test_resolve_element_reference() {
    let statements = vec![
        ComposeStatementAst {
            element: Some("usernameInput".to_string()),
            apply: Some("clearAndType".to_string()),
            args: vec![ComposeArgAst::Named {
                name: "username".to_string(),
                arg_type: "argumentReference".to_string(),
            }],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        },
    ];

    let method_args = vec![MethodArgAst {
        name: "username".to_string(),
        arg_type: "string".to_string(),
    }];

    let compiled = compile_compose_statements(&statements, &method_args, &[]).unwrap();
    assert_eq!(compiled.len(), 1);

    match &compiled[0].kind {
        StatementKind::ApplyAction { action, args } => {
            assert_eq!(action, "clearAndType");
            assert_eq!(args.len(), 1);
            match &args[0] {
                CompiledArg::ArgumentReference(name) => {
                    assert_eq!(name, "username");
                }
                _ => panic!("Expected ArgumentReference"),
            }
        }
        _ => panic!("Expected ApplyAction"),
    }
}

#[test]
fn test_handle_argument_reference_not_found() {
    let statements = vec![ComposeStatementAst {
        element: Some("usernameInput".to_string()),
        apply: Some("clearAndType".to_string()),
        args: vec![ComposeArgAst::Named {
            name: "nonExistent".to_string(),
            arg_type: "argumentReference".to_string(),
        }],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: None,
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let method_args = vec![MethodArgAst {
        name: "username".to_string(),
        arg_type: "string".to_string(),
    }];

    let result = compile_compose_statements(&statements, &method_args, &[]);
    assert!(result.is_err());
}

#[test]
fn test_chained_statements() {
    let statements = vec![
        ComposeStatementAst {
            element: Some("searchBox".to_string()),
            apply: Some("clearAndType".to_string()),
            args: vec![ComposeArgAst::Value(serde_json::json!("test query"))],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        },
        ComposeStatementAst {
            element: None,
            apply: Some("click".to_string()),
            args: vec![],
            chain: true,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        },
    ];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    assert_eq!(compiled.len(), 2);

    match &compiled[0].kind {
        StatementKind::ApplyAction { action, .. } => {
            assert_eq!(action, "clearAndType");
        }
        _ => panic!("Expected ApplyAction"),
    }

    match &compiled[1].kind {
        StatementKind::ChainAction { action, .. } => {
            assert_eq!(action, "click");
        }
        _ => panic!("Expected ChainAction"),
    }
}

#[test]
fn test_matcher_contains() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "contains".to_string(),
            args: vec![ComposeArgAst::Value(serde_json::json!("test"))],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    assert_eq!(compiled.len(), 1);

    match &compiled[0].kind {
        StatementKind::MatcherAssert { matcher, value } => {
            assert_eq!(*matcher, MatcherKind::Contains);
            match value {
                CompiledArg::Literal(s) => assert_eq!(s, "\"test\""),
                _ => panic!("Expected Literal"),
            }
        }
        _ => panic!("Expected MatcherAssert"),
    }
}

#[test]
fn test_matcher_equals() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "equals".to_string(),
            args: vec![ComposeArgAst::Value(serde_json::json!("exact value"))],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    match &compiled[0].kind {
        StatementKind::MatcherAssert { matcher, .. } => {
            assert_eq!(*matcher, MatcherKind::Equals);
        }
        _ => panic!("Expected MatcherAssert"),
    }
}

#[test]
fn test_matcher_starts_with() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "startsWith".to_string(),
            args: vec![ComposeArgAst::Value(serde_json::json!("prefix"))],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    match &compiled[0].kind {
        StatementKind::MatcherAssert { matcher, .. } => {
            assert_eq!(*matcher, MatcherKind::StartsWith);
        }
        _ => panic!("Expected MatcherAssert"),
    }
}

#[test]
fn test_matcher_ends_with() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "endsWith".to_string(),
            args: vec![ComposeArgAst::Value(serde_json::json!("suffix"))],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    match &compiled[0].kind {
        StatementKind::MatcherAssert { matcher, .. } => {
            assert_eq!(*matcher, MatcherKind::EndsWith);
        }
        _ => panic!("Expected MatcherAssert"),
    }
}

#[test]
fn test_matcher_invalid_type() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "invalidMatcher".to_string(),
            args: vec![ComposeArgAst::Value(serde_json::json!("test"))],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let result = compile_compose_statements(&statements, &[], &[]);
    assert!(result.is_err());
}

#[test]
fn test_matcher_missing_argument() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: None,
        args: vec![],
        chain: false,
        return_type: None,
        return_all: false,
        matcher: Some(MatcherAst {
            matcher_type: "contains".to_string(),
            args: vec![],
        }),
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let result = compile_compose_statements(&statements, &[], &[]);
    assert!(result.is_err());
}

#[test]
fn test_literal_arguments_types() {
    let statements = vec![
        ComposeStatementAst {
            element: Some("input".to_string()),
            apply: Some("setText".to_string()),
            args: vec![
                ComposeArgAst::Value(serde_json::json!("string value")),
                ComposeArgAst::Value(serde_json::json!(42)),
                ComposeArgAst::Value(serde_json::json!(true)),
            ],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        },
    ];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    match &compiled[0].kind {
        StatementKind::ApplyAction { args, .. } => {
            assert_eq!(args.len(), 3);
            assert!(matches!(args[0], CompiledArg::Literal(_)));
            assert!(matches!(args[1], CompiledArg::Literal(_)));
            assert!(matches!(args[2], CompiledArg::Literal(_)));
        }
        _ => panic!("Expected ApplyAction"),
    }
}

#[test]
fn test_utam_type_to_rust_locator() {
    assert_eq!(utam_type_to_rust("locator"), "By");
}

#[test]
fn test_utam_type_to_rust_function() {
    assert_eq!(utam_type_to_rust("function"), "/* predicate */");
}

#[test]
fn test_utam_type_to_rust_unknown() {
    // Unknown types are passed through as-is
    assert_eq!(utam_type_to_rust("customType"), "customType");
}

#[test]
fn test_snake_case_edge_cases() {
    assert_eq!(to_snake_case("A"), "a");
    assert_eq!(to_snake_case("ABC"), "a_b_c");
    assert_eq!(to_snake_case("getHTMLElement"), "get_h_t_m_l_element");
}

#[test]
fn test_pascal_case_edge_cases() {
    assert_eq!(to_pascal_case("a"), "A");
    assert_eq!(to_pascal_case("A"), "A");
    assert_eq!(to_pascal_case("a-b-c"), "ABC");
    assert_eq!(to_pascal_case("a_b_c"), "ABC");
}

#[test]
fn test_statement_with_return_type() {
    let statements = vec![ComposeStatementAst {
        element: Some("resultList".to_string()),
        apply: None,
        args: vec![],
        chain: false,
        return_type: Some("string".to_string()),
        return_all: false,
        matcher: None,
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
    assert_eq!(compiled[0].return_type, Some("string".to_string()));
}

#[test]
fn test_invalid_statement_no_element_no_matcher() {
    let statements = vec![ComposeStatementAst {
        element: None,
        apply: Some("click".to_string()),
        args: vec![],
        chain: false, // Not chaining, no element, no matcher
        return_type: None,
        return_all: false,
        matcher: None,
        apply_external: None,
        filter: None,
        return_element: false,
        predicate: None,
    }];

    let result = compile_compose_statements(&statements, &[], &[]);
    assert!(result.is_err());
}
