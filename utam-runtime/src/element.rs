//! Dynamic element wrapper and runtime value types
//!
//! [`DynamicElement`] wraps a driver-agnostic [`ElementHandle`] and dispatches
//! UTAM action names to the correct methods at runtime.
//!
//! [`RuntimeValue`] is the dynamically-typed currency flowing through the interpreter.

use std::fmt;

use async_trait::async_trait;

use crate::driver::{ElementHandle, Selector};
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
    fn supported_actions(&self) -> Vec<&'static str>;
}

/// The declared capability level of an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementCapability {
    Base,
    Clickable,
    Editable,
    Draggable,
}

impl ElementCapability {
    /// Determine capability from UTAM type list (e.g. `["clickable", "editable"]`).
    ///
    /// Picks the most capable: draggable > editable > clickable > base.
    pub fn from_type_list(types: &[String]) -> Self {
        if types.iter().any(|t| t == "draggable") {
            ElementCapability::Draggable
        } else if types.iter().any(|t| t == "editable") {
            ElementCapability::Editable
        } else if types.iter().any(|t| t == "clickable") {
            ElementCapability::Clickable
        } else {
            ElementCapability::Base
        }
    }
}

/// Driver-agnostic element wrapper.
///
/// Holds an [`ElementHandle`] (from any driver backend) plus the
/// declared capability from the UTAM JSON. Dispatches action name
/// strings to the appropriate `ElementHandle` methods at runtime.
#[derive(Debug)]
pub struct DynamicElement {
    handle: Box<dyn ElementHandle>,
    capability: ElementCapability,
}

impl Clone for DynamicElement {
    fn clone(&self) -> Self {
        Self { handle: self.handle.clone_handle(), capability: self.capability }
    }
}

impl DynamicElement {
    /// Create from an element handle and UTAM type list
    pub fn new(handle: Box<dyn ElementHandle>, types: &[String]) -> Self {
        Self { handle, capability: ElementCapability::from_type_list(types) }
    }

    /// Create a base-capability element
    pub fn base(handle: Box<dyn ElementHandle>) -> Self {
        Self { handle, capability: ElementCapability::Base }
    }

    /// Get the underlying element handle
    pub fn handle(&self) -> &dyn ElementHandle {
        &*self.handle
    }

    /// Get the capability level
    pub fn capability(&self) -> ElementCapability {
        self.capability
    }

    /// What kind of element is this?
    pub fn type_name(&self) -> &'static str {
        match self.capability {
            ElementCapability::Base => "base",
            ElementCapability::Clickable => "clickable",
            ElementCapability::Editable => "editable",
            ElementCapability::Draggable => "draggable",
        }
    }

    /// Check if this element supports a given action
    pub fn supports_action(&self, action: &str) -> bool {
        if BASE_ACTIONS.contains(&action) {
            return true;
        }
        match self.capability {
            ElementCapability::Draggable => {
                DRAG_ACTIONS.contains(&action)
                    || CLICK_ACTIONS.contains(&action)
                    || EDIT_ACTIONS.contains(&action)
            }
            ElementCapability::Editable => {
                EDIT_ACTIONS.contains(&action) || CLICK_ACTIONS.contains(&action)
            }
            ElementCapability::Clickable => CLICK_ACTIONS.contains(&action),
            ElementCapability::Base => false,
        }
    }
}

/// All actions available on every element
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
    "waitForVisible",
    "waitForInvisible",
    "waitForAbsence",
    "waitForEnabled",
    "containsElement",
];

const CLICK_ACTIONS: &[&str] = &["click", "doubleClick", "rightClick", "clickAndHold"];
const EDIT_ACTIONS: &[&str] = &["clear", "setText", "clearAndType", "press"];
const DRAG_ACTIONS: &[&str] = &["dragAndDropByOffset"];

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

