//! Page object discovery — detect known page objects on a live page and
//! auto-discover unknown components that could become page objects.
//!
//! # Two-phase discovery
//!
//! 1. **Match known page objects** — run registry root selectors against the
//!    live DOM to find which page objects are present on the current page.
//!
//! 2. **Discover unknown components** — walk the DOM for component boundaries
//!    (custom elements, shadow hosts, interactive clusters) and generate
//!    `.utam.json` skeletons for components not already in the registry.
//!
//! Together these give an agent a **coverage map**: what's known, what's not,
//! and draft page objects for the gaps.

use serde::{Deserialize, Serialize};

use utam_compiler::ast::*;

use crate::driver::{Selector, UtamDriver};
use crate::error::{RuntimeError, RuntimeResult};
use crate::registry::PageObjectRegistry;

// ---------------------------------------------------------------------------
// Discovery results
// ---------------------------------------------------------------------------

/// Result of running discovery on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryReport {
    /// URL of the page that was analyzed
    pub url: String,
    /// Known page objects whose root selector matched
    pub matched: Vec<MatchedPageObject>,
    /// Components discovered in the DOM that aren't in the registry
    pub discovered: Vec<DiscoveredComponent>,
}

/// A known page object that matched on the current page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPageObject {
    /// Registry name (e.g. "helpers/login")
    pub name: String,
    /// The root selector that matched
    pub selector: String,
    /// Number of methods available
    pub method_count: usize,
    /// Number of elements defined
    pub element_count: usize,
}

/// A component discovered in the DOM that isn't in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredComponent {
    /// The custom element tag name (e.g. "lightning-button")
    pub tag_name: String,
    /// CSS selector that uniquely identifies this component
    pub selector: String,
    /// Whether this element has a shadow root
    pub has_shadow: bool,
    /// Interactive child elements found inside
    pub children: Vec<DiscoveredChild>,
    /// Generated UTAM JSON for this component
    pub utam_json: String,
}

/// An interactive child element found during discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredChild {
    /// Tag name (e.g. "input", "button", "a")
    pub tag_name: String,
    /// A generated name for this element
    pub name: String,
    /// UTAM types inferred from the tag/attributes
    pub types: Vec<String>,
    /// CSS selector relative to the parent component
    pub selector: String,
}

// ---------------------------------------------------------------------------
// Phase 1: Match known page objects
// ---------------------------------------------------------------------------

/// Find which registered page objects are present on the current page.
///
/// Two-phase match:
///   1. Root selector matches at least one DOM element.  ALL 1454+
///      selectors are tested in a SINGLE `execute_script` call —
///      one-shot batch, not one find_elements per PO, which would be
///      thousands of driver round-trips and take 10+ minutes.
///   2. At least one of the PO's *required* (non-nullable) public
///      sub-elements also resolves against that root.  This filters
///      false positives where a PO has an overly generic root
///      selector (e.g. `.container`) that happens to match something
///      on the page that isn't actually the declared component.
///
/// Returns only page objects that pass both phases.
pub async fn find_known_page_objects(
    driver: &dyn UtamDriver,
    registry: &PageObjectRegistry,
) -> RuntimeResult<Vec<MatchedPageObject>> {
    // ── Phase 1: batch selector test in one JS round trip ─────────────
    // Gather (name, css_selector) pairs for every root PO that has a
    // CSS root selector.
    let mut candidates: Vec<(String, String, PageObjectAst)> = Vec::new();
    for name in registry.list() {
        let ast = match registry.get(&name) {
            Ok(ast) => ast,
            Err(_) => continue,
        };
        if !ast.root {
            continue;
        }
        let Some(css) = ast.selector.as_ref().and_then(|s| s.css.clone()) else {
            continue;
        };
        candidates.push((name, css, ast));
    }

    // One JS call: for each selector, return true iff it matches anything.
    // The browser does the work locally — no round-trip per selector.
    let selectors_json = serde_json::to_string(
        &candidates.iter().map(|(_, css, _)| css.as_str()).collect::<Vec<_>>(),
    )
    .map_err(RuntimeError::Json)?;
    let script = format!(
        "return (function() {{ \
             const sels = {selectors_json}; \
             const out = new Array(sels.length); \
             for (let i = 0; i < sels.length; i++) {{ \
                 try {{ out[i] = !!document.querySelector(sels[i]); }} \
                 catch (e) {{ out[i] = false; }} \
             }} \
             return out; \
         }})()"
    );
    let result = driver.execute_script(&script, vec![]).await?;
    let matches_bitmap: Vec<bool> = result
        .as_array()
        .map(|arr| arr.iter().map(|v| v.as_bool().unwrap_or(false)).collect())
        .unwrap_or_default();

    // ── Phase 2: verify each phase-1 match with an anchor child ───────
    // For POs where root matches, do a per-candidate find_elements to get
    // a handle, then probe one required child selector to confirm the
    // match is the real component (not a generic false positive).
    let mut matched = Vec::new();
    for (i, (name, selector_css, ast)) in candidates.into_iter().enumerate() {
        if !matches_bitmap.get(i).copied().unwrap_or(false) {
            continue;
        }
        let root_elements =
            match driver.find_elements(&Selector::Css(selector_css.clone())).await {
                Ok(els) if !els.is_empty() => els,
                _ => continue,
            };
        let first_root = &root_elements[0];
        if !confirm_page_object_match(first_root.as_ref(), &ast).await {
            continue;
        }

        let element_count =
            ast.elements.len() + ast.shadow.as_ref().map_or(0, |s| s.elements.len());
        matched.push(MatchedPageObject {
            name,
            selector: selector_css,
            method_count: ast.methods.len(),
            element_count,
        });
    }

    Ok(matched)
}

