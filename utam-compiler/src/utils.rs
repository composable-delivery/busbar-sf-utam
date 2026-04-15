//! Utility functions for the UTAM compiler
//!
//! Common helpers used across the compiler codebase.

/// Rust reserved keywords that cannot be used as identifiers.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while", "yield",
];

/// Returns true if the given identifier is a Rust reserved keyword.
pub fn is_rust_keyword(s: &str) -> bool {
    RUST_KEYWORDS.contains(&s)
}

/// Convert string to snake_case.
///
/// Converts camelCase and PascalCase strings to snake_case.
/// Consecutive uppercase letters are kept together (e.g., "HTTPRequest" -> "httprequest").
/// If the result is a Rust keyword, appends `_` to avoid conflicts.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lowercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && prev_lowercase {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_lowercase = false;
        } else {
            result.push(ch);
            prev_lowercase = ch.is_lowercase();
        }
    }

    // Escape Rust keywords by appending underscore
    if is_rust_keyword(&result) {
        result.push('_');
    }

    result
}

/// Convert string to PascalCase
///
/// Converts strings with various separators (_, -, /, .) to PascalCase.
/// Example: "login-form" -> "LoginForm", "simple_button" -> "SimpleButton"
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch == '/' || ch == '.' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("submitButton"), "submit_button");
        assert_eq!(to_snake_case("usernameInput"), "username_input");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("HTTPRequest"), "httprequest");
        assert_eq!(to_snake_case("myHTTPSConnection"), "my_httpsconnection");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("login-form"), "LoginForm");
        assert_eq!(to_pascal_case("simple_button"), "SimpleButton");
        assert_eq!(to_pascal_case("simpleButton"), "SimpleButton");
        assert_eq!(to_pascal_case("component"), "Component");
        assert_eq!(to_pascal_case("my-test.component"), "MyTestComponent");
    }
}
