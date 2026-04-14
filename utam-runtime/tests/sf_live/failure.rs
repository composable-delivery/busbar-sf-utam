//! Classify failure messages into categories so we can see systemic patterns.
//!
//! Individual page object failures don't tell us much.  But when we aggregate
//! across hundreds of page objects, patterns emerge: "150 stale selectors"
//! vs "80 timeouts" vs "40 missing args" lead to very different fixes.

use std::fmt;

/// High-level category of a failure message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureKind {
    /// Element selector didn't match anything in the DOM.
    StaleSelector,
    /// waitFor predicate didn't converge within the timeout.
    Timeout,
    /// Method needs an argument that wasn't provided (compose-level param).
    ArgumentMissing,
    /// Action called on an element type that doesn't support it
    /// (e.g. click on a Base element — nullable fallback bug).
    UnsupportedAction,
    /// Method executed but returned the wrong type.
    ReturnTypeMismatch,
    /// Shadow root expected but element has none.
    ShadowRootMissing,
    /// The page object references a custom component that couldn't resolve.
    CustomComponentUnresolved,
    /// Anything we haven't classified yet.
    Other,
}

impl FailureKind {
    pub fn name(self) -> &'static str {
        match self {
            FailureKind::StaleSelector => "StaleSelector",
            FailureKind::Timeout => "Timeout",
            FailureKind::ArgumentMissing => "ArgumentMissing",
            FailureKind::UnsupportedAction => "UnsupportedAction",
            FailureKind::ReturnTypeMismatch => "ReturnTypeMismatch",
            FailureKind::ShadowRootMissing => "ShadowRootMissing",
            FailureKind::CustomComponentUnresolved => "CustomComponentUnresolved",
            FailureKind::Other => "Other",
        }
    }
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Classify an error message into a `FailureKind`.
pub fn classify(message: &str) -> FailureKind {
    let m = message.to_lowercase();

    if m.contains("argumentmissing") || m.contains("argument missing") {
        return FailureKind::ArgumentMissing;
    }
    if m.contains("not supported for element type") {
        return FailureKind::UnsupportedAction;
    }
    if m.contains("return type mismatch") || m.contains("declared returntype") {
        return FailureKind::ReturnTypeMismatch;
    }
    // "scope has no shadow root" → element wasn't in DOM (PO matched a
    // generic selector on the wrong element).  Classify as stale selector.
    if m.contains("parent scope has no shadow root")
        || m.contains("expected to be in a shadow root")
    {
        return FailureKind::StaleSelector;
    }
    if m.contains("has no shadow root") || m.contains("element has no shadow") {
        return FailureKind::ShadowRootMissing;
    }
    if m.contains("pageobjectnotfound") || m.contains("page object not found") {
        return FailureKind::CustomComponentUnresolved;
    }
    if m.contains("timed out")
        || m.contains("timeout")
        || m.contains("waitfor predicate")
    {
        return FailureKind::Timeout;
    }
    if m.contains("no such element")
        || m.contains("element not found")
        || m.contains("elementnotdefined")
        || m.contains("not in the dom")
        || m.contains("not found in dom")
        || m.contains("unable to locate element")
    {
        return FailureKind::StaleSelector;
    }
    FailureKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_stale_selector() {
        assert_eq!(
            classify("WebDriver error: no such element: Unable to locate element"),
            FailureKind::StaleSelector
        );
        assert_eq!(
            classify("ElementNotDefined: foo not in the DOM"),
            FailureKind::StaleSelector
        );
    }

    #[test]
    fn test_classify_timeout() {
        assert_eq!(classify("Timed out after 10s"), FailureKind::Timeout);
        assert_eq!(classify("waitFor predicate did not converge"), FailureKind::Timeout);
    }

    #[test]
    fn test_classify_argument_missing() {
        assert_eq!(
            classify("ArgumentMissing { method: foo, arg_name: bar }"),
            FailureKind::ArgumentMissing
        );
    }

    #[test]
    fn test_classify_unsupported_action() {
        assert_eq!(
            classify("Action 'click' not supported for element type 'base'"),
            FailureKind::UnsupportedAction
        );
    }

    #[test]
    fn test_classify_return_type_mismatch() {
        assert_eq!(
            classify("return type mismatch: declared returnType 'string' but got Bool"),
            FailureKind::ReturnTypeMismatch
        );
    }

    #[test]
    fn test_classify_shadow_root() {
        // Generic shadow-root errors (not our own ElementNotFound).
        assert_eq!(
            classify("Unsupported: element has no shadow root"),
            FailureKind::ShadowRootMissing
        );
    }

    #[test]
    fn test_classify_missing_shadow_is_stale_selector() {
        // Our ElementNotFound error from `parent scope has no shadow root`
        // means the page object matched a generic root on the wrong element.
        // Classify as StaleSelector so the pattern jumps out in aggregate.
        assert_eq!(
            classify("Element 'foo' not found in DOM: parent scope has no shadow root"),
            FailureKind::StaleSelector
        );
        assert_eq!(
            classify(
                "Element 'bar' not found in DOM: ancestor 'p' expected to be in a shadow root, \
                 but its parent has no shadow"
            ),
            FailureKind::StaleSelector
        );
    }

    #[test]
    fn test_classify_other() {
        assert_eq!(classify("some random error"), FailureKind::Other);
    }
}
