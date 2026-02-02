//! Code generation module for UTAM compiler
//!
//! This module handles transformation of AST types into Rust source code.

use crate::ast::{ComposeArgAst, ComposeStatementAst, ElementAst, MethodArgAst, MethodAst};
use crate::error::{CompilerError, CompilerResult};

/// Rust method signature
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    pub name: String,
    pub args: Vec<RustArg>,
    pub return_type: String,
    pub is_async: bool,
}

/// Rust method argument
#[derive(Debug, Clone, PartialEq)]
pub struct RustArg {
    pub name: String,
    pub rust_type: String,
}

/// Compiled argument that can be a literal value or a reference to a method argument
#[derive(Debug, Clone, PartialEq)]
pub enum CompiledArg {
    /// Literal value (e.g., "hello", 42, true)
    Literal(String),
    /// Reference to a method argument
    ArgumentReference(String),
}

/// Compiled statement ready for code generation
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledStatement {
    pub kind: StatementKind,
    pub return_type: Option<String>,
}

/// Kind of statement in a compose method
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    /// Get element: self.get_element_name().await?
    GetElement { name: String },
    /// Apply action: element.action(args).await?
    ApplyAction {
        action: String,
        args: Vec<CompiledArg>,
    },
    /// Chain from previous: prev.action(args).await?
    ChainAction {
        action: String,
        args: Vec<CompiledArg>,
    },
    /// Matcher assertion
    MatcherAssert {
        matcher: MatcherKind,
        value: CompiledArg,
    },
}

/// Matcher types for element filtering
#[derive(Debug, Clone, PartialEq)]
pub enum MatcherKind {
    Contains,
    Equals,
    StartsWith,
    EndsWith,
}

impl MethodAst {
    /// Generate a Rust method signature from the UTAM method definition
    pub fn rust_signature(&self) -> MethodSignature {
        MethodSignature {
            name: to_snake_case(&self.name),
            args: self
                .args
                .iter()
                .map(|a| RustArg {
                    name: to_snake_case(&a.name),
                    rust_type: utam_type_to_rust(&a.arg_type),
                })
                .collect(),
            return_type: self
                .return_type
                .as_ref()
                .map(|t| utam_type_to_rust(t))
                .unwrap_or_else(|| "()".to_string()),
            is_async: true,
        }
    }
}

/// Convert UTAM type string to Rust type string
pub fn utam_type_to_rust(utam_type: &str) -> String {
    match utam_type {
        "string" => "String".to_string(),
        "boolean" => "bool".to_string(),
        "number" => "i64".to_string(),
        "locator" => "By".to_string(),
        "function" => "/* predicate */".to_string(),
        t if t.contains('/') => {
            // Custom type reference - extract the last component
            let parts: Vec<&str> = t.split('/').collect();
            if let Some(last) = parts.last() {
                // Convert to PascalCase for Rust type
                to_pascal_case(last)
            } else {
                t.to_string()
            }
        }
        t => t.to_string(),
    }
}

/// Compile compose statements into executable code structure
pub fn compile_compose_statements(
    statements: &[ComposeStatementAst],
    method_args: &[MethodArgAst],
    _elements: &[ElementAst],
) -> CompilerResult<Vec<CompiledStatement>> {
    let mut compiled = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let kind = if stmt.chain && i > 0 {
            // Chain from previous result
            StatementKind::ChainAction {
                action: stmt.apply.clone().unwrap_or_default(),
                args: compile_args(&stmt.args, method_args)?,
            }
        } else if let Some(element) = &stmt.element {
            if stmt.apply.is_some() {
                StatementKind::ApplyAction {
                    action: stmt.apply.clone().unwrap(),
                    args: compile_args(&stmt.args, method_args)?,
                }
            } else {
                StatementKind::GetElement {
                    name: element.clone(),
                }
            }
        } else if let Some(matcher) = &stmt.matcher {
            // Matcher assertion
            let matcher_kind = match matcher.matcher_type.as_str() {
                "contains" => MatcherKind::Contains,
                "equals" => MatcherKind::Equals,
                "startsWith" => MatcherKind::StartsWith,
                "endsWith" => MatcherKind::EndsWith,
                _ => {
                    return Err(CompilerError::InvalidStatement(format!(
                        "Unknown matcher type: {}",
                        matcher.matcher_type
                    )))
                }
            };
            let value = if let Some(first_arg) = matcher.args.first() {
                compile_single_arg(first_arg, method_args)?
            } else {
                return Err(CompilerError::InvalidStatement(
                    "Matcher requires an argument".to_string(),
                ));
            };
            StatementKind::MatcherAssert {
                matcher: matcher_kind,
                value,
            }
        } else {
            return Err(CompilerError::InvalidStatement(format!(
                "Invalid statement at index {}",
                i
            )));
        };

        compiled.push(CompiledStatement {
            kind,
            return_type: stmt.return_type.clone(),
        });
    }

    Ok(compiled)
}

/// Compile compose arguments into typed argument references
///
/// # Arguments
///
/// * `args` - The compose arguments to compile
/// * `method_args` - The method arguments for reference validation
///
/// # Returns
///
/// A vector of compiled arguments
///
/// # Errors
///
/// Returns `InvalidStatement` if an argument reference is not found in method arguments
fn compile_args(
    args: &[ComposeArgAst],
    method_args: &[MethodArgAst],
) -> CompilerResult<Vec<CompiledArg>> {
    args.iter()
        .map(|arg| compile_single_arg(arg, method_args))
        .collect()
}

