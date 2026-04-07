//! Dynamic element wrapper and runtime value types
//!
//! [`DynamicElement`] wraps utam-core element types behind a uniform interface,
//! dispatching action names to the correct trait methods at runtime.
//!
//! [`RuntimeValue`] is the dynamically-typed currency flowing through the interpreter.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use thirtyfour::WebElement;

use utam_core::elements::{BaseElement, ClickableElement, DraggableElement, EditableElement};
use utam_core::error::UtamError;
use utam_core::traits::{Actionable, Clickable, Draggable, Editable, Key};

use crate::error::{RuntimeError, RuntimeResult};

/// Dynamically-typed value flowing through the runtime interpreter.
///
/// Since the interpreter cannot know return types at compile time,
/// all values are wrapped in this enum.
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    /// No value / null result
    Null,
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// Integer value
    Number(i64),
    /// A resolved element
    Element(Box<DynamicElement>),
    /// A list of resolved elements
    Elements(Vec<DynamicElement>),
}

impl RuntimeValue {
    /// Extract as string, or return an error
    pub fn as_str(&self) -> RuntimeResult<&str> {
        match self {
            RuntimeValue::String(s) => Ok(s),
            other => Err(RuntimeError::ArgumentTypeMismatch {
                expected: "string".into(),
                actual: format!("{other:?}"),
            }),
        }
    }

    /// Extract as bool
    pub fn as_bool(&self) -> RuntimeResult<bool> {
        match self {
            RuntimeValue::Bool(b) => Ok(*b),
            other => Err(RuntimeError::ArgumentTypeMismatch {
                expected: "boolean".into(),
                actual: format!("{other:?}"),
            }),
        }
    }
}

impl fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeValue::Null => write!(f, "null"),
            RuntimeValue::String(s) => write!(f, "{s}"),
            RuntimeValue::Bool(b) => write!(f, "{b}"),
            RuntimeValue::Number(n) => write!(f, "{n}"),
            RuntimeValue::Element(_) => write!(f, "<element>"),
            RuntimeValue::Elements(v) => write!(f, "<{} elements>", v.len()),
        }
    }
}

/// Trait for executing actions on a resolved element at runtime.
///
/// The action name uses UTAM's camelCase convention (e.g. `"setText"`, `"click"`).
#[async_trait]
pub trait ElementRuntime: Send + Sync {
    /// Execute an action by name with the given arguments
    async fn execute(&self, action: &str, args: &[RuntimeValue]) -> RuntimeResult<RuntimeValue>;

    /// List the actions this element supports
    fn supported_actions(&self) -> &'static [&'static str];
}

/// Type-erased element wrapper.
///
/// Constructed from a `WebElement` + the element's declared type list
/// (from the UTAM JSON). Dispatches action name strings to the underlying
/// trait methods at runtime.
#[derive(Debug, Clone)]
pub enum DynamicElement {
    Base(BaseElement),
    Clickable(ClickableElement),
    Editable(EditableElement),
    Draggable(DraggableElement),
}

/// All actions supported by BaseElement (available on every variant)
const BASE_ACTIONS: &[&str] = &[
    "getText",
    "getAttribute",
    "getClassAttribute",
    "getCssPropertyValue",
    "getTitle",
    "getValue",
    "isEnabled",
    "isFocused",
    "isPresent",
    "isVisible",
    "focus",
    "blur",
    "scrollIntoView",
    "scrollToCenter",
    "scrollToTop",
    "moveTo",
    "waitForVisible",
    "waitForInvisible",
    "waitForAbsence",
    "waitForEnabled",
    "containsElement",
];

const CLICK_ACTIONS: &[&str] = &["click", "doubleClick", "rightClick", "clickAndHold"];
const EDIT_ACTIONS: &[&str] = &["clear", "setText", "clearAndType", "press"];
const DRAG_ACTIONS: &[&str] = &["dragAndDropByOffset"];

impl DynamicElement {
    /// Construct from a WebElement and the UTAM type list (e.g. `["clickable", "editable"]`).
    ///
    /// Picks the most capable wrapper: draggable > editable > clickable > base.
    pub fn from_type_list(element: WebElement, types: &[String]) -> Self {
        if types.iter().any(|t| t == "draggable") {
            DynamicElement::Draggable(DraggableElement::new(element))
        } else if types.iter().any(|t| t == "editable") {
            DynamicElement::Editable(EditableElement::new(element))
        } else if types.iter().any(|t| t == "clickable") {
            DynamicElement::Clickable(ClickableElement::new(element))
        } else {
            DynamicElement::Base(BaseElement::new(element))
        }
    }

    /// Construct as a plain base element (no special capabilities)
    pub fn base(element: WebElement) -> Self {
        DynamicElement::Base(BaseElement::new(element))
    }

    /// Get the underlying BaseElement (all variants contain one)
    fn as_base(&self) -> BaseElement {
        match self {
            DynamicElement::Base(e) => e.clone(),
            DynamicElement::Clickable(e) => BaseElement::new(e.inner().clone()),
            DynamicElement::Editable(e) => BaseElement::new(e.inner().clone()),
            DynamicElement::Draggable(e) => BaseElement::new(e.inner().clone()),
        }
    }

