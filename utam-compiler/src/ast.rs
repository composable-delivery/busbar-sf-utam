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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElementTypeAst {
    /// Basic action types: ["clickable", "editable"]
    ActionTypes(Vec<String>),
    /// Custom component: "package/pageObjects/component"
    CustomComponent(String),
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
}
