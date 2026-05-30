//! Unit tests for `page_object` runtime logic.
//!
//! Kept in a sibling file (included via `#[path]` as a child module) so it
//! retains private access to the items under test — `execute_document_action`,
//! `resolve_selector`, `build_element_index`, `json_to_runtime_value`, etc. —
//! while being excluded from coverage measurement (see `codecov.yml`). The
//! in-memory `MockDriver`/`MockElement` doubles are pure test scaffolding:
//! their many `unimplemented!()` trait stubs exist only to satisfy the trait
//! and never run, so counting them as "uncovered" production lines would be
//! misleading. This mirrors the existing exclusion of `utam-runtime/tests/**`.

use super::*;
use crate::driver::ShadowRootHandle;

// -- Minimal in-memory driver double for unit-testing runtime logic that
//    doesn't need a real browser (e.g. document-level actions). --

#[derive(Debug)]
struct MockElement;

#[async_trait]
impl ElementHandle for MockElement {
    fn clone_handle(&self) -> Box<dyn ElementHandle> {
        Box::new(MockElement)
    }
    async fn text(&self) -> RuntimeResult<String> {
        unimplemented!()
    }
    async fn attribute(&self, _: &str) -> RuntimeResult<Option<String>> {
        unimplemented!()
    }
    async fn class_name(&self) -> RuntimeResult<String> {
        unimplemented!()
    }
    async fn css_value(&self, _: &str) -> RuntimeResult<String> {
        unimplemented!()
    }
    async fn property_value(&self) -> RuntimeResult<String> {
        unimplemented!()
    }
    async fn title(&self) -> RuntimeResult<String> {
        unimplemented!()
    }
    async fn is_displayed(&self) -> RuntimeResult<bool> {
        unimplemented!()
    }
    async fn is_enabled(&self) -> RuntimeResult<bool> {
        unimplemented!()
    }
    async fn is_present(&self) -> RuntimeResult<bool> {
        unimplemented!()
    }
    async fn is_focused(&self) -> RuntimeResult<bool> {
        unimplemented!()
    }
    async fn click(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn double_click(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn right_click(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn click_and_hold(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn focus(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn blur(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn send_keys(&self, _: &str) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn clear(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn press_key(&self, _: &str) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn scroll_into_view(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn drag_by_offset(&self, _: i64, _: i64) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn shadow_root(&self) -> RuntimeResult<Option<Box<dyn ShadowRootHandle>>> {
        unimplemented!()
    }
    async fn find_element(&self, _: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        unimplemented!()
    }
    async fn find_elements(&self, _: &Selector) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        unimplemented!()
    }
}

/// Driver double whose `find_elements` returns `found` mock elements,
/// letting us drive document-level presence checks deterministically.
struct MockDriver {
    found: usize,
}

#[async_trait]
impl UtamDriver for MockDriver {
    async fn navigate(&self, _: &str) -> RuntimeResult<()> {
        unimplemented!()
    }
    async fn current_url(&self) -> RuntimeResult<String> {
        Ok("https://example.lightning.force.com/lightning/page/home".into())
    }
    async fn title(&self) -> RuntimeResult<String> {
        Ok("Home".into())
    }
    async fn screenshot_png(&self) -> RuntimeResult<Vec<u8>> {
        unimplemented!()
    }
    async fn execute_script(
        &self,
        _: &str,
        _: Vec<serde_json::Value>,
    ) -> RuntimeResult<serde_json::Value> {
        unimplemented!()
    }
    async fn find_element(&self, _: &Selector) -> RuntimeResult<Box<dyn ElementHandle>> {
        Ok(Box::new(MockElement))
    }
    async fn find_elements(&self, _: &Selector) -> RuntimeResult<Vec<Box<dyn ElementHandle>>> {
        Ok((0..self.found).map(|_| Box::new(MockElement) as Box<dyn ElementHandle>).collect())
    }
    async fn wait_for_element(
        &self,
        _: &Selector,
        _: std::time::Duration,
    ) -> RuntimeResult<Box<dyn ElementHandle>> {
        Ok(Box::new(MockElement))
    }
    async fn quit(&self) -> RuntimeResult<()> {
        unimplemented!()
    }
}

fn mock_page(found: usize) -> DynamicPageObject {
    let ast: PageObjectAst =
        serde_json::from_str(r#"{"root":true,"selector":{"css":"body"}}"#).unwrap();
    let driver: Arc<dyn UtamDriver> = Arc::new(MockDriver { found });
    DynamicPageObject::from_element(driver, ast, Box::new(MockElement))
}

#[tokio::test]
async fn document_contains_element_reflects_dom_presence() {
    // Present in the DOM -> true.
    let po = mock_page(2);
    let r = execute_document_action(&po, "containsElement", &[RuntimeValue::String(".x".into())])
        .await
        .unwrap();
    assert!(matches!(r, RuntimeValue::Bool(true)));

    // Absent from the DOM -> false (not an error).
    let po0 = mock_page(0);
    let r0 = execute_document_action(&po0, "containsElement", &[RuntimeValue::String(".x".into())])
        .await
        .unwrap();
    assert!(matches!(r0, RuntimeValue::Bool(false)));
}

#[tokio::test]
async fn document_contains_element_requires_selector_arg() {
    let po = mock_page(1);
    assert!(execute_document_action(&po, "containsElement", &[]).await.is_err());
}

#[tokio::test]
async fn document_action_unknown_is_unsupported() {
    let po = mock_page(1);
    assert!(execute_document_action(&po, "frobnicate", &[]).await.is_err());
}

#[tokio::test]
async fn matcher_is_applied_to_the_action_result_not_the_previous_one() {
    // Regression: a statement with both an action and a matcher
    // (`document.getUrl()` stringContains url) must run the action first,
    // then match its result — not short-circuit the matcher against the
    // previous (empty) result, which made waitFor predicates never pass.
    let ast: PageObjectAst = serde_json::from_str(
        r#"{
            "root": true,
            "selector": {"css": "body"},
            "methods": [{
                "name": "urlContains",
                "args": [{"name": "url", "type": "string"}],
                "compose": [{
                    "element": "document",
                    "apply": "getUrl",
                    "matcher": {
                        "type": "stringContains",
                        "args": [{"name": "url", "type": "string"}]
                    }
                }]
            }]
        }"#,
    )
    .unwrap();
    let driver: Arc<dyn UtamDriver> = Arc::new(MockDriver { found: 0 });
    let po = DynamicPageObject::from_element(driver, ast, Box::new(MockElement));

    // MockDriver.current_url() contains "lightning".
    let mut args = HashMap::new();
    args.insert("url".to_string(), RuntimeValue::String("lightning".into()));
    let hit = po.call_method("urlContains", &args).await.unwrap();
    assert!(matches!(hit, RuntimeValue::Bool(true)), "expected true, got {hit:?}");

    let mut miss_args = HashMap::new();
    miss_args.insert("url".to_string(), RuntimeValue::String("not-in-the-url".into()));
    let miss = po.call_method("urlContains", &miss_args).await.unwrap();
    assert!(matches!(miss, RuntimeValue::Bool(false)), "expected false, got {miss:?}");
}

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
    assert!(matches!(json_to_runtime_value(&serde_json::json!(true)), RuntimeValue::Bool(true)));
    assert!(matches!(json_to_runtime_value(&serde_json::json!(42)), RuntimeValue::Number(42)));
    assert!(
        matches!(json_to_runtime_value(&serde_json::json!("hi")), RuntimeValue::String(s) if s == "hi")
    );
}

#[test]
fn test_json_to_runtime_value_locator_css() {
    // Typed locator literal: {"type": "locator", "value": {"css": ".foo"}}
    // must produce RuntimeValue::String(".foo") so actions like
    // containsElement(locator) receive the CSS string they expect.
    let v = serde_json::json!({ "type": "locator", "value": { "css": ".foo" } });
    match json_to_runtime_value(&v) {
        RuntimeValue::String(s) => assert_eq!(s, ".foo"),
        other => panic!("expected String(.foo), got {other:?}"),
    }
}

#[test]
fn test_json_to_runtime_value_typed_literal() {
    // {"type": "string", "value": "hi"} unwraps to the inner value.
    let v = serde_json::json!({ "type": "string", "value": "hi" });
    match json_to_runtime_value(&v) {
        RuntimeValue::String(s) => assert_eq!(s, "hi"),
        other => panic!("expected String(hi), got {other:?}"),
    }
}

#[test]
fn test_json_to_runtime_value_locator_accessid() {
    let v = serde_json::json!({ "type": "locator", "value": { "accessid": "my-id" } });
    match json_to_runtime_value(&v) {
        RuntimeValue::String(s) => assert_eq!(s, "my-id"),
        other => panic!("expected String, got {other:?}"),
    }
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
