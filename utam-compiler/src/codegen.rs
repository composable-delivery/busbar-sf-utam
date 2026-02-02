//! Codegen module for generating Rust code from UTAM AST
//!
//! This module provides functions to generate Rust code from parsed UTAM page objects.

use crate::ast::SelectorAst;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates Rust code for a selector, handling parameterized selectors
///
/// For parameterized selectors (with args), generates a format! call that
/// substitutes %s and %d placeholders with the provided arguments.
///
/// # Examples
///
/// Simple selector:
/// ```text
/// By::Css("button.submit")
/// ```
///
/// Parameterized selector with %s:
/// ```text
/// By::Css(&format!("button[data-id='{}']", button_id))
/// ```
pub fn generate_selector_code(selector: &SelectorAst) -> TokenStream {
    if selector.has_parameters() {
        // Get the selector string - we only support CSS for now with parameters
        let template = match selector.css.as_ref() {
            Some(css) => css,
            None => {
                // For non-CSS selectors with parameters, we'll need to handle them later
                return quote! { compile_error!("Parameterized selectors only supported for CSS") };
            }
        };

        // Generate the argument list
        let args: Vec<_> = selector
            .args
            .iter()
            .map(|a| {
                let name = format_ident!("{}", a.name);
                quote! { #name }
            })
            .collect();

        // Replace %s and %d with {} for format!
        let format_str = template.replace("%s", "{}").replace("%d", "{}");

        quote! {
            thirtyfour::By::Css(&format!(#format_str, #(#args),*))
        }
    } else {
        // Simple selector without parameters
        if let Some(css) = &selector.css {
            quote! { thirtyfour::By::Css(#css) }
        } else if let Some(accessid) = &selector.accessid {
            quote! { thirtyfour::By::Id(#accessid) }
        } else if let Some(classchain) = &selector.classchain {
            quote! { thirtyfour::By::IosClassChain(#classchain) }
        } else if let Some(uiautomator) = &selector.uiautomator {
            quote! { thirtyfour::By::AndroidUiAutomator(#uiautomator) }
        } else {
            quote! { compile_error!("Selector must have at least one selector type") }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SelectorArgAst;

    #[test]
    fn test_generate_simple_css_selector() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        assert!(code_str.contains("thirtyfour :: By :: Css"));
        assert!(code_str.contains("button.submit"));
    }

    #[test]
    fn test_generate_parameterized_selector_with_string() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "button_id".to_string(),
                arg_type: "string".to_string(),
            }],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        // TokenStream adds spaces between tokens, so "format!" becomes "format !"
        assert!(code_str.contains("format !"));
        assert!(code_str.contains("button_id"));
        assert!(code_str.contains("{}"));
    }

    #[test]
    fn test_generate_parameterized_selector_with_number() {
        let selector = SelectorAst {
            css: Some("li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "index".to_string(),
                arg_type: "number".to_string(),
            }],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        // TokenStream adds spaces between tokens
        assert!(code_str.contains("format !"));
        assert!(code_str.contains("index"));
    }

    #[test]
    fn test_generate_mobile_selector_accessid() {
        let selector = SelectorAst {
            css: None,
            accessid: Some("submit-button".to_string()),
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        let code = generate_selector_code(&selector);
        let code_str = code.to_string();
        assert!(code_str.contains("thirtyfour :: By :: Id"));
        assert!(code_str.contains("submit-button"));
    }
}