/// Compile a single ComposeArgAst into a CompiledArg, validating argument references
///
/// # Arguments
///
/// * `arg` - The compose argument to compile
/// * `method_args` - The method arguments for reference validation
///
/// # Returns
///
/// A compiled argument (either a literal value or an argument reference)
///
/// # Errors
///
/// Returns `InvalidStatement` if an argument reference is not found in method arguments
fn compile_single_arg(arg: &ComposeArgAst, method_args: &[MethodArgAst]) -> CompilerResult<CompiledArg> {
    match arg {
        ComposeArgAst::Named { name, arg_type } => {
            // Check if this is an argumentReference
            if arg_type == "argumentReference" {
                // Verify the argument exists in method args
                if method_args.iter().any(|a| a.name == *name) {
                    Ok(CompiledArg::ArgumentReference(name.clone()))
                } else {
                    Err(CompilerError::InvalidStatement(format!(
                        "Argument reference '{}' not found in method arguments",
                        name
                    )))
                }
            } else {
                // Regular named argument - treat as reference
                Ok(CompiledArg::ArgumentReference(name.clone()))
            }
        }
        ComposeArgAst::Value(v) => {
            // Literal value
            let literal = if v.is_string() {
                format!("\"{}\"", v.as_str().unwrap_or(""))
            } else if v.is_boolean() {
                v.as_bool().unwrap_or(false).to_string()
            } else if v.is_number() {
                v.to_string()
            } else {
                v.to_string()
            };
            Ok(CompiledArg::Literal(literal))
        }
    }
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("myMethod"), "my_method");
        assert_eq!(to_snake_case("MyMethod"), "my_method");
        assert_eq!(to_snake_case("getElementList"), "get_element_list");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-component"), "MyComponent");
        assert_eq!(to_pascal_case("my_component"), "MyComponent");
        assert_eq!(to_pascal_case("component"), "Component");
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_utam_type_to_rust_basic() {
        assert_eq!(utam_type_to_rust("string"), "String");
        assert_eq!(utam_type_to_rust("boolean"), "bool");
        assert_eq!(utam_type_to_rust("number"), "i64");
        assert_eq!(utam_type_to_rust("locator"), "By");
    }

    #[test]
    fn test_utam_type_to_rust_custom() {
        assert_eq!(
            utam_type_to_rust("utam-applications/pageObjects/component"),
            "Component"
        );
        assert_eq!(
            utam_type_to_rust("package/pageObjects/my-button"),
            "MyButton"
        );
    }

    #[test]
    fn test_method_rust_signature() {
        let method = MethodAst {
            name: "loginUser".to_string(),
            description: None,
            args: vec![
                MethodArgAst {
                    name: "username".to_string(),
                    arg_type: "string".to_string(),
                },
                MethodArgAst {
                    name: "password".to_string(),
                    arg_type: "string".to_string(),
                },
            ],
            compose: vec![],
            return_type: None,
            return_all: false,
        };

        let sig = method.rust_signature();
        assert_eq!(sig.name, "login_user");
        assert_eq!(sig.args.len(), 2);
        assert_eq!(sig.args[0].name, "username");
        assert_eq!(sig.args[0].rust_type, "String");
        assert_eq!(sig.return_type, "()");
        assert!(sig.is_async);
    }

    #[test]
    fn test_compile_single_arg_literal() {
        let arg = ComposeArgAst::Value(serde_json::json!("test"));
        let method_args = vec![];
        let compiled = compile_single_arg(&arg, &method_args).unwrap();
        assert_eq!(compiled, CompiledArg::Literal("\"test\"".to_string()));
    }

    #[test]
    fn test_compile_single_arg_reference() {
        let arg = ComposeArgAst::Named {
            name: "username".to_string(),
            arg_type: "argumentReference".to_string(),
        };
        let method_args = vec![MethodArgAst {
            name: "username".to_string(),
            arg_type: "string".to_string(),
        }];
        let compiled = compile_single_arg(&arg, &method_args).unwrap();
        assert_eq!(compiled, CompiledArg::ArgumentReference("username".to_string()));
    }

    #[test]
    fn test_compile_compose_statements_get_element() {
        let statements = vec![ComposeStatementAst {
            element: Some("submitButton".to_string()),
            apply: None,
            args: vec![],
            chain: false,
            return_type: None,
            return_all: false,
            matcher: None,
            apply_external: None,
            filter: None,
            return_element: false,
            predicate: None,
        }];

        let compiled = compile_compose_statements(&statements, &[], &[]).unwrap();
        assert_eq!(compiled.len(), 1);
        match &compiled[0].kind {
            StatementKind::GetElement { name } => {
                assert_eq!(name, "submitButton");
            }
            _ => panic!("Expected GetElement"),
        }
    }

    #[test]
    fn test_compile_compose_statements_apply_action() {
        let statements = vec![ComposeStatementAst {
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
        }];

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
            }
            _ => panic!("Expected ApplyAction"),
        }
    }
}
