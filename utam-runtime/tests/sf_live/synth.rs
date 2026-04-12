//! Synthesize arguments and validate return values from UTAM type declarations.
//!
//! UTAM methods declare argument types (`"string"`, `"number"`, `"boolean"`)
//! and return types.  This module converts those declarations into runtime
//! values and validates results.

use std::collections::HashMap;

use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::MethodInfo;

/// Synthesize default arguments for a method based on its declared arg types.
///
/// Returns a HashMap suitable for `call_method`.  Strings default to empty,
/// numbers to 0, booleans to false, and unknown types to Null.
///
/// Callers with domain knowledge can override specific (page_object, method)
/// pairs via [`override_args`].
pub fn synth_args(info: &MethodInfo) -> HashMap<String, RuntimeValue> {
    let mut args = HashMap::new();
    for arg in &info.args {
        args.insert(arg.name.clone(), default_value_for_type(&arg.arg_type));
    }
    args
}

/// Produce a default runtime value for a declared UTAM type.
pub fn default_value_for_type(utam_type: &str) -> RuntimeValue {
    match utam_type {
        "string" => RuntimeValue::String(String::new()),
        "number" => RuntimeValue::Number(0),
        "boolean" => RuntimeValue::Bool(false),
        _ => RuntimeValue::Null,
    }
}

/// Page-object-specific argument overrides for methods that need real values.
///
/// The generic runner uses empty strings as defaults, which works for most
/// getters but fails for selector substitution.  This map supplies known-good
/// values for specific methods we care about.
pub fn override_args(po_name: &str, method_name: &str) -> Option<HashMap<String, RuntimeValue>> {
    let mut args = HashMap::new();
    match (po_name, method_name) {
        ("global/header", "getSearch") => {
            args.insert("searchTerm".into(), RuntimeValue::String("Accounts".into()));
        }
        ("setup/setupNavTree", "getAndWaitForNavTreeNodeByName") => {
            args.insert("ariaLabel".into(), RuntimeValue::String("Users".into()));
        }
        ("setup/setupNavTree", "waitForUrl") => {
            args.insert("url".into(), RuntimeValue::String("Setup".into()));
        }
        _ => return None,
    }
    Some(args)
}

/// Page-object-specific element arguments (for parameterized selectors).
pub fn override_element_args(
    po_name: &str,
    element_name: &str,
) -> Option<HashMap<String, RuntimeValue>> {
    let mut args = HashMap::new();
    match (po_name, element_name) {
        ("global/globalCreate", "globalCreateMenuItem") => {
            args.insert("titleString".into(), RuntimeValue::String("New Contact".into()));
        }
        ("setup/setupNavTree", "navTreeNodeByName") => {
            args.insert("ariaLabel".into(), RuntimeValue::String("Users".into()));
        }
        _ => return None,
    }
    Some(args)
}

/// Validate that a runtime value matches a declared UTAM return type.
///
/// Returns `Ok(())` if the type matches or if the declared type is unknown
/// (we don't fail on types we don't understand — just log).
pub fn validate_return(value: &RuntimeValue, declared: &str) -> Result<(), String> {
    // Normalize — some return types are comma-separated when an array was declared
    let primary = declared.split(',').next().unwrap_or(declared).trim();
    let ok = match primary {
        "string" => matches!(value, RuntimeValue::String(_)),
        "boolean" => matches!(value, RuntimeValue::Bool(_)),
        "number" => matches!(value, RuntimeValue::Number(_)),
        "void" | "none" | "null" => matches!(value, RuntimeValue::Null),
        // Element capability types — we accept Element, Null, or CustomComponent
        "clickable" | "editable" | "actionable" | "draggable" => {
            matches!(
                value,
                RuntimeValue::Element(_)
                    | RuntimeValue::Elements(_)
                    | RuntimeValue::CustomComponent { .. }
                    | RuntimeValue::Null
            )
        }
        // Custom component paths like "utam-global/pageObjects/appNav" — accept
        // anything non-null (they're resolved elements or child page objects)
        _ if primary.contains('/') => !matches!(value, RuntimeValue::Null),
        // Unknown types — accept anything
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("declared returnType '{primary}' but got {value:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utam_runtime::page_object::ArgInfo;

    #[test]
    fn test_default_values() {
        assert!(matches!(default_value_for_type("string"), RuntimeValue::String(s) if s.is_empty()));
        assert!(matches!(default_value_for_type("number"), RuntimeValue::Number(0)));
        assert!(matches!(default_value_for_type("boolean"), RuntimeValue::Bool(false)));
        assert!(matches!(default_value_for_type("mystery"), RuntimeValue::Null));
    }

    #[test]
    fn test_synth_args() {
        let info = MethodInfo {
            name: "login".into(),
            args: vec![
                ArgInfo { name: "user".into(), arg_type: "string".into() },
                ArgInfo { name: "age".into(), arg_type: "number".into() },
            ],
            return_type: None,
        };
        let args = synth_args(&info);
        assert_eq!(args.len(), 2);
        assert!(args.contains_key("user"));
        assert!(args.contains_key("age"));
    }

    #[test]
    fn test_override_args_known() {
        let args = override_args("global/header", "getSearch").unwrap();
        assert!(matches!(args.get("searchTerm"), Some(RuntimeValue::String(_))));
    }

    #[test]
    fn test_override_args_unknown() {
        assert!(override_args("unknown/page", "unknown_method").is_none());
    }

    #[test]
    fn test_validate_return_string() {
        assert!(validate_return(&RuntimeValue::String("hi".into()), "string").is_ok());
        assert!(validate_return(&RuntimeValue::Bool(true), "string").is_err());
    }

    #[test]
    fn test_validate_return_void() {
        assert!(validate_return(&RuntimeValue::Null, "void").is_ok());
        assert!(validate_return(&RuntimeValue::Null, "none").is_ok());
    }

    #[test]
    fn test_validate_return_unknown_type_passes() {
        // We don't fail on unknown types
        assert!(validate_return(&RuntimeValue::String("hi".into()), "mysterious").is_ok());
    }

    #[test]
    fn test_validate_return_custom_component() {
        // Custom component paths are accepted for non-null values
        assert!(validate_return(
            &RuntimeValue::String("elem".into()),
            "utam-global/pageObjects/appNav"
        )
        .is_ok());
        assert!(
            validate_return(&RuntimeValue::Null, "utam-global/pageObjects/appNav").is_err()
        );
    }
}