/// Probe a candidate root element with required child selectors from the
/// PO's declaration.  Returns true iff **a majority** of the PO's anchors
/// resolve — single anchor hits are insufficient to distinguish a real
/// match from a coincidence (e.g. `aura/body` has 30 unrelated elements,
/// one of which happens to be on the home page).
///
/// The majority threshold: at least 50% of anchors, rounded up, with a
/// minimum of 1.  POs with only 1 anchor require that anchor to match
/// (same behaviour as before).  POs with no usable anchors fall through
/// to "trust the root" — we can't distinguish without running load().
async fn confirm_page_object_match(
    root: &dyn crate::driver::ElementHandle,
    ast: &PageObjectAst,
) -> bool {
    let light_anchors: Vec<&ElementAst> = ast
        .elements
        .iter()
        .filter(|e| is_verification_anchor(e))
        .collect();
    let shadow_anchors: Vec<&ElementAst> = ast
        .shadow
        .as_ref()
        .map(|s| s.elements.iter().filter(|e| is_verification_anchor(e)).collect())
        .unwrap_or_default();

    let total_anchors = light_anchors.len() + shadow_anchors.len();
    if total_anchors == 0 {
        return true;
    }
    // Ceiling of total/2: {1→1, 2→1, 3→2, 4→2, 5→3, ..., 30→15}
    let required_hits = total_anchors.div_ceil(2);

    let mut hits = 0usize;

    for anchor in &light_anchors {
        if let Some(sel) = &anchor.selector {
            if let Some(css) = &sel.css {
                if root
                    .find_elements(&Selector::Css(css.clone()))
                    .await
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
                {
                    hits += 1;
                    if hits >= required_hits {
                        return true;
                    }
                }
            }
        }
    }

    if let Ok(Some(shadow)) = root.shadow_root().await {
        for anchor in &shadow_anchors {
            if let Some(sel) = &anchor.selector {
                if let Some(css) = &sel.css {
                    if shadow
                        .find_elements(&Selector::Css(css.clone()))
                        .await
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                    {
                        hits += 1;
                        if hits >= required_hits {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

fn is_verification_anchor(e: &ElementAst) -> bool {
    !e.nullable
        && e.selector
            .as_ref()
            .and_then(|s| s.css.as_deref())
            .map(|css| !css.contains("%s") && !css.contains("%d"))
            .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Phase 2: Discover unknown components
// ---------------------------------------------------------------------------

/// JavaScript that walks the DOM and returns component boundaries.
///
/// Identifies:
/// - Custom elements (tag names with hyphens — the Web Components convention)
/// - Elements with shadow roots
/// - Interactive children (inputs, buttons, links, textareas, selects)
const DISCOVERY_JS: &str = r#"
(() => {
    const components = [];
    const seen = new Set();

    function inferName(el) {
        if (el.id) return el.id;
        if (el.name) return el.name;
        if (el.getAttribute('data-aura-rendered-by')) return el.tagName.toLowerCase();
        const classes = el.className ? el.className.split(/\s+/).filter(c => c && !c.startsWith('slds-')) : [];
        if (classes.length > 0) return classes[0];
        return el.tagName.toLowerCase();
    }

    function inferTypes(el) {
        const tag = el.tagName.toLowerCase();
        const type = el.getAttribute('type') || '';
        if (tag === 'input' || tag === 'textarea' || tag === 'select') return ['editable'];
        if (tag === 'button' || tag === 'a' || type === 'submit' || type === 'button') return ['clickable'];
        if (el.getAttribute('draggable') === 'true') return ['draggable'];
        if (el.getAttribute('role') === 'button') return ['clickable'];
        if (el.getAttribute('role') === 'textbox') return ['editable'];
        return ['actionable'];
    }

    function selectorFor(el, parent) {
        if (el.id) return '#' + el.id;
        const tag = el.tagName.toLowerCase();
        const type = el.getAttribute('type');
        const name = el.getAttribute('name');
        if (name) return tag + "[name='" + name + "']";
        if (type) return tag + "[type='" + type + "']";
        // Use nth-of-type if there are siblings with the same tag
        const siblings = parent ? parent.querySelectorAll(':scope > ' + tag) : [];
        if (siblings.length > 1) {
            const idx = Array.from(siblings).indexOf(el) + 1;
            return tag + ':nth-of-type(' + idx + ')';
        }
        return tag;
    }

    function discoverChildren(root) {
        const children = [];
        const interactive = root.querySelectorAll(
            'input, button, a[href], textarea, select, [role="button"], [role="textbox"], [contenteditable="true"]'
        );
        for (const el of interactive) {
            const name = inferName(el);
            if (seen.has(name)) continue;
            seen.add(name);
            children.push({
                tagName: el.tagName.toLowerCase(),
                name: name,
                types: inferTypes(el),
                selector: selectorFor(el, root)
            });
        }
        return children;
    }

    // Find all custom elements (tag names with hyphens)
    const allElements = document.querySelectorAll('*');
    for (const el of allElements) {
        const tag = el.tagName.toLowerCase();
        if (!tag.includes('-')) continue;
        if (seen.has(tag)) continue;
        seen.add(tag);

        const hasShadow = !!el.shadowRoot;
        const searchRoot = hasShadow ? el.shadowRoot : el;
        const children = discoverChildren(searchRoot);

        components.push({
            tagName: tag,
            selector: tag,
            hasShadow: hasShadow,
            children: children
        });
    }

    return JSON.stringify(components);
})()
"#;

/// Raw component data returned from the browser JS.
#[derive(Debug, Deserialize)]
struct RawComponent {
    #[serde(rename = "tagName")]
    tag_name: String,
    selector: String,
    #[serde(rename = "hasShadow")]
    has_shadow: bool,
    children: Vec<RawChild>,
}

#[derive(Debug, Deserialize)]
struct RawChild {
    #[serde(rename = "tagName")]
    tag_name: String,
    name: String,
    types: Vec<String>,
    selector: String,
}

/// Discover components on the current page that aren't in the registry.
///
/// Runs JavaScript to walk the DOM for custom elements and interactive
/// children, then filters out any that are already known in the registry.
pub async fn discover_components(
    driver: &dyn UtamDriver,
    registry: Option<&PageObjectRegistry>,
) -> RuntimeResult<Vec<DiscoveredComponent>> {
    let result = driver.execute_script(DISCOVERY_JS, vec![]).await?;

    let json_str = match result {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };

    let raw: Vec<RawComponent> = serde_json::from_str(&json_str).unwrap_or_default();

    // Filter out components already in the registry
    let known_selectors: std::collections::HashSet<String> = registry
        .map(|r| {
            r.list()
                .into_iter()
                .filter_map(|name| {
                    r.get(&name).ok().and_then(|ast| ast.selector.and_then(|s| s.css))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut discovered = Vec::new();
    for comp in raw {
        if known_selectors.contains(&comp.selector) {
            continue;
        }

        let children: Vec<DiscoveredChild> = comp
            .children
            .iter()
            .map(|c| DiscoveredChild {
                tag_name: c.tag_name.clone(),
                name: c.name.clone(),
                types: c.types.clone(),
                selector: c.selector.clone(),
            })
            .collect();

        // Generate UTAM JSON for this component
        let utam_json =
            generate_utam_json(&comp.tag_name, &comp.selector, comp.has_shadow, &children);

        discovered.push(DiscoveredComponent {
            tag_name: comp.tag_name,
            selector: comp.selector,
            has_shadow: comp.has_shadow,
            children,
            utam_json,
        });
    }

    Ok(discovered)
}

/// Run full discovery: match known page objects + discover unknown components.
pub async fn discover(
    driver: &dyn UtamDriver,
    registry: &PageObjectRegistry,
) -> RuntimeResult<DiscoveryReport> {
    let url = driver.current_url().await?;
    let matched = find_known_page_objects(driver, registry).await?;
    let discovered = discover_components(driver, Some(registry)).await?;

    Ok(DiscoveryReport { url, matched, discovered })
}

// ---------------------------------------------------------------------------
// UTAM JSON generation
// ---------------------------------------------------------------------------

/// Generate a `.utam.json` string for a discovered component.
fn generate_utam_json(
    tag_name: &str,
    selector: &str,
    has_shadow: bool,
    children: &[DiscoveredChild],
) -> String {
    let elements: Vec<serde_json::Value> = children
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": to_camel_case(&c.name),
                "type": c.types,
                "selector": { "css": c.selector },
                "public": true
            })
        })
        .collect();

    let mut page_obj = serde_json::json!({
        "description": format!("Auto-discovered page object for <{tag_name}>"),
        "root": true,
        "selector": { "css": selector }
    });

    if has_shadow && !elements.is_empty() {
        page_obj["shadow"] = serde_json::json!({ "elements": elements });
    } else if !elements.is_empty() {
        page_obj["elements"] = serde_json::json!(elements);
    }

    serde_json::to_string_pretty(&page_obj).unwrap_or_default()
}

/// Convert a string to camelCase (simple heuristic).
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, c) in s.chars().enumerate() {
        if c == '-' || c == '_' || c == '.' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else if i == 0 {
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Convert discovery to AST (for direct use with DynamicPageObject)
// ---------------------------------------------------------------------------

/// Convert a discovered component into a `PageObjectAst` that can be
/// loaded directly with `DynamicPageObject`.
pub fn to_page_object_ast(component: &DiscoveredComponent) -> PageObjectAst {
    serde_json::from_str(&component.utam_json).unwrap_or_else(|_| PageObjectAst {
        description: Some(DescriptionAst::Simple(format!(
            "Auto-discovered: <{}>",
            component.tag_name
        ))),
        root: true,
        selector: Some(SelectorAst {
            css: Some(component.selector.clone()),
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
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("submit-button"), "submitButton");
        assert_eq!(to_camel_case("user_name"), "userName");
        assert_eq!(to_camel_case("simple"), "simple");
        assert_eq!(to_camel_case("MyComponent"), "myComponent");
        assert_eq!(to_camel_case("a-b-c"), "aBC");
    }

    #[test]
    fn test_generate_utam_json_simple() {
        let children = vec![DiscoveredChild {
            tag_name: "button".into(),
            name: "submit-btn".into(),
            types: vec!["clickable".into()],
            selector: "button[type='submit']".into(),
        }];

        let json = generate_utam_json("my-form", "my-form", false, &children);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["root"], true);
        assert_eq!(parsed["selector"]["css"], "my-form");
        assert!(parsed["elements"].is_array());
        assert_eq!(parsed["elements"][0]["name"], "submitBtn");
        assert_eq!(parsed["elements"][0]["type"][0], "clickable");
    }

    #[test]
    fn test_generate_utam_json_shadow() {
        let children = vec![DiscoveredChild {
            tag_name: "input".into(),
            name: "search-input".into(),
            types: vec!["editable".into()],
            selector: "input[name='q']".into(),
        }];

        let json = generate_utam_json("lightning-input", "lightning-input", true, &children);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["shadow"]["elements"].is_array());
        assert_eq!(parsed["shadow"]["elements"][0]["name"], "searchInput");
    }

    #[test]
    fn test_generate_utam_json_empty() {
        let json = generate_utam_json("my-empty", "my-empty", false, &[]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["root"], true);
        assert!(parsed.get("elements").is_none());
        assert!(parsed.get("shadow").is_none());
    }

    #[test]
    fn test_to_page_object_ast() {
        let comp = DiscoveredComponent {
            tag_name: "my-button".into(),
            selector: "my-button".into(),
            has_shadow: false,
            children: vec![DiscoveredChild {
                tag_name: "button".into(),
                name: "click-me".into(),
                types: vec!["clickable".into()],
                selector: "button".into(),
            }],
            utam_json: generate_utam_json(
                "my-button",
                "my-button",
                false,
                &[DiscoveredChild {
                    tag_name: "button".into(),
                    name: "click-me".into(),
                    types: vec!["clickable".into()],
                    selector: "button".into(),
                }],
            ),
        };

        let ast = to_page_object_ast(&comp);
        assert!(ast.root);
        assert_eq!(ast.selector.as_ref().unwrap().css.as_deref(), Some("my-button"));
        assert_eq!(ast.elements.len(), 1);
        assert_eq!(ast.elements[0].name, "clickMe");
    }

    #[test]
    fn test_discovery_js_is_valid() {
        // Just verify the JS string is not empty and contains expected tokens
        assert!(DISCOVERY_JS.contains("querySelectorAll"));
        assert!(DISCOVERY_JS.contains("shadowRoot"));
        assert!(DISCOVERY_JS.contains("tagName"));
        assert!(DISCOVERY_JS.contains("JSON.stringify"));
    }
}
