//! Dynamic page object — loads UTAM JSON at runtime, resolves elements
//! and executes compose methods against a live browser session.
//!
//! This is the core of the runtime interpreter: where the compile-time
//! codegen turns JSON into Rust structs, this module interprets the same
//! JSON on the fly.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use utam_compiler::ast::*;

use crate::driver::{ElementHandle, Selector, UtamDriver};
use crate::element::{DynamicElement, ElementRuntime, RuntimeValue};
use crate::error::{RuntimeError, RuntimeResult};
use crate::registry::PageObjectRegistry;

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
    driver: Arc<dyn UtamDriver>,
    /// Flattened element index: name -> (ElementAst, is_in_shadow, parent_name)
    element_index: HashMap<String, (ElementAst, bool, Option<String>)>,
    /// Optional registry for cross-page-object resolution
    registry: Option<Arc<PageObjectRegistry>>,
}

impl DynamicPageObject {
    /// Load a root page object from the current page.
    ///
    /// Finds the root element using the AST's selector, executes any
    /// `beforeLoad` predicates, and returns a new `DynamicPageObject`.
    pub async fn load(
        driver: impl Into<Arc<dyn UtamDriver>>,
        ast: PageObjectAst,
    ) -> RuntimeResult<Self> {
        let driver: Arc<dyn UtamDriver> = driver.into();
        let selector_ast =
            ast.selector.as_ref().ok_or_else(|| RuntimeError::UnsupportedAction {
                action: "load".into(),
                element_type: "page object has no root selector".into(),
            })?;
        let selector = resolve_selector(selector_ast, &HashMap::new())?;

        // If there are beforeLoad steps, wait for the root element
        let root = if !ast.before_load.is_empty() {
            driver.wait_for_element(&selector, std::time::Duration::from_secs(10)).await?
        } else {
            driver.find_element(&selector).await?
        };

        let element_index = build_element_index(&ast);
        Ok(Self { ast, root, driver, element_index, registry: None })
    }

    /// Wrap an existing element as a page object.
    pub fn from_element(
        driver: impl Into<Arc<dyn UtamDriver>>,
        ast: PageObjectAst,
        root: Box<dyn ElementHandle>,
    ) -> Self {
        let element_index = build_element_index(&ast);
        Self { ast, root, driver: driver.into(), element_index, registry: None }
    }