    /// Get the inner WebElement
    pub fn web_element(&self) -> &WebElement {
        match self {
            DynamicElement::Base(e) => e.inner(),
            DynamicElement::Clickable(e) => e.inner(),
            DynamicElement::Editable(e) => e.inner(),
            DynamicElement::Draggable(e) => e.inner(),
        }
    }

    /// What kind of element is this?
    pub fn type_name(&self) -> &'static str {
        match self {
            DynamicElement::Base(_) => "base",
            DynamicElement::Clickable(_) => "clickable",
            DynamicElement::Editable(_) => "editable",
            DynamicElement::Draggable(_) => "draggable",
        }
    }
}

/// Execute base-level actions available on any element variant.
async fn execute_base_action(
    base: &BaseElement,
    action: &str,
    args: &[RuntimeValue],
) -> RuntimeResult<RuntimeValue> {
    match action {
        "getText" => Ok(RuntimeValue::String(base.get_text().await?)),
        "getAttribute" => {
            let name = args.first().map(|a| a.as_str()).transpose()?.unwrap_or("value");
            Ok(RuntimeValue::String(
                base.get_attribute(name).await?.unwrap_or_default(),
            ))
        }
        "getClassAttribute" => Ok(RuntimeValue::String(base.get_class_attribute().await?)),
        "getCssPropertyValue" => {
            let name = require_str_arg(args, 0, "getCssPropertyValue")?;
            Ok(RuntimeValue::String(base.get_css_property_value(name).await?))
        }
        "getTitle" => Ok(RuntimeValue::String(base.get_title().await?)),
        "getValue" => Ok(RuntimeValue::String(base.get_value().await?)),
        "isEnabled" => Ok(RuntimeValue::Bool(base.is_enabled().await?)),
        "isFocused" => Ok(RuntimeValue::Bool(base.is_focused().await?)),
        "isPresent" => Ok(RuntimeValue::Bool(base.is_present().await?)),
        "isVisible" => Ok(RuntimeValue::Bool(base.is_visible().await?)),
        "focus" => {
            base.focus().await?;
            Ok(RuntimeValue::Null)
        }
        "blur" => {
            base.blur().await?;
            Ok(RuntimeValue::Null)
        }
        "scrollIntoView" => {
            base.scroll_into_view().await?;
            Ok(RuntimeValue::Null)
        }
        "scrollToCenter" => {
            base.scroll_to_center().await?;
            Ok(RuntimeValue::Null)
        }
        "scrollToTop" => {
            base.scroll_to_top().await?;
            Ok(RuntimeValue::Null)
        }
        "moveTo" => {
            base.move_to().await?;
            Ok(RuntimeValue::Null)
        }
        "waitForVisible" => {
            base.wait_for_visible(Duration::from_secs(10)).await?;
            Ok(RuntimeValue::Null)
        }
        "waitForInvisible" => {
            base.wait_for_invisible(Duration::from_secs(10)).await?;
            Ok(RuntimeValue::Null)
        }
        "waitForAbsence" => {
            base.wait_for_absence(Duration::from_secs(10)).await?;
            Ok(RuntimeValue::Null)
        }
        "waitForEnabled" => {
            base.wait_for_enabled(Duration::from_secs(10)).await?;
            Ok(RuntimeValue::Null)
        }
        "containsElement" => {
            let selector = require_str_arg(args, 0, "containsElement")?;
            Ok(RuntimeValue::Bool(
                base.contains_element(selector, false).await?,
            ))
        }
        _ => Err(RuntimeError::UnsupportedAction {
            action: action.to_string(),
            element_type: "base".to_string(),
        }),
    }
}

/// Helper to extract a required string argument at a given position.
fn require_str_arg<'a>(
    args: &'a [RuntimeValue],
    index: usize,
    action: &str,
) -> RuntimeResult<&'a str> {
    args.get(index)
        .ok_or_else(|| RuntimeError::ArgumentMissing {
            method: action.to_string(),
            arg_name: format!("arg[{index}]"),
        })?
        .as_str()
}

/// Convert a string to a Key enum value.
fn parse_key(name: &str) -> RuntimeResult<Key> {
    match name {
        "Enter" => Ok(Key::Enter),
        "Tab" => Ok(Key::Tab),
        "Escape" => Ok(Key::Escape),
        "Backspace" => Ok(Key::Backspace),
        "Delete" => Ok(Key::Delete),
        "ArrowUp" => Ok(Key::ArrowUp),
        "ArrowDown" => Ok(Key::ArrowDown),
        "ArrowLeft" => Ok(Key::ArrowLeft),
        "ArrowRight" => Ok(Key::ArrowRight),
        "Home" => Ok(Key::Home),
        "End" => Ok(Key::End),
        "PageUp" => Ok(Key::PageUp),
        "PageDown" => Ok(Key::PageDown),
        "Space" => Ok(Key::Space),
        _ => Err(RuntimeError::ArgumentTypeMismatch {
            expected: "valid key name".into(),
            actual: name.into(),
        }),
    }
}

