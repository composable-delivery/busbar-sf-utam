//! AST types representing the parsed UTAM JSON structure.
//!
//! These types define the Abstract Syntax Tree for UTAM page object definitions.
//! All types derive Serialize, Deserialize, Debug, and Clone for proper JSON
//! handling and debugging.

use serde::{Deserialize, Serialize};

/// Root page object definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageObjectAst {
    #[serde(default)]
    pub description: Option<DescriptionAst>,
    #[serde(default)]
    pub root: bool,
    pub selector: Option<SelectorAst>,
    #[serde(rename = "exposeRootElement", default)]
    pub expose_root_element: bool,
    #[serde(rename = "type", default)]
    pub action_types: Vec<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub implements: Option<String>,
    #[serde(rename = "interface", default)]
    pub is_interface: bool,
    #[serde(default)]
    pub shadow: Option<ShadowAst>,
    #[serde(default)]
    pub elements: Vec<ElementAst>,
    #[serde(default)]
    pub methods: Vec<MethodAst>,
    #[serde(rename = "beforeLoad", default)]
    pub before_load: Vec<ComposeStatementAst>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Description can be a simple string or detailed object with author
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DescriptionAst {
    Simple(String),
    Detailed {
        text: Vec<String>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        #[serde(rename = "return")]
        return_desc: Option<String>,
    },
}

/// Shadow DOM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowAst {
    pub elements: Vec<ElementAst>,
}

/// Element definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAst {
    pub name: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub element_type: Option<ElementTypeAst>,
    pub selector: Option<SelectorAst>,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub nullable: bool,
    #[serde(rename = "wait", default)]
    pub generate_wait: bool,
    #[serde(default)]
    pub load: bool,
    #[serde(default)]
    pub shadow: Option<ShadowAst>,
    #[serde(default)]
    pub elements: Vec<ElementAst>,
    #[serde(default)]
    pub filter: Option<FilterAst>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub list: bool,
}

/// Element type - can be action types, custom component, container, or frame
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ElementTypeAst {
    /// Basic action types: ["clickable", "editable"] or "clickable"
    ActionTypes(Vec<String>),
    /// Custom component: "package/pageObjects/component"
    CustomComponent(String),
    /// Container literal
    Container,
    /// Frame literal
    Frame,
}

impl<'de> Deserialize<'de> for ElementTypeAst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct ElementTypeVisitor;

        impl<'de> Visitor<'de> for ElementTypeVisitor {
            type Value = ElementTypeAst;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or array of strings representing element type")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Check for literal types first
                match value {
                    "container" => return Ok(ElementTypeAst::Container),
                    "frame" => return Ok(ElementTypeAst::Frame),
                    _ => {}
                }

                // Check for known action types
                const ACTION_TYPES: &[&str] = &["clickable", "editable", "actionable", "draggable"];

                if ACTION_TYPES.contains(&value) {
                    // Single action type - wrap in ActionTypes
                    Ok(ElementTypeAst::ActionTypes(vec![value.to_string()]))
                } else {
                    // Everything else is a custom component
                    Ok(ElementTypeAst::CustomComponent(value.to_string()))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut types = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    types.push(value);
                }
                Ok(ElementTypeAst::ActionTypes(types))
            }
        }

        deserializer.deserialize_any(ElementTypeVisitor)
    }
}

/// Selector for locating elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorAst {
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub accessid: Option<String>,
    #[serde(default)]
    pub classchain: Option<String>,
    #[serde(default)]
    pub uiautomator: Option<String>,
    #[serde(default)]
    pub args: Vec<SelectorArgAst>,
    #[serde(rename = "returnAll", default)]
    pub return_all: bool,
}

/// Selector argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorArgAst {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
}

/// Types of selectors supported by UTAM
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorType {
    /// CSS selector
    Css(String),
    /// Mobile accessibility ID selector
    AccessibilityId(String),
    /// iOS class chain selector
    IosClassChain(String),
    /// Android UI automator selector
    AndroidUiAutomator(String),
    /// Unknown or empty selector
    Unknown,
}

