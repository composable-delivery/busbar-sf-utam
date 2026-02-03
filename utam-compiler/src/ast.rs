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

/// Categorizes element types for code generation and validation
#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    /// Basic element with no specific action types
    Basic,
    /// Element with specific action types (clickable, editable, etc.)
    Typed(Vec<String>),
    /// Custom component reference to another page object
    Custom(CustomComponentRef),
    /// Container element with default selector
    Container,
    /// Frame element for iframe handling
    Frame,
}

/// Reference to a custom component page object
#[derive(Debug, Clone, PartialEq)]
pub struct CustomComponentRef {
    /// Package name (e.g., "utam-applications")
    pub package: String,
    /// Path segments between package and name (e.g., ["components"])
    pub path: Vec<String>,
    /// Component name (e.g., "button-component")
    pub name: String,
}

impl CustomComponentRef {
    /// Parse a custom component reference from "package/pageObjects/path/name" format
    ///
    /// # Arguments
    ///
    /// * `s` - The custom component path string
    ///
    /// # Examples
    ///
    /// ```
    /// # use utam_compiler::ast::CustomComponentRef;
    /// let ref_str = "utam-applications/pageObjects/components/button-component";
    /// let comp_ref = CustomComponentRef::parse(ref_str);
    /// assert_eq!(comp_ref.package, "utam-applications");
    /// assert_eq!(comp_ref.path, vec!["components"]);
    /// assert_eq!(comp_ref.name, "button-component");
    /// ```
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.split('/').collect();
        
        // Handle various formats:
        // - "package/pageObjects/name" -> package="package", path=[], name="name"
        // - "package/pageObjects/path/name" -> package="package", path=["path"], name="name"
        // - "simple-component" (no slashes) -> package="", path=[], name="simple-component"
        
        if parts.len() == 1 {
            // Simple component reference with no package
            Self {
                package: String::new(),
                path: Vec::new(),
                name: parts[0].to_string(),
            }
        } else if parts.len() >= 3 {
            // Full path with package/pageObjects/...
            Self {
                package: parts[0].to_string(),
                path: if parts.len() > 3 {
                    parts[2..parts.len() - 1].iter().map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                },
                name: parts.last().unwrap().to_string(),
            }
        } else {
            // Fallback: treat as simple name
            Self {
                package: String::new(),
                path: Vec::new(),
                name: s.to_string(),
            }
        }
    }

    /// Convert the component name to a Rust type name (PascalCase)
    ///
    /// Converts kebab-case component names to PascalCase type names.
    ///
    /// # Returns
    ///
    /// A PascalCase type name suitable for Rust code generation
    ///
    /// # Examples
    ///
    /// ```
    /// # use utam_compiler::ast::CustomComponentRef;
    /// let comp_ref = CustomComponentRef {
    ///     package: "pkg".to_string(),
    ///     path: vec![],
    ///     name: "button-component".to_string(),
    /// };
    /// assert_eq!(comp_ref.to_rust_type(), "ButtonComponent");
    /// ```
    pub fn to_rust_type(&self) -> String {
        // Convert kebab-case to PascalCase
        self.name
            .split('-')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(c).collect(),
                }
            })
            .collect()
    }
}

impl ElementAst {
    /// Determine the element kind for code generation and validation
    ///
    /// Categorizes the element based on its type specification:
    /// - Basic: No type specified or empty action types
    /// - Typed: Has action types (clickable, editable, etc.)
    /// - Custom: References a custom component page object
    /// - Container: Container element
    /// - Frame: Frame element for iframe handling
    ///
    /// # Returns
    ///
    /// The element kind category
    ///
    /// # Examples
    ///
    /// ```
    /// # use utam_compiler::ast::{ElementAst, ElementTypeAst, ElementKind};
    /// let element = ElementAst {
    ///     name: "button".to_string(),
    ///     element_type: Some(ElementTypeAst::ActionTypes(vec!["clickable".to_string()])),
    ///     selector: None,
    ///     public: false,
    ///     nullable: false,
    ///     generate_wait: false,
    ///     load: false,
    ///     shadow: None,
    ///     elements: vec![],
    ///     filter: None,
    ///     description: None,
    ///     list: false,
    /// };
    /// match element.element_kind() {
    ///     ElementKind::Typed(types) => assert_eq!(types[0], "clickable"),
    ///     _ => panic!("Expected Typed element kind"),
    /// }
    /// ```
    pub fn element_kind(&self) -> ElementKind {
        match &self.element_type {
            None => ElementKind::Basic,
            Some(ElementTypeAst::ActionTypes(types)) => {
                if types.is_empty() {
                    ElementKind::Basic
                } else {
                    ElementKind::Typed(types.clone())
                }
            }
            Some(ElementTypeAst::CustomComponent(path)) => {
                ElementKind::Custom(CustomComponentRef::parse(path))
            }
            Some(ElementTypeAst::Container) => ElementKind::Container,
            Some(ElementTypeAst::Frame) => ElementKind::Frame,
        }
    }