    /// Attach a registry for cross-page-object resolution.
    pub fn with_registry(mut self, registry: Arc<PageObjectRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Get the underlying driver (borrowed)
    pub fn driver(&self) -> &dyn UtamDriver {
        &*self.driver
    }

    /// Get a shared reference to the driver for creating child page objects
    pub fn driver_arc(&self) -> Arc<dyn UtamDriver> {
        Arc::clone(&self.driver)
    }

    /// Get the root element handle
    pub fn root(&self) -> &dyn ElementHandle {
        &*self.root
    }

    /// Get the raw AST
    pub fn ast(&self) -> &PageObjectAst {
        &self.ast
    }

    /// Get the optional registry
    pub fn registry(&self) -> Option<&PageObjectRegistry> {
        self.registry.as_deref()
    }

    /// Resolve a list of elements matching a selector (for `returnAll`/`list` elements).
    async fn resolve_elements(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<Vec<DynamicElement>> {
        let (elem_ast, in_shadow, ref parent_name) =
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
        let types = match &elem_ast.element_type {
            Some(ElementTypeAst::ActionTypes(types)) => types.clone(),
            _ => vec![],
        };

        // Resolve the search scope — if this element has a parent,
        // search within the parent element instead of the root.
        let scope: Box<dyn ElementHandle> = if let Some(pname) = parent_name {
            if let Some((parent_ast, parent_in_shadow, _)) = self.element_index.get(pname) {
                if let Some(parent_sel_ast) = &parent_ast.selector {
                    let parent_sel = resolve_selector(parent_sel_ast, args)?;
                    if *parent_in_shadow {
                        let shadow = self.root.shadow_root().await?.ok_or_else(|| {
                            RuntimeError::UnsupportedAction {
                                action: "shadow_root".into(),
                                element_type: "root has no shadow root".into(),
                            }
                        })?;
                        shadow.find_element(&parent_sel).await?
                    } else {
                        self.root.find_element(&parent_sel).await?
                    }
                } else {
                    self.root.clone_handle()
                }
            } else {
                self.root.clone_handle()
            }
        } else {
            self.root.clone_handle()
        };

        let handles = if *in_shadow {
            let shadow =
                scope.shadow_root().await?.ok_or_else(|| RuntimeError::UnsupportedAction {
                    action: "shadow_root".into(),
                    element_type: "element has no shadow root".into(),
                })?;
            shadow.find_elements(&selector).await?
        } else {
            scope.find_elements(&selector).await?
        };

        Ok(handles.into_iter().map(|h| DynamicElement::new(h, &types)).collect())
    }

    /// Resolve a single element by name from the DOM.
    async fn resolve_element(
        &self,
        name: &str,
        args: &HashMap<String, RuntimeValue>,
    ) -> RuntimeResult<DynamicElement> {
        // Special "root" pseudo-element — returns the root element itself
        if name == "root" {
            return Ok(DynamicElement::base(self.root.clone_handle()));
        }

        let (elem_ast, in_shadow, ref parent_elem_name) =
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

        // Resolve search scope — nested elements search within their parent.
        // We resolve the parent's selector directly to avoid async recursion.
        let scope: Box<dyn ElementHandle> = if let Some(pname) = parent_elem_name {
            if let Some((parent_ast, parent_in_shadow, _)) = self.element_index.get(pname) {
                if let Some(parent_sel_ast) = &parent_ast.selector {
                    let parent_sel = resolve_selector(parent_sel_ast, args)?;
                    if *parent_in_shadow {
                        let shadow = self.root.shadow_root().await?.ok_or_else(|| {
                            RuntimeError::UnsupportedAction {
                                action: "shadow_root".into(),
                                element_type: "root has no shadow root".into(),
                            }
                        })?;
                        shadow.find_element(&parent_sel).await?
                    } else {
                        self.root.find_element(&parent_sel).await?
                    }
                } else {
                    self.root.clone_handle()
                }
            } else {
                self.root.clone_handle()
            }
        } else {
            self.root.clone_handle()
        };

        // Determine where to search.
        //
        // Nullable elements that don't match return `RuntimeError::NullableAbsent`
        // so the compose-method interpreter can short-circuit to Null rather
        // than executing actions against a fake base element.
        let found: Box<dyn ElementHandle> = if *in_shadow {
            let shadow = match scope.shadow_root().await? {
                Some(sr) => sr,
                None if elem_ast.nullable => {
                    return Err(RuntimeError::NullableAbsent { element: name.to_string() });
                }
                None => {
                    return Err(RuntimeError::UnsupportedAction {
                        action: "shadow_root".into(),
                        element_type: "element has no shadow root".into(),
                    });
                }
            };
            if selector_ast.return_all {
                let handles = shadow.find_elements(&selector).await?;
                if handles.is_empty() && elem_ast.nullable {
                    return Err(RuntimeError::NullableAbsent { element: name.to_string() });
                }
                return Ok(wrap_element(
                    handles.into_iter().next().ok_or_else(|| RuntimeError::ElementNotDefined {
                        page_object: self.struct_name(),
                        element: name.to_string(),
                    })?,
                    elem_ast,
                ));
            }
            match shadow.find_element(&selector).await {
                Ok(el) => el,
                Err(_) if elem_ast.nullable => {
                    return Err(RuntimeError::NullableAbsent { element: name.to_string() });
                }
                Err(e) => return Err(e),
            }
        } else if selector_ast.return_all {
            let handles = scope.find_elements(&selector).await?;
            if handles.is_empty() && elem_ast.nullable {
                return Err(RuntimeError::NullableAbsent { element: name.to_string() });
            }
            return Ok(wrap_element(
                handles.into_iter().next().ok_or_else(|| RuntimeError::ElementNotDefined {
                    page_object: self.struct_name(),
                    element: name.to_string(),
                })?,
                elem_ast,
            ));
        } else {
            match scope.find_element(&selector).await {
                Ok(el) => el,
                Err(_) if elem_ast.nullable => {
                    return Err(RuntimeError::NullableAbsent { element: name.to_string() });
                }
                Err(e) => return Err(e),
            }
        };

        Ok(wrap_element(found, elem_ast))
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

/// Build a flat name→(ast, in_shadow, parent_name) index from a PageObjectAst,
/// recursively collecting elements from nested `elements` and `shadow`.
fn build_element_index(ast: &PageObjectAst) -> HashMap<String, (ElementAst, bool, Option<String>)> {
    let mut index = HashMap::new();
    for elem in &ast.elements {
        collect_elements(elem, false, None, &mut index);
    }
    if let Some(shadow) = &ast.shadow {
        for elem in &shadow.elements {
            collect_elements(elem, true, None, &mut index);
        }
    }
    index
}

fn collect_elements(
    elem: &ElementAst,
    in_shadow: bool,
    parent: Option<&str>,
    index: &mut HashMap<String, (ElementAst, bool, Option<String>)>,
) {
    index.insert(elem.name.clone(), (elem.clone(), in_shadow, parent.map(|s| s.to_string())));
    for child in &elem.elements {
        collect_elements(child, in_shadow, Some(&elem.name), index);
    }
    // Also recurse into element-level shadow
    if let Some(shadow) = &elem.shadow {
        for child in &shadow.elements {
            collect_elements(child, true, Some(&elem.name), index);
        }
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
            // 1. Handle filter statements — resolve list + filter
            if let Some(filters) = &stmt.filter {
                if let Some(elem_name) = &stmt.element {
                    let all = page.resolve_elements(elem_name, method_args).await?;
                    let filtered = apply_filters(all, filters, method_args).await?;
                    last_result = RuntimeValue::Elements(filtered);
                    continue;
                }
            }

            // 2. Handle applyExternal — cross-page-object method call
            if let Some(external) = &stmt.apply_external {
                let ext_args = resolve_compose_args(external.args(), method_args)?;
                // For now, if we have a registry we can try to resolve the external method.
                // The external method name format varies; store result and continue.
                last_result = RuntimeValue::String(format!(
                    "<external: {} with {} args>",
                    external.method(),
                    ext_args.len()
                ));
                continue;
            }

            // 3. Resolve the element (if specified)
            //
            // When the element has a CustomComponent type and a registry is
            // available, look up the referenced page object AST so chained
            // method calls can resolve through the child page object.
            //
            // Nullable elements that are absent produce `NullableAbsent`
            // errors; we catch them here, set the current result to Null,
            // and skip this statement.  This preserves the UTAM contract
            // that `nullable: true` means "method handles absence gracefully."
            let mut resolved_as_custom = false;
            let element = if let Some(elem_name) = &stmt.element {
                if elem_name == "document" {
                    None
                } else {
                    let el = match page.resolve_element(elem_name, method_args).await {
                        Ok(el) => el,
                        Err(RuntimeError::NullableAbsent { .. }) => {
                            last_result = RuntimeValue::Null;
                            last_element = None;
                            continue;
                        }
                        Err(e) => return Err(e),
                    };

                    // Check if this element is a custom component type
                    if let Some((elem_ast, _, _)) = page.element_index.get(elem_name) {
                        if let Some(ElementTypeAst::CustomComponent(ref path)) =
                            elem_ast.element_type
                        {
                            if let Some(ref registry) = page.registry {
                                if let Ok(child_ast) = registry.get(path) {
                                    // Wrap as CustomComponent for chaining
                                    last_result = RuntimeValue::CustomComponent {
                                        element: Box::new(el.clone()),
                                        ast: Box::new(child_ast),
                                        registry: Some(Arc::clone(registry)),
                                    };
                                    last_element = Some(el.clone());
                                    resolved_as_custom = true;
                                }
                            }
                        }
                    }

                    if resolved_as_custom {
                        None
                    } else {
                        Some(el)
                    }
                }
            } else if stmt.chain {
                match &last_result {
                    RuntimeValue::Element(el) => Some(*el.clone()),
                    RuntimeValue::CustomComponent { element, .. } => Some(*element.clone()),
                    _ => last_element.clone(),
                }
            } else {
                None
            };

            // If we resolved a custom component without an apply action,
            // skip to the next statement (the element is now in last_result
            // for chained access).
            if resolved_as_custom && stmt.apply.is_none() {
                continue;
            }

            // 4. Resolve arguments for this statement
            let stmt_args = resolve_compose_args(&stmt.args, method_args)?;

            // 5. Handle matcher assertions (e.g. stringContains on a result)
            if let Some(matcher) = &stmt.matcher {
                let matcher_args = resolve_compose_args(&matcher.args, method_args)?;
                let matched = evaluate_matcher(&matcher.matcher_type, &last_result, &matcher_args);
                last_result = RuntimeValue::Bool(matched);
                continue;
            }

            // 6. Execute the action
            if let Some(apply) = &stmt.apply {
                // Handle "waitFor" with predicate specially
                if apply == "waitFor" {
                    // The predicate can be at stmt.predicate OR inside args
                    // as {"type": "function", "predicate": [...]}
                    let predicate_stmts = if let Some(ref pred) = stmt.predicate {
                        Some(pred.clone())
                    } else {
                        // Extract predicate from function-type arg
                        stmt.args.iter().find_map(|arg| {
                            if let ComposeArgAst::Value(v) = arg {
                                if v.get("type").and_then(|t| t.as_str()) == Some("function") {
                                    if let Some(pred_val) = v.get("predicate") {
                                        serde_json::from_value::<Vec<ComposeStatementAst>>(
                                            pred_val.clone(),
                                        )
                                        .ok()
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                    };

                    if let Some(predicate) = predicate_stmts {
                        let predicate_result =
                            execute_wait_for_predicate(page, &predicate, method_args).await?;
                        last_result = predicate_result;
                    }
                    continue;
                }

                // If the last result is a CustomComponent and we're chaining,
                // delegate the method call to a nested DynamicPageObject.
                if stmt.chain {
                    if let RuntimeValue::CustomComponent { ref element, ref ast, ref registry } =
                        last_result
                    {
                        let child = DynamicPageObject::from_element(
                            page.driver_arc(),
                            *ast.clone(),
                            element.handle().clone_handle(),
                        );
                        let child = if let Some(reg) = registry {
                            child.with_registry(Arc::clone(reg))
                        } else {
                            child
                        };
                        // Try calling as a method on the child page object first
                        let mut sub_args = method_args.clone();
                        for (i, val) in stmt_args.iter().enumerate() {
                            if let Some(method) =
                                child.ast.methods.iter().find(|m| m.name == *apply)
                            {
                                if let Some(arg_def) = method.args.get(i) {
                                    sub_args.insert(arg_def.name.clone(), val.clone());
                                }
                            }
                        }
                        if child.ast.methods.iter().any(|m| m.name == *apply) {
                            last_result = execute_method_by_name(&child, apply, &sub_args).await?;
                            continue;
                        }
                        // If not a method, try resolving as an element getter
                        if child.element_index.contains_key(apply.as_str())
                            || child.element_index.contains_key(
                                &apply.strip_prefix("get").unwrap_or(apply).to_lowercase(),
                            )
                        {
                            let elem_name = if child.element_index.contains_key(apply.as_str()) {
                                apply.clone()
                            } else {
                                apply.strip_prefix("get").unwrap_or(apply).to_lowercase()
                            };
                            let el = child.resolve_element(&elem_name, &sub_args).await?;
                            last_result = RuntimeValue::Element(Box::new(el.clone()));
                            last_element = Some(el);
                            continue;
                        }
                        // Fall through to regular element execution
                    }
                }

                if let Some(el) = &element {
                    last_result = el.execute(apply, &stmt_args).await?;
                    last_element = Some(el.clone());
                } else if stmt.element.as_deref() == Some("document") {
                    last_result = execute_document_action(page, apply, &stmt_args).await?;
                } else {
                    // Self-referential method call
                    let mut sub_args = method_args.clone();
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
                if let Some(el) = element {
                    last_result = RuntimeValue::Element(Box::new(el.clone()));
                    last_element = Some(el);
                }
            } else if let Some(el) = element {
                last_result = RuntimeValue::Element(Box::new(el.clone()));
                last_element = Some(el);
            }
        }

        Ok(last_result)
    }) // end Box::pin
}

/// Apply filter predicates to a list of elements.
async fn apply_filters(
    elements: Vec<DynamicElement>,
    filters: &[FilterAst],
    _method_args: &HashMap<String, RuntimeValue>,
) -> RuntimeResult<Vec<DynamicElement>> {
    let mut result = elements;

    for filter in filters {
        let matcher_type = &filter.matcher.matcher_type;
        let mut kept = Vec::new();

        for el in result {
            let pass = match matcher_type.as_str() {
                "isVisible" => el.execute("isVisible", &[]).await?.as_bool().unwrap_or(false),
                "isEnabled" => el.execute("isEnabled", &[]).await?.as_bool().unwrap_or(false),
                "isPresent" => el.execute("isPresent", &[]).await?.as_bool().unwrap_or(false),
                _ => true, // Unknown matchers pass through
            };
            if pass {
                kept.push(el);
            }
        }
        result = kept;
    }

    Ok(result)
}

/// Evaluate a matcher against a runtime value.
fn evaluate_matcher(matcher_type: &str, value: &RuntimeValue, args: &[RuntimeValue]) -> bool {
    let actual = match value {
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Bool(b) => b.to_string(),
        RuntimeValue::Number(n) => n.to_string(),
        _ => return false,
    };
    let expected = args.first().map(|a| a.to_string()).unwrap_or_default();

    match matcher_type {
        "stringContains" | "contains" => actual.contains(&expected),
        "stringEquals" | "equals" => actual == expected,
        "startsWith" => actual.starts_with(&expected),
        "endsWith" => actual.ends_with(&expected),
        "isVisible" | "isTrue" => actual == "true",
        _ => false,
    }
}

/// Execute a `waitFor` predicate by polling its compose statements.
async fn execute_wait_for_predicate(
    page: &DynamicPageObject,
    predicate: &[ComposeStatementAst],
    method_args: &HashMap<String, RuntimeValue>,
) -> RuntimeResult<RuntimeValue> {
    utam_core::wait::wait_for(
        || async {
            match execute_compose(page, predicate, method_args).await {
                Ok(RuntimeValue::Bool(false)) => Ok(None), // Explicitly false → keep polling
                Ok(RuntimeValue::Null) => Ok(None),        // Null → keep polling
                Ok(val) => Ok(Some(val)),                  // Any truthy value → done
                Err(_) => Ok(None),                        // Error → keep polling
            }
        },
        &utam_core::wait::WaitConfig::default(),
        "waitFor predicate",
    )
    .await
    .map_err(RuntimeError::Utam)
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
