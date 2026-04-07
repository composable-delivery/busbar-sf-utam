//! Dynamic page object — loads UTAM JSON at runtime, resolves elements
//! and executes compose methods against a live browser session.
//!
//! This is the core of the runtime interpreter: where the compile-time
//! codegen turns JSON into Rust structs, this module interprets the same
//! JSON on the fly.

use std::collections::HashMap;

use async_trait::async_trait;

use utam_compiler::ast::*;

use crate::driver::{ElementHandle, Selector, UtamDriver};
use crate::element::{DynamicElement, ElementCapability, RuntimeValue};
use crate::error::{RuntimeError, RuntimeResult};

// ---------------------------------------------------------------------------
// Selector resolution
// ---------------------------------------------------------------------------

/// Convert an AST selector to a runtime `Selector`, substituting `%s` / `%d`
/// placeholders with the provided arguments.
pub fn resolve_selector(
    ast: &SelectorAst,
    args: &HashMap<String, RuntimeValue>,
) -> RuntimeResult<Selector> {
    let raw = if let Some(css) = &ast.css {
        css.clone()
    } else if let Some(aid) = &ast.accessid {
        return Ok(Selector::AccessibilityId(aid.clone()));
    } else if let Some(cc) = &ast.classchain {
        return Ok(Selector::IosClassChain(cc.clone()));
    } else if let Some(ua) = &ast.uiautomator {
        return Ok(Selector::AndroidUiAutomator(ua.clone()));
    } else {
        return Err(RuntimeError::UnsupportedAction {
            action: "resolve_selector".into(),
            element_type: "selector has no type".into(),
        });
    };

    // Substitute parameters if any
    if ast.args.is_empty() {
        return Ok(Selector::Css(raw));
    }

    let mut result = raw;
    for arg_def in &ast.args {
        let val = args.get(&arg_def.name).ok_or_else(|| RuntimeError::ArgumentMissing {
            method: "selector".into(),
            arg_name: arg_def.name.clone(),
        })?;
        let replacement = match val {
            RuntimeValue::String(s) => s.clone(),
            RuntimeValue::Number(n) => n.to_string(),
            RuntimeValue::Bool(b) => b.to_string(),
            _ => val.to_string(),
        };
        // Replace first occurrence of %s or %d
        if result.contains("%s") {
            result = result.replacen("%s", &replacement, 1);
        } else if result.contains("%d") {
            result = result.replacen("%d", &replacement, 1);
        }
    }

    Ok(Selector::Css(result))
}

// ---------------------------------------------------------------------------
// PageObjectRuntime trait
// ---------------------------------------------------------------------------

/// Trait for interacting with a loaded page object at runtime.
#[async_trait]
pub trait PageObjectRuntime: Send + Sync {
    /// Call a compose method by name
    async fn call_method(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<RuntimeValue>;

    /// Resolve a named element
    async fn get_element(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<DynamicElement>;

    /// List available method names and their argument signatures
    fn method_signatures(&self) -> Vec<MethodInfo>;

    /// List available element names
    fn element_names(&self) -> Vec<&str>;

    /// Get the page object's description, if any
    fn description(&self) -> Option<String>;
}

/// Method signature information for introspection
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub args: Vec<ArgInfo>,
    pub return_type: Option<String>,
}

/// Argument information
#[derive(Debug, Clone)]
pub struct ArgInfo {
    pub name: String,
    pub arg_type: String,
}

// ---------------------------------------------------------------------------
// DynamicPageObject
// ---------------------------------------------------------------------------

/// A page object loaded from UTAM JSON, backed by a live browser session.
///
/// # Example
///
/// ```rust,ignore
/// let ast: PageObjectAst = serde_json::from_str(&json)?;
/// let page = DynamicPageObject::load(&driver, ast).await?;
///
/// let mut args = HashMap::new();
/// args.insert("userNameStr".into(), RuntimeValue::String("admin@sf.com".into()));
/// args.insert("passwordStr".into(), RuntimeValue::String("pass".into()));
/// page.call_method("login", &args).await?;
/// ```
pub struct DynamicPageObject {
    ast: PageObjectAst,
    root: Box<dyn ElementHandle>,
    driver: Box<dyn UtamDriver>,
    /// Flattened element index: name -> (ElementAst, is_in_shadow)
    element_index: HashMap<String, (ElementAst, bool)>,
}

impl DynamicPageObject {
    /// Load a root page object from the current page.
    ///
    /// Finds the root element using the AST's selector and returns a
    /// new `DynamicPageObject` backed by it.
    pub async fn load(driver: Box<dyn UtamDriver>, ast: PageObjectAst) -> RuntimeResult<Self> {
        let selector_ast =
            ast.selector.as_ref().ok_or_else(|| RuntimeError::UnsupportedAction {
                action: "load".into(),
                element_type: "page object has no root selector".into(),
            })?;
        let selector = resolve_selector(selector_ast, &HashMap::new())?;
        let root = driver.find_element(&selector).await?;

        let element_index = build_element_index(&ast);
        Ok(Self { ast, root, driver, element_index })
    }