#[async_trait]
impl ElementRuntime for DynamicElement {
    async fn execute(&self, action: &str, args: &[RuntimeValue]) -> RuntimeResult<RuntimeValue> {
        let h = &*self.handle;

        // -- Base actions (all elements) --
        match action {
            "getText" => return Ok(RuntimeValue::String(h.text().await?)),
            "getAttribute" => {
                let name = args.first().map(|a| a.as_str()).transpose()?.unwrap_or("value");
                return Ok(RuntimeValue::String(h.attribute(name).await?.unwrap_or_default()));
            }
            "getClassAttribute" => return Ok(RuntimeValue::String(h.class_name().await?)),
            "getCssPropertyValue" => {
                let name = require_str_arg(args, 0, "getCssPropertyValue")?;
                return Ok(RuntimeValue::String(h.css_value(name).await?));
            }
            "getTitle" => return Ok(RuntimeValue::String(h.title().await?)),
            "getValue" => return Ok(RuntimeValue::String(h.property_value().await?)),
            "isEnabled" => return Ok(RuntimeValue::Bool(h.is_enabled().await?)),
            "isFocused" => return Ok(RuntimeValue::Bool(h.is_focused().await?)),
            "isPresent" => return Ok(RuntimeValue::Bool(h.is_present().await?)),
            "isVisible" => return Ok(RuntimeValue::Bool(h.is_displayed().await?)),
            "focus" => {
                h.focus().await?;
                return Ok(RuntimeValue::Null);
            }
            "blur" => {
                h.blur().await?;
                return Ok(RuntimeValue::Null);
            }
            "scrollIntoView" => {
                h.scroll_into_view().await?;
                return Ok(RuntimeValue::Null);
            }
            "waitForVisible" => {
                // Poll until displayed, with timeout
                let handle = self.handle.clone_handle();
                utam_core::wait::wait_for(
                    || async {
                        match handle.is_displayed().await {
                            Ok(true) => Ok(Some(())),
                            _ => Ok(None),
                        }
                    },
                    &utam_core::wait::WaitConfig::default(),
                    "element to become visible",
                )
                .await?;
                return Ok(RuntimeValue::Null);
            }
            "waitForInvisible" => {
                let handle = self.handle.clone_handle();
                utam_core::wait::wait_for(
                    || async {
                        match handle.is_displayed().await {
                            Ok(false) => Ok(Some(())),
                            _ => Ok(None),
                        }
                    },
                    &utam_core::wait::WaitConfig::default(),
                    "element to become invisible",
                )
                .await?;
                return Ok(RuntimeValue::Null);
            }
            "waitForAbsence" => {
                let handle = self.handle.clone_handle();
                utam_core::wait::wait_for(
                    || async {
                        match handle.is_present().await {
                            Ok(false) => Ok(Some(())),
                            _ => Ok(None),
                        }
                    },
                    &utam_core::wait::WaitConfig::default(),
                    "element to be absent",
                )
                .await?;
                return Ok(RuntimeValue::Null);
            }
            "waitForEnabled" => {
                let handle = self.handle.clone_handle();
                utam_core::wait::wait_for(
                    || async {
                        match handle.is_enabled().await {
                            Ok(true) => Ok(Some(())),
                            _ => Ok(None),
                        }
                    },
                    &utam_core::wait::WaitConfig::default(),
                    "element to become enabled",
                )
                .await?;
                return Ok(RuntimeValue::Null);
            }
            "containsElement" => {
                let css = require_str_arg(args, 0, "containsElement")?;
                let found = h.find_elements(&Selector::Css(css.to_string())).await?;
                return Ok(RuntimeValue::Bool(!found.is_empty()));
            }
            _ => {} // fall through to capability-specific actions
        }

        // -- Capability-specific actions --
        if !self.supports_action(action) {
            return Err(RuntimeError::UnsupportedAction {
                action: action.to_string(),
                element_type: self.type_name().to_string(),
            });
        }

        match action {
            // Click actions
            "click" => {
                h.click().await?;
                Ok(RuntimeValue::Null)
            }
            "doubleClick" => {
                h.double_click().await?;
                Ok(RuntimeValue::Null)
            }
            "rightClick" => {
                h.right_click().await?;
                Ok(RuntimeValue::Null)
            }
            "clickAndHold" => {
                h.click_and_hold().await?;
                Ok(RuntimeValue::Null)
            }

            // Edit actions
            "clear" => {
                h.clear().await?;
                Ok(RuntimeValue::Null)
            }
            "setText" => {
                let text = require_str_arg(args, 0, "setText")?;
                h.send_keys(text).await?;
                Ok(RuntimeValue::Null)
            }
            "clearAndType" => {
                let text = require_str_arg(args, 0, "clearAndType")?;
                h.clear().await?;
                h.send_keys(text).await?;
                Ok(RuntimeValue::Null)
            }
            "press" => {
                let key = require_str_arg(args, 0, "press")?;
                h.press_key(key).await?;
                Ok(RuntimeValue::Null)
            }

            // Drag actions
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
                h.drag_by_offset(x, y).await?;
                Ok(RuntimeValue::Null)
            }

            _ => Err(RuntimeError::UnsupportedAction {
                action: action.to_string(),
                element_type: self.type_name().to_string(),
            }),
        }
    }

    fn supported_actions(&self) -> Vec<&'static str> {
        let mut actions: Vec<&str> = BASE_ACTIONS.to_vec();
        match self.capability {
            ElementCapability::Draggable => {
                actions.extend_from_slice(CLICK_ACTIONS);
                actions.extend_from_slice(EDIT_ACTIONS);
                actions.extend_from_slice(DRAG_ACTIONS);
            }
            ElementCapability::Editable => {
                actions.extend_from_slice(CLICK_ACTIONS);
                actions.extend_from_slice(EDIT_ACTIONS);
            }
            ElementCapability::Clickable => {
                actions.extend_from_slice(CLICK_ACTIONS);
            }
            ElementCapability::Base => {}
        }
        actions
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
    fn test_element_capability_from_types() {
        assert_eq!(
            ElementCapability::from_type_list(&["clickable".into()]),
            ElementCapability::Clickable
        );
        assert_eq!(
            ElementCapability::from_type_list(&["clickable".into(), "editable".into()]),
            ElementCapability::Editable
        );
        assert_eq!(
            ElementCapability::from_type_list(&["draggable".into(), "clickable".into()]),
            ElementCapability::Draggable
        );
        assert_eq!(
            ElementCapability::from_type_list(&["actionable".into()]),
            ElementCapability::Base
        );
        assert_eq!(ElementCapability::from_type_list(&[]), ElementCapability::Base);
    }
}