impl SelectorAst {
    /// Returns the type and value of this selector
    pub fn selector_type(&self) -> SelectorType {
        if let Some(css) = &self.css {
            SelectorType::Css(css.clone())
        } else if let Some(accessid) = &self.accessid {
            SelectorType::AccessibilityId(accessid.clone())
        } else if let Some(classchain) = &self.classchain {
            SelectorType::IosClassChain(classchain.clone())
        } else if let Some(uiautomator) = &self.uiautomator {
            SelectorType::AndroidUiAutomator(uiautomator.clone())
        } else {
            SelectorType::Unknown
        }
    }

    /// Returns true if this selector has parameters
    pub fn has_parameters(&self) -> bool {
        !self.args.is_empty()
    }

    /// Counts the number of placeholders (%s and %d) in the selector string
    pub fn count_placeholders(&self) -> usize {
        let selector_str = match self.selector_type() {
            SelectorType::Css(s) => s,
            SelectorType::AccessibilityId(s) => s,
            SelectorType::IosClassChain(s) => s,
            SelectorType::AndroidUiAutomator(s) => s,
            SelectorType::Unknown => return 0,
        };

        let string_count = selector_str.matches("%s").count();
        let int_count = selector_str.matches("%d").count();
        string_count + int_count
    }

    /// Validates that the number of parameters matches the number of placeholders
    pub fn validate(&self) -> Result<(), crate::error::SelectorError> {
        if self.has_parameters() {
            let placeholder_count = self.count_placeholders();
            let arg_count = self.args.len();
            if placeholder_count != arg_count {
                return Err(crate::error::SelectorError::ParameterMismatch {
                    expected: placeholder_count,
                    actual: arg_count,
                });
            }
        }
        Ok(())
    }
}

/// Method definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodAst {
    pub name: String,
    #[serde(default)]
    pub description: Option<DescriptionAst>,
    #[serde(default)]
    pub args: Vec<MethodArgAst>,
    #[serde(default)]
    pub compose: Vec<ComposeStatementAst>,
    #[serde(rename = "returnType")]
    pub return_type: Option<String>,
    #[serde(rename = "returnAll", default)]
    pub return_all: bool,
}

/// Method argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodArgAst {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
}

/// Compose statement in a method body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeStatementAst {
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub apply: Option<String>,
    #[serde(default)]
    pub args: Vec<ComposeArgAst>,
    #[serde(default)]
    pub chain: bool,
    #[serde(rename = "returnType")]
    pub return_type: Option<String>,
    #[serde(rename = "returnAll", default)]
    pub return_all: bool,
    #[serde(default)]
    pub matcher: Option<MatcherAst>,
    #[serde(rename = "applyExternal")]
    pub apply_external: Option<ApplyExternalAst>,
    #[serde(default)]
    pub filter: Option<Vec<FilterAst>>,
    #[serde(rename = "returnElement", default)]
    pub return_element: bool,
    #[serde(default)]
    pub predicate: Option<Vec<ComposeStatementAst>>,
}

/// Argument in a compose statement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeArgAst {
    Named {
        name: String,
        #[serde(rename = "type")]
        arg_type: String,
    },
    Value(serde_json::Value),
}

/// External method application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyExternalAst {
    pub method: String,
    #[serde(default)]
    pub args: Vec<ComposeArgAst>,
}

/// Filter for element selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterAst {
    pub matcher: MatcherAst,
}