    /// Wrap an existing element as a page object.
    pub fn from_element(
        driver: Box<dyn UtamDriver>,
        ast: PageObjectAst,
        root: Box<dyn ElementHandle>,
    ) -> Self {
        let element_index = build_element_index(&ast);
        Self { ast, root, driver, element_index }
    }

    /// Get the underlying driver
    pub fn driver(&self) -> &dyn UtamDriver {
        &*self.driver
    }

    /// Get the root element handle
    pub fn root(&self) -> &dyn ElementHandle {
        &*self.root
    }

    /// Get the raw AST
    pub fn ast(&self) -> &PageObjectAst {
        &self.ast
    }

    /// Resolve a single element by name from the DOM.
    async fn resolve_element(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<DynamicElement> {
        let (elem_ast, in_shadow) =
            self.element_index.get(name).ok_or_else(|| RuntimeError::ElementNotDefined {
                page_object: self.struct_name(),
                element: name.to_string(),
            })?;

        let selector_ast =
            elem_ast.selector.as_ref().ok_or_else(|| RuntimeError::ElementNotDefined {
                page_object: self.struct_name(),
                element: format!("{name} (no selector)"),
            })?;

        let selector = resolve_selector(selector_ast, args)?;

        // Determine where to search
        let parent: Box<dyn ElementHandle> = if *in_shadow {
            // Search inside the root's shadow DOM
            let shadow =
                self.root.shadow_root().await?.ok_or_else(|| RuntimeError::UnsupportedAction {
                    action: "shadow_root".into(),
                    element_type: "root has no shadow root".into(),
                })?;
            if selector_ast.return_all {
                let handles = shadow.find_elements(&selector).await?;
                if elem_ast.nullable && handles.is_empty() {
                    return Ok(DynamicElement::base(self.root.clone_handle()));
                }
                // For returnAll, wrap as Elements via RuntimeValue
                // But DynamicElement is singular — return the first for now
                // (the caller should use get_elements for lists)
                return Ok(wrap_element(
                    handles.into_iter().next().ok_or_else(|| RuntimeError::ElementNotDefined {
                        page_object: self.struct_name(),
                        element: name.to_string(),
                    })?,
                    elem_ast,
                ));
            }
            shadow.find_element(&selector).await?
        } else if selector_ast.return_all {
            let handles = self.root.find_elements(&selector).await?;
            if elem_ast.nullable && handles.is_empty() {
                return Ok(DynamicElement::base(self.root.clone_handle()));
            }
            return Ok(wrap_element(
                handles.into_iter().next().ok_or_else(|| RuntimeError::ElementNotDefined {
                    page_object: self.struct_name(),
                    element: name.to_string(),
                })?,
                elem_ast,
            ));
        } else {
            match self.root.find_element(&selector).await {
                Ok(el) => el,
                Err(e) if elem_ast.nullable => {
                    // Nullable elements return a base stub on not-found
                    let _ = e;
                    return Ok(DynamicElement::base(self.root.clone_handle()));
                }
                Err(e) => return Err(e),
            }
        };

        Ok(wrap_element(parent, elem_ast))
    }

    fn struct_name(&self) -> String {
        match &self.ast.description {
            Some(DescriptionAst::Simple(s)) => s.clone(),
            Some(DescriptionAst::Detailed { text, .. }) => {
                text.first().cloned().unwrap_or_else(|| "<unnamed>".into())
            }
            None => "<unnamed>".into(),
        }
    }
}

/// Wrap a raw ElementHandle in a DynamicElement with the right capability.
fn wrap_element(handle: Box<dyn ElementHandle>, ast: &ElementAst) -> DynamicElement {
    let types = match &ast.element_type {
        Some(ElementTypeAst::ActionTypes(types)) => types.clone(),
        _ => vec![],
    };
    DynamicElement::new(handle, &types)
}

/// Build a flat name→(ast, in_shadow) index from a PageObjectAst,
/// recursively collecting elements from nested `elements` and `shadow`.
fn build_element_index(ast: &PageObjectAst) -> HashMap<String, (ElementAst, bool)> {
    let mut index = HashMap::new();
    for elem in &ast.elements {
        collect_elements(elem, false, &mut index);
    }
    if let Some(shadow) = &ast.shadow {
        for elem in &shadow.elements {
            collect_elements(elem, true, &mut index);
        }
    }
    index
}

fn collect_elements(
    elem: &ElementAst,
    in_shadow: bool,
    index: &mut HashMap<String, (ElementAst, bool)>,
) {
    index.insert(elem.name.clone(), (elem.clone(), in_shadow));
    for child in &elem.elements {
        collect_elements(child, in_shadow, index);
    }
}

// ---------------------------------------------------------------------------
// Compose method interpreter
// ---------------------------------------------------------------------------

/// Execute a compose method's statements against a live page object.
fn execute_compose<'a>(
    page: &'a DynamicPageObject,
    statements: &'a [ComposeStatementAst],
    method_args: &'a HashMap<String, RuntimeValue>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = RuntimeResult<RuntimeValue>> + Send + 'a>> {
    Box::pin(async move {
        let mut last_result = RuntimeValue::Null;
        let mut last_element: Option<DynamicElement> = None;

        for stmt in statements {
            // 1. Resolve the element (if specified)
            let element = if let Some(elem_name) = &stmt.element {
                if elem_name == "document" {
                    // Special "document" element → use driver-level operations
                    None
                } else {
                    Some(page.resolve_element(elem_name, method_args).await?)
                }
            } else if stmt.chain {
                // Chain from previous result
                match &last_result {
                    RuntimeValue::Element(el) => Some(*el.clone()),
                    _ => last_element.clone(),
                }
            } else {
                None
            };

            // 2. Resolve arguments for this statement
            let stmt_args = resolve_compose_args(&stmt.args, method_args)?;

            // 3. Execute the action
            if let Some(apply) = &stmt.apply {
                if let Some(el) = &element {
                    // Element + action
                    use crate::element::ElementRuntime;
                    last_result = el.execute(apply, &stmt_args).await?;
                    last_element = Some(el.clone());
                } else if stmt.element.as_deref() == Some("document") {
                    // Document-level actions
                    last_result = execute_document_action(page, apply, &stmt_args).await?;
                } else {
                    // Self-referential method call (calling another method on this page object)
                    let mut sub_args = method_args.clone();
                    // Merge positional args into named args
                    for (i, val) in stmt_args.iter().enumerate() {
                        if let Some(method) = page.ast.methods.iter().find(|m| m.name == *apply) {
                            if let Some(arg_def) = method.args.get(i) {
                                sub_args.insert(arg_def.name.clone(), val.clone());
                            }
                        }
                    }
                    last_result = execute_method_by_name(page, apply, &sub_args).await?;
                }
            } else if stmt.return_element {
                // Just return the element
                if let Some(el) = element {
                    last_result = RuntimeValue::Element(Box::new(el.clone()));
                    last_element = Some(el);
                }
            } else if let Some(el) = element {
                // Element reference without action — store for chaining
                last_result = RuntimeValue::Element(Box::new(el.clone()));
                last_element = Some(el);
            }
        }

        Ok(last_result)
    }) // end Box::pin
}

/// Resolve compose statement arguments against method-level args.
fn resolve_compose_args(
    compose_args: &[ComposeArgAst],
    method_args: &HashMap<String, RuntimeValue>,
) -> RuntimeResult<Vec<RuntimeValue>> {
    let mut resolved = Vec::new();
    for arg in compose_args {
        match arg {
            ComposeArgAst::Named { name, arg_type } => {
                if arg_type == "argumentReference" || method_args.contains_key(name) {
                    // Reference to a method-level argument
                    let val = method_args.get(name).cloned().unwrap_or(RuntimeValue::Null);
                    resolved.push(val);
                } else {
                    // Literal named arg — treat name as the value
                    resolved.push(RuntimeValue::String(name.clone()));
                }
            }
            ComposeArgAst::Value(v) => {
                resolved.push(json_to_runtime_value(v));
            }
        }
    }
    Ok(resolved)
}

/// Convert a serde_json::Value to a RuntimeValue.
fn json_to_runtime_value(v: &serde_json::Value) -> RuntimeValue {
    match v {
        serde_json::Value::Null => RuntimeValue::Null,
        serde_json::Value::Bool(b) => RuntimeValue::Bool(*b),
        serde_json::Value::Number(n) => RuntimeValue::Number(n.as_i64().unwrap_or(0)),
        serde_json::Value::String(s) => RuntimeValue::String(s.clone()),
        _ => RuntimeValue::String(v.to_string()),
    }
}

/// Execute document-level actions (getUrl, getTitle, etc.)
async fn execute_document_action(
    page: &DynamicPageObject,
    action: &str,
    _args: &[RuntimeValue],
) -> RuntimeResult<RuntimeValue> {
    match action {
        "getUrl" => Ok(RuntimeValue::String(page.driver.current_url().await?)),
        "getTitle" => Ok(RuntimeValue::String(page.driver.title().await?)),
        _ => Err(RuntimeError::UnsupportedAction {
            action: action.to_string(),
            element_type: "document".to_string(),
        }),
    }
}

/// Find and execute a method by name on the page object.
async fn execute_method_by_name(
    page: &DynamicPageObject,
    name: &str,
    args: &HashMap<String, RuntimeValue>,
) -> RuntimeResult<RuntimeValue> {
    let method = page.ast.methods.iter().find(|m| m.name == name).ok_or_else(|| {
        RuntimeError::MethodNotFound { page_object: page.struct_name(), method: name.to_string() }
    })?;

    execute_compose(page, &method.compose, args).await
}

// ---------------------------------------------------------------------------
// PageObjectRuntime impl
// ---------------------------------------------------------------------------

#[async_trait]
impl PageObjectRuntime for DynamicPageObject {
    async fn call_method(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<RuntimeValue> {
        execute_method_by_name(self, name, args).await
    }

    async fn get_element(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<DynamicElement> {
        self.resolve_element(name, args).await
    }

    fn method_signatures(&self) -> Vec<MethodInfo> {
        self.ast
            .methods
            .iter()
            .map(|m| MethodInfo {
                name: m.name.clone(),
                args: m
                    .args
                    .iter()
                    .map(|a| ArgInfo { name: a.name.clone(), arg_type: a.arg_type.clone() })
                    .collect(),
                return_type: m.return_type.clone(),
            })
            .collect()
    }

    fn element_names(&self) -> Vec<&str> {
        self.element_index.keys().map(|k| k.as_str()).collect()
    }

    fn description(&self) -> Option<String> {
        match &self.ast.description {
            Some(DescriptionAst::Simple(s)) => Some(s.clone()),
            Some(DescriptionAst::Detailed { text, .. }) => Some(text.join(" ")),
            None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_selector_simple_css() {
        let ast = SelectorAst {
            css: Some("button.submit".into()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };
        let sel = resolve_selector(&ast, &HashMap::new()).unwrap();
        assert!(matches!(sel, Selector::Css(s) if s == "button.submit"));
    }

    #[test]
    fn test_resolve_selector_parameterized() {
        let ast = SelectorAst {
            css: Some("div[data-id='%s']".into()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst { name: "id".into(), arg_type: "string".into() }],
            return_all: false,
        };
        let mut args = HashMap::new();
        args.insert("id".into(), RuntimeValue::String("abc123".into()));
        let sel = resolve_selector(&ast, &args).unwrap();
        assert!(matches!(sel, Selector::Css(s) if s == "div[data-id='abc123']"));
    }

    #[test]
    fn test_resolve_selector_missing_arg() {
        let ast = SelectorAst {
            css: Some("div[data-id='%s']".into()),
            accessid: None,
            classchain: None,
            uiautomator: None,
            args: vec![SelectorArgAst { name: "id".into(), arg_type: "string".into() }],
            return_all: false,
        };
        let result = resolve_selector(&ast, &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_selector_accessid() {
        let ast = SelectorAst {
            css: None,
            accessid: Some("login-button".into()),
            classchain: None,
            uiautomator: None,
            args: vec![],
            return_all: false,
        };
        let sel = resolve_selector(&ast, &HashMap::new()).unwrap();
        assert!(matches!(sel, Selector::AccessibilityId(s) if s == "login-button"));
    }

    #[test]
    fn test_build_element_index() {
        let json = r#"{
            "root": true,
            "selector": { "css": ".page" },
            "elements": [
                { "name": "button", "selector": { "css": "button" } },
                { "name": "input", "selector": { "css": "input" } }
            ],
            "shadow": {
                "elements": [
                    { "name": "inner", "selector": { "css": ".inner" } }
                ]
            }
        }"#;
        let ast: PageObjectAst = serde_json::from_str(json).unwrap();
        let index = build_element_index(&ast);
        assert_eq!(index.len(), 3);
        assert!(!index["button"].1); // not in shadow
        assert!(!index["input"].1);
        assert!(index["inner"].1); // in shadow
    }

    #[test]
    fn test_resolve_compose_args_references() {
        let compose_args = vec![ComposeArgAst::Named {
            name: "username".into(),
            arg_type: "argumentReference".into(),
        }];
        let mut method_args = HashMap::new();
        method_args.insert("username".into(), RuntimeValue::String("admin".into()));

        let resolved = resolve_compose_args(&compose_args, &method_args).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].as_str().unwrap(), "admin");
    }

    #[test]
    fn test_resolve_compose_args_literal() {
        let compose_args = vec![ComposeArgAst::Value(serde_json::json!(42))];
        let resolved = resolve_compose_args(&compose_args, &HashMap::new()).unwrap();
        assert_eq!(resolved.len(), 1);
        assert!(matches!(resolved[0], RuntimeValue::Number(42)));
    }

    #[test]
    fn test_json_to_runtime_value() {
        assert!(matches!(json_to_runtime_value(&serde_json::json!(null)), RuntimeValue::Null));
        assert!(matches!(
            json_to_runtime_value(&serde_json::json!(true)),
            RuntimeValue::Bool(true)
        ));
        assert!(matches!(json_to_runtime_value(&serde_json::json!(42)), RuntimeValue::Number(42)));
        assert!(
            matches!(json_to_runtime_value(&serde_json::json!("hi")), RuntimeValue::String(s) if s == "hi")
        );
    }

    #[test]
    fn test_method_signatures_introspection() {
        let json = r#"{
            "root": true,
            "selector": { "css": ".page" },
            "elements": [
                { "name": "user", "type": ["editable"], "selector": { "css": "input" } },
                { "name": "btn", "type": ["clickable"], "selector": { "css": "button" } }
            ],
            "methods": [
                {
                    "name": "login",
                    "args": [
                        { "name": "username", "type": "string" },
                        { "name": "password", "type": "string" }
                    ],
                    "compose": []
                },
                {
                    "name": "getTitle",
                    "compose": [],
                    "returnType": "string"
                }
            ]
        }"#;
        let ast: PageObjectAst = serde_json::from_str(json).unwrap();
        let index = build_element_index(&ast);
        let page_obj_description = match &ast.description {
            Some(DescriptionAst::Simple(s)) => Some(s.clone()),
            Some(DescriptionAst::Detailed { text, .. }) => Some(text.join(" ")),
            None => None,
        };

        // Check element index
        assert_eq!(index.len(), 2);
        assert!(index.contains_key("user"));
        assert!(index.contains_key("btn"));

        // Check method signatures
        let sigs: Vec<MethodInfo> = ast
            .methods
            .iter()
            .map(|m| MethodInfo {
                name: m.name.clone(),
                args: m
                    .args
                    .iter()
                    .map(|a| ArgInfo { name: a.name.clone(), arg_type: a.arg_type.clone() })
                    .collect(),
                return_type: m.return_type.clone(),
            })
            .collect();
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0].name, "login");
        assert_eq!(sigs[0].args.len(), 2);
        assert_eq!(sigs[0].args[0].name, "username");
        assert_eq!(sigs[1].name, "getTitle");
        assert_eq!(sigs[1].return_type, Some("string".into()));
        assert!(page_obj_description.is_none());
    }
}