#[async_trait]
impl ElementRuntime for DynamicElement {
    async fn execute(&self, action: &str, args: &[RuntimeValue]) -> RuntimeResult<RuntimeValue> {
        // Try base actions first (available on all variants)
        let base = self.as_base();
        if BASE_ACTIONS.contains(&action) {
            return execute_base_action(&base, action, args).await;
        }

        match self {
            DynamicElement::Clickable(el) => match action {
                "click" => {
                    el.click().await?;
                    Ok(RuntimeValue::Null)
                }
                "doubleClick" => {
                    el.double_click().await?;
                    Ok(RuntimeValue::Null)
                }
                "rightClick" => {
                    el.right_click().await?;
                    Ok(RuntimeValue::Null)
                }
                "clickAndHold" => {
                    el.click_and_hold().await?;
                    Ok(RuntimeValue::Null)
                }
                _ => Err(RuntimeError::UnsupportedAction {
                    action: action.to_string(),
                    element_type: "clickable".into(),
                }),
            },

            DynamicElement::Editable(el) => match action {
                // Editable also supports click actions
                "click" => {
                    el.inner().click().await.map_err(UtamError::WebDriver)?;
                    Ok(RuntimeValue::Null)
                }
                "clear" => {
                    el.clear().await?;
                    Ok(RuntimeValue::Null)
                }
                "setText" => {
                    let text = require_str_arg(args, 0, "setText")?;
                    el.set_text(text).await?;
                    Ok(RuntimeValue::Null)
                }
                "clearAndType" => {
                    let text = require_str_arg(args, 0, "clearAndType")?;
                    el.clear_and_type(text).await?;
                    Ok(RuntimeValue::Null)
                }
                "press" => {
                    let key_name = require_str_arg(args, 0, "press")?;
                    el.press(parse_key(key_name)?).await?;
                    Ok(RuntimeValue::Null)
                }
                _ => Err(RuntimeError::UnsupportedAction {
                    action: action.to_string(),
                    element_type: "editable".into(),
                }),
            },

            DynamicElement::Draggable(el) => match action {
                // Draggable also supports click actions
                "click" => {
                    el.inner().click().await.map_err(UtamError::WebDriver)?;
                    Ok(RuntimeValue::Null)
                }
                "dragAndDropByOffset" => {
                    let x = match args.first() {
                        Some(RuntimeValue::Number(n)) => *n,
                        _ => {
                            return Err(RuntimeError::ArgumentMissing {
                                method: "dragAndDropByOffset".into(),
                                arg_name: "x".into(),
                            })
                        }
                    };
                    let y = match args.get(1) {
                        Some(RuntimeValue::Number(n)) => *n,
                        _ => {
                            return Err(RuntimeError::ArgumentMissing {
                                method: "dragAndDropByOffset".into(),
                                arg_name: "y".into(),
                            })
                        }
                    };
                    el.drag_and_drop_by_offset(x, y).await?;
                    Ok(RuntimeValue::Null)
                }
                _ => Err(RuntimeError::UnsupportedAction {
                    action: action.to_string(),
                    element_type: "draggable".into(),
                }),
            },

            DynamicElement::Base(_) => Err(RuntimeError::UnsupportedAction {
                action: action.to_string(),
                element_type: "base".into(),
            }),
        }
    }

    fn supported_actions(&self) -> &'static [&'static str] {
        match self {
            DynamicElement::Base(_) => BASE_ACTIONS,
            DynamicElement::Clickable(_) => CLICK_ACTIONS,
            DynamicElement::Editable(_) => EDIT_ACTIONS,
            DynamicElement::Draggable(_) => DRAG_ACTIONS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_value_display() {
        assert_eq!(RuntimeValue::Null.to_string(), "null");
        assert_eq!(RuntimeValue::String("hi".into()).to_string(), "hi");
        assert_eq!(RuntimeValue::Bool(true).to_string(), "true");
        assert_eq!(RuntimeValue::Number(42).to_string(), "42");
        assert_eq!(RuntimeValue::Elements(vec![]).to_string(), "<0 elements>");
    }

    #[test]
    fn test_runtime_value_as_str() {
        let v = RuntimeValue::String("hello".into());
        assert_eq!(v.as_str().unwrap(), "hello");

        let v = RuntimeValue::Number(42);
        assert!(v.as_str().is_err());
    }

    #[test]
    fn test_runtime_value_as_bool() {
        let v = RuntimeValue::Bool(true);
        assert!(v.as_bool().unwrap());

        let v = RuntimeValue::Null;
        assert!(v.as_bool().is_err());
    }

    #[test]
    fn test_parse_key_valid() {
        assert_eq!(parse_key("Enter").unwrap(), Key::Enter);
        assert_eq!(parse_key("Tab").unwrap(), Key::Tab);
        assert_eq!(parse_key("Escape").unwrap(), Key::Escape);
        assert_eq!(parse_key("ArrowDown").unwrap(), Key::ArrowDown);
    }

    #[test]
    fn test_parse_key_invalid() {
        assert!(parse_key("NotAKey").is_err());
    }
}