/// Matcher for filtering elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatcherAst {
    #[serde(rename = "type")]
    pub matcher_type: String,
    #[serde(default)]
    pub args: Vec<ComposeArgAst>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_description() {
        let json = r#""Simple description""#;
        let desc: DescriptionAst = serde_json::from_str(json).unwrap();
        match desc {
            DescriptionAst::Simple(s) => assert_eq!(s, "Simple description"),
            _ => panic!("Expected Simple variant"),
        }
    }

    #[test]
    fn test_detailed_description() {
        let json = r#"{"text": ["Line 1", "Line 2"], "author": "Test Author"}"#;
        let desc: DescriptionAst = serde_json::from_str(json).unwrap();
        match desc {
            DescriptionAst::Detailed { text, author, .. } => {
                assert_eq!(text.len(), 2);
                assert_eq!(author, Some("Test Author".to_string()));
            }
            _ => panic!("Expected Detailed variant"),
        }
    }

    #[test]
    fn test_selector_css() {
        let json = r#"{"css": ".button"}"#;
        let selector: SelectorAst = serde_json::from_str(json).unwrap();
        assert_eq!(selector.css, Some(".button".to_string()));
        assert!(selector.accessid.is_none());
    }

    #[test]
    fn test_element_type_action_types() {
        let json = r#"["clickable", "editable"]"#;
        let elem_type: ElementTypeAst = serde_json::from_str(json).unwrap();
        match elem_type {
            ElementTypeAst::ActionTypes(types) => {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&"clickable".to_string()));
            }
            _ => panic!("Expected ActionTypes variant"),
        }
    }

    #[test]
    fn test_element_type_custom_component() {
        let json = r#""utam-applications/pageObjects/component""#;
        let elem_type: ElementTypeAst = serde_json::from_str(json).unwrap();
        match elem_type {
            ElementTypeAst::CustomComponent(path) => {
                assert!(path.contains("component"));
            }
            _ => panic!("Expected CustomComponent variant"),
        }
    }

    #[test]
    fn test_element_type_single_action_type() {
        let json = r#""clickable""#;
        let elem_type: ElementTypeAst = serde_json::from_str(json).unwrap();
        match elem_type {
            ElementTypeAst::ActionTypes(types) => {
                assert_eq!(types.len(), 1);
                assert_eq!(types[0], "clickable");
            }
            _ => panic!("Expected ActionTypes variant with single type"),
        }
    }

    #[test]
    fn test_element_type_container() {
        let json = r#""container""#;
        let elem_type: ElementTypeAst = serde_json::from_str(json).unwrap();
        match elem_type {
            ElementTypeAst::Container => {}
            _ => panic!("Expected Container variant"),
        }
    }

    #[test]
    fn test_element_type_frame() {
        let json = r#""frame""#;
        let elem_type: ElementTypeAst = serde_json::from_str(json).unwrap();
        match elem_type {
            ElementTypeAst::Frame => {}
            _ => panic!("Expected Frame variant"),
        }
    }

    #[test]
    fn test_page_object_minimal() {
        let json = r#"{
            "root": true,
            "selector": {"css": ".app"}
        }"#;
        let page: PageObjectAst = serde_json::from_str(json).unwrap();
        assert!(page.root);
        assert!(page.selector.is_some());
    }

    #[test]
    fn test_round_trip_serialization() {
        let original = PageObjectAst {
            description: Some(DescriptionAst::Simple("Test".to_string())),
            root: true,
            selector: Some(SelectorAst {
                css: Some(".test".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            expose_root_element: false,
            action_types: vec![],
            platform: None,
            implements: None,
            is_interface: false,
            shadow: None,
            elements: vec![],
            methods: vec![],
            before_load: vec![],
            metadata: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: PageObjectAst = serde_json::from_str(&json).unwrap();

        assert_eq!(original.root, deserialized.root);
        assert!(deserialized.selector.is_some());
    }

    #[test]
    fn test_selector_type_css() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        match selector.selector_type() {
            SelectorType::Css(s) => assert_eq!(s, "button.submit"),
            _ => panic!("Expected Css selector type"),
        }
    }

    #[test]
    fn test_selector_type_accessid() {
        let selector = SelectorAst {
            css: None,
            accessid: Some("submit-btn".to_string()),
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        match selector.selector_type() {
            SelectorType::AccessibilityId(s) => assert_eq!(s, "submit-btn"),
            _ => panic!("Expected AccessibilityId selector type"),
        }
    }

    #[test]
    fn test_selector_type_classchain() {
        let selector = SelectorAst {
            css: None,
            accessid: None,
            classchain: Some("XCUIElementTypeButton[1]".to_string()),
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        match selector.selector_type() {
            SelectorType::IosClassChain(s) => assert_eq!(s, "XCUIElementTypeButton[1]"),
            _ => panic!("Expected IosClassChain selector type"),
        }
    }

    #[test]
    fn test_selector_type_uiautomator() {
        let selector = SelectorAst {
            css: None,
            accessid: None,
            classchain: None,
            uiautomator: Some("new UiSelector().text(\"Submit\")".to_string()),
            args: vec![],
            return_all: false,
        };

        match selector.selector_type() {
            SelectorType::AndroidUiAutomator(s) => {
                assert_eq!(s, "new UiSelector().text(\"Submit\")")
            }
            _ => panic!("Expected AndroidUiAutomator selector type"),
        }
    }

    #[test]
    fn test_selector_type_unknown() {
        let selector = SelectorAst {
            css: None,
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        match selector.selector_type() {
            SelectorType::Unknown => {}
            _ => panic!("Expected Unknown selector type"),
        }
    }

    #[test]
    fn test_has_parameters_true() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "id".to_string(),
                arg_type: "string".to_string(),
            }],
            return_all: false,
        };

        assert!(selector.has_parameters());
    }

    #[test]
    fn test_has_parameters_false() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert!(!selector.has_parameters());
    }

    #[test]
    fn test_count_placeholders_string() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert_eq!(selector.count_placeholders(), 1);
    }

    #[test]
    fn test_count_placeholders_number() {
        let selector = SelectorAst {
            css: Some("li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert_eq!(selector.count_placeholders(), 1);
    }

    #[test]
    fn test_count_placeholders_multiple() {
        let selector = SelectorAst {
            css: Some("div[data-type='%s'] > li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert_eq!(selector.count_placeholders(), 2);
    }

    #[test]
    fn test_count_placeholders_none() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert_eq!(selector.count_placeholders(), 0);
    }

    #[test]
    fn test_count_placeholders_mobile_selector() {
        let selector = SelectorAst {
            css: None,
            accessid: Some("submit-%s".to_string()),
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert_eq!(selector.count_placeholders(), 1);
    }

    #[test]
    fn test_validate_success_no_params() {
        let selector = SelectorAst {
            css: Some("button.submit".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };

        assert!(selector.validate().is_ok());
    }

    #[test]
    fn test_validate_success_matching_params() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "id".to_string(),
                arg_type: "string".to_string(),
            }],
            return_all: false,
        };

        assert!(selector.validate().is_ok());
    }

    #[test]
    fn test_validate_success_multiple_params() {
        let selector = SelectorAst {
            css: Some("div[data-type='%s'] > li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![
                SelectorArgAst {
                    name: "element_type".to_string(),
                    arg_type: "string".to_string(),
                },
                SelectorArgAst {
                    name: "index".to_string(),
                    arg_type: "number".to_string(),
                },
            ],
            return_all: false,
        };

        assert!(selector.validate().is_ok());
    }

    #[test]
    fn test_validate_error_too_many_args() {
        let selector = SelectorAst {
            css: Some("button[data-id='%s']".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![
                SelectorArgAst {
                    name: "id".to_string(),
                    arg_type: "string".to_string(),
                },
                SelectorArgAst {
                    name: "extra".to_string(),
                    arg_type: "string".to_string(),
                },
            ],
            return_all: false,
        };

        let result = selector.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SelectorError::ParameterMismatch { expected, actual } => {
                assert_eq!(expected, 1);
                assert_eq!(actual, 2);
            }
        }
    }

    #[test]
    fn test_validate_error_too_few_args() {
        let selector = SelectorAst {
            css: Some("div[data-type='%s'] > li:nth-child(%d)".to_string()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst {
                name: "element_type".to_string(),
                arg_type: "string".to_string(),
            }],
            return_all: false,
        };

        let result = selector.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SelectorError::ParameterMismatch { expected, actual } => {
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
        }
    }
}