    /// Validate element constraints
    ///
    /// Checks:
    /// - Element name is a valid Rust identifier
    /// - Frame elements do not have returnAll: true
    /// - Element names are unique within their scope
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, otherwise returns error messages
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate element name is a valid Rust identifier
        if !is_valid_rust_identifier(&self.name) {
            errors.push(format!(
                "Element name '{}' is not a valid Rust identifier. \
                 Names must start with a letter or underscore and contain only \
                 alphanumeric characters and underscores.",
                self.name
            ));
        }

        // Frame elements cannot have returnAll: true in their selector
        if matches!(self.element_kind(), ElementKind::Frame) {
            if let Some(selector) = &self.selector {
                if selector.return_all {
                    errors.push(format!(
                        "Frame element '{}' cannot have returnAll: true",
                        self.name
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Check if a string is a valid Rust identifier
///
/// Valid Rust identifiers:
/// - Start with a letter (a-z, A-Z) or underscore (_)
/// - Contain only letters, digits, and underscores
/// - Are not Rust keywords (basic check)
fn is_valid_rust_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check first character
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
    }

    // Check remaining characters
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return false;
        }
    }

    // Check against Rust keywords (basic list)
    const RUST_KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];

    !RUST_KEYWORDS.contains(&name)
}

impl PageObjectAst {
    /// Validate uniqueness of element names within the page object
    ///
    /// Checks that all element names are unique within:
    /// - Top-level elements
    /// - Shadow DOM elements
    /// - Nested elements
    ///
    /// # Returns
    ///
    /// `Ok(())` if all names are unique, otherwise returns error messages
    pub fn validate_element_names(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let mut names = std::collections::HashSet::new();

        // Validate top-level elements
        for element in &self.elements {
            if !names.insert(&element.name) {
                errors.push(format!(
                    "Duplicate element name '{}' in top-level elements",
                    element.name
                ));
            }
        }

        // Validate shadow elements
        if let Some(shadow) = &self.shadow {
            let mut shadow_names = std::collections::HashSet::new();
            for element in &shadow.elements {
                if !shadow_names.insert(&element.name) {
                    errors.push(format!(
                        "Duplicate element name '{}' in shadow elements",
                        element.name
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
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
    // Element kind tests
    #[test]
    fn test_element_kind_basic() {
        let element = ElementAst {
            name: "button".to_string(),
            element_type: None,
            selector: Some(SelectorAst {
                css: Some(".btn".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Basic => {}
            _ => panic!("Expected Basic element kind"),
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
    fn test_element_kind_typed() {
        let element = ElementAst {
            name: "button".to_string(),
            element_type: Some(ElementTypeAst::ActionTypes(vec![
                "clickable".to_string(),
                "actionable".to_string(),
            ])),
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Typed(types) => {
                assert_eq!(types.len(), 2);
                assert_eq!(types[0], "clickable");
                assert_eq!(types[1], "actionable");
            }
            _ => panic!("Expected Typed element kind"),
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
    fn test_element_kind_custom() {
        let element = ElementAst {
            name: "customBtn".to_string(),
            element_type: Some(ElementTypeAst::CustomComponent(
                "utam-applications/pageObjects/components/button-component".to_string(),
            )),
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Custom(comp_ref) => {
                assert_eq!(comp_ref.package, "utam-applications");
                assert_eq!(comp_ref.path, vec!["components"]);
                assert_eq!(comp_ref.name, "button-component");
            }
            _ => panic!("Expected Custom element kind"),
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
    fn test_element_kind_container() {
        let element = ElementAst {
            name: "container".to_string(),
            element_type: Some(ElementTypeAst::Container),
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Container => {}
            _ => panic!("Expected Container element kind"),
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
    fn test_element_kind_frame() {
        let element = ElementAst {
            name: "iframe".to_string(),
            element_type: Some(ElementTypeAst::Frame),
            selector: Some(SelectorAst {
                css: Some("iframe".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Frame => {}
            _ => panic!("Expected Frame element kind"),
        }
    }

    // CustomComponentRef tests
    #[test]
    fn test_custom_component_ref_parse_full_path() {
        let ref_str = "utam-applications/pageObjects/components/button-component";
        let comp_ref = CustomComponentRef::parse(ref_str);

        assert_eq!(comp_ref.package, "utam-applications");
        assert_eq!(comp_ref.path, vec!["components"]);
        assert_eq!(comp_ref.name, "button-component");
    }

    #[test]
    fn test_custom_component_ref_parse_nested_path() {
        let ref_str = "utam-pkg/pageObjects/level1/level2/component";
        let comp_ref = CustomComponentRef::parse(ref_str);

        assert_eq!(comp_ref.package, "utam-pkg");
        assert_eq!(comp_ref.path, vec!["level1", "level2"]);
        assert_eq!(comp_ref.name, "component");
    }

    #[test]
    fn test_custom_component_ref_parse_minimal() {
        let ref_str = "pkg/pageObjects/component";
        let comp_ref = CustomComponentRef::parse(ref_str);

        assert_eq!(comp_ref.package, "pkg");
        assert_eq!(comp_ref.path.len(), 0);
        assert_eq!(comp_ref.name, "component");
    }

    #[test]
    fn test_custom_component_ref_parse_simple() {
        let ref_str = "simple-component";
        let comp_ref = CustomComponentRef::parse(ref_str);

        assert_eq!(comp_ref.package, "");
        assert_eq!(comp_ref.path.len(), 0);
        assert_eq!(comp_ref.name, "simple-component");
    }

    #[test]
    fn test_custom_component_ref_to_rust_type() {
        let comp_ref = CustomComponentRef {
            package: "utam-applications".to_string(),
            path: vec!["components".to_string()],
            name: "button-component".to_string(),
        };

        assert_eq!(comp_ref.to_rust_type(), "ButtonComponent");
    }

    #[test]
    fn test_custom_component_ref_to_rust_type_single_word() {
        let comp_ref = CustomComponentRef {
            package: "pkg".to_string(),
            path: vec![],
            name: "button".to_string(),
        };

        assert_eq!(comp_ref.to_rust_type(), "Button");
    }

    #[test]
    fn test_custom_component_ref_to_rust_type_multiple_dashes() {
        let comp_ref = CustomComponentRef {
            package: "pkg".to_string(),
            path: vec![],
            name: "my-custom-button-component".to_string(),
        };

        assert_eq!(comp_ref.to_rust_type(), "MyCustomButtonComponent");
    }

    #[test]
    fn test_element_kind_empty_action_types() {
        let element = ElementAst {
            name: "element".to_string(),
            element_type: Some(ElementTypeAst::ActionTypes(vec![])),
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        match element.element_kind() {
            ElementKind::Basic => {}
            _ => panic!("Expected Basic element kind for empty action types"),
        }
    }

    // Validation tests
    #[test]
    fn test_validate_element_valid_name() {
        let element = ElementAst {
            name: "validButton".to_string(),
            element_type: Some(ElementTypeAst::ActionTypes(vec!["clickable".to_string()])),
            selector: Some(SelectorAst {
                css: Some(".btn".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        assert!(element.validate().is_ok());
    }

    #[test]
    fn test_validate_element_invalid_name_starts_with_digit() {
        let element = ElementAst {
            name: "123invalid".to_string(),
            element_type: None,
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        assert!(element.validate().is_err());
    }

    #[test]
    fn test_validate_element_invalid_name_with_hyphen() {
        let element = ElementAst {
            name: "invalid-name".to_string(),
            element_type: None,
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        assert!(element.validate().is_err());
    }

    #[test]
    fn test_validate_element_valid_name_with_underscore() {
        let element = ElementAst {
            name: "valid_name_123".to_string(),
            element_type: None,
            selector: None,
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        assert!(element.validate().is_ok());
    }

    #[test]
    fn test_validate_frame_with_return_all_fails() {
        let element = ElementAst {
            name: "myFrame".to_string(),
            element_type: Some(ElementTypeAst::Frame),
            selector: Some(SelectorAst {
                css: Some("iframe".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: true, // This should fail validation
            }),
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        let result = element.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("returnAll")));
    }

    #[test]
    fn test_validate_frame_without_return_all_succeeds() {
        let element = ElementAst {
            name: "myFrame".to_string(),
            element_type: Some(ElementTypeAst::Frame),
            selector: Some(SelectorAst {
                css: Some("iframe".to_string()),
                accessid: None,
                classchain: None,
                uiautomator: None,
                args: vec![],
                return_all: false,
            }),
            public: false,
            nullable: false,
            generate_wait: false,
            load: false,
            shadow: None,
            elements: vec![],
            filter: None,
            description: None,
            list: false,
        };

        assert!(element.validate().is_ok());
    }

    #[test]
    fn test_validate_element_names_unique() {
        let page = PageObjectAst {
            description: None,
            root: true,
            selector: Some(SelectorAst {
                css: Some(".root".to_string()),
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
            shadow: Some(ShadowAst {
                elements: vec![
                    ElementAst {
                        name: "button1".to_string(),
                        element_type: None,
                        selector: None,
                        public: false,
                        nullable: false,
                        generate_wait: false,
                        load: false,
                        shadow: None,
                        elements: vec![],
                        filter: None,
                        description: None,
                        list: false,
                    },
                    ElementAst {
                        name: "button2".to_string(),
                        element_type: None,
                        selector: None,
                        public: false,
                        nullable: false,
                        generate_wait: false,
                        load: false,
                        shadow: None,
                        elements: vec![],
                        filter: None,
                        description: None,
                        list: false,
                    },
                ],
            }),
            elements: vec![],
            methods: vec![],
            before_load: vec![],
            metadata: None,
        };

        assert!(page.validate_element_names().is_ok());
    }

    #[test]
    fn test_validate_element_names_duplicate() {
        let page = PageObjectAst {
            description: None,
            root: true,
            selector: Some(SelectorAst {
                css: Some(".root".to_string()),
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
            shadow: Some(ShadowAst {
                elements: vec![
                    ElementAst {
                        name: "button".to_string(),
                        element_type: None,
                        selector: None,
                        public: false,
                        nullable: false,
                        generate_wait: false,
                        load: false,
                        shadow: None,
                        elements: vec![],
                        filter: None,
                        description: None,
                        list: false,
                    },
                    ElementAst {
                        name: "button".to_string(), // Duplicate name
                        element_type: None,
                        selector: None,
                        public: false,
                        nullable: false,
                        generate_wait: false,
                        load: false,
                        shadow: None,
                        elements: vec![],
                        filter: None,
                        description: None,
                        list: false,
                    },
                ],
            }),
            elements: vec![],
            methods: vec![],
            before_load: vec![],
            metadata: None,
        };

        let result = page.validate_element_names();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("Duplicate")));
    }

    #[test]
    fn test_is_valid_rust_identifier() {
        assert!(super::is_valid_rust_identifier("validName"));
        assert!(super::is_valid_rust_identifier("_private"));
        assert!(super::is_valid_rust_identifier("button123"));
        assert!(super::is_valid_rust_identifier("MyButton"));
        
        // Invalid identifiers
        assert!(!super::is_valid_rust_identifier("123invalid"));
        assert!(!super::is_valid_rust_identifier("invalid-name"));
        assert!(!super::is_valid_rust_identifier("invalid name"));
        assert!(!super::is_valid_rust_identifier(""));
        
        // Rust keywords should be invalid
        assert!(!super::is_valid_rust_identifier("fn"));
        assert!(!super::is_valid_rust_identifier("let"));
        assert!(!super::is_valid_rust_identifier("struct"));
    }
}
