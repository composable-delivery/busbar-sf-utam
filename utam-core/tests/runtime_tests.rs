//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits and element wrappers.

mod common;

use std::time::Duration;
use utam_core::prelude::*;

#[test]
fn test_error_types() {
    // Test that error types can be constructed and displayed
    let error1 = UtamError::ElementNotFound {
        name: "testButton".to_string(),
        selector: ".test".to_string(),
    };
    assert!(error1.to_string().contains("testButton"));
    assert!(error1.to_string().contains(".test"));

    let error2 = UtamError::Timeout { condition: "element to be visible".to_string() };
    assert!(error2.to_string().contains("Timeout"));

    let error3 = UtamError::ShadowRootNotFound { element: "myElement".to_string() };
    assert!(error3.to_string().contains("myElement"));

    let error4 = UtamError::InvalidSelector { selector: "invalid[".to_string() };
    assert!(error4.to_string().contains("invalid["));

    let error5 = UtamError::FrameNotFound { name: "myFrame".to_string() };
    assert!(error5.to_string().contains("myFrame"));

    let error6 =
        UtamError::AssertionFailed { expected: "foo".to_string(), actual: "bar".to_string() };
    assert!(error6.to_string().contains("foo"));
    assert!(error6.to_string().contains("bar"));
}

#[test]
fn test_prelude_exports() {
    // Test that all expected types are exported from prelude
    // This ensures the public API is stable
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_element_wrappers_compile() {
    // Test that element wrapper types can be constructed
    // This ensures the API is stable and types are exported

    // These would normally be created from actual WebElements,
    // but we're just testing that the types compile and are accessible

    // Note: We can't actually instantiate these without a WebDriver,
    // but we can verify the types are exported and have the expected API
    fn _check_base_element(_elem: BaseElement) {}
    fn _check_clickable_element(_elem: ClickableElement) {}
    fn _check_editable_element(_elem: EditableElement) {}
    fn _check_draggable_element(_elem: DraggableElement) {}
}

#[test]
fn test_traits_compile() {
    // Test that trait bounds compile
    // This ensures traits are exported and have the correct signatures

    fn _check_actionable<T: Actionable>(_elem: T) {}
    fn _check_clickable<T: Clickable>(_elem: T) {}
    fn _check_editable<T: Editable>(_elem: T) {}
    fn _check_draggable<T: Draggable>(_elem: T) {}
}

#[test]
fn test_draggable_method_signatures() {
    // Verify that Draggable trait methods have the correct signatures
    // This is a compile-time test to ensure API stability

    async fn _test_drag_and_drop<T: Draggable>(elem: &T, target: &WebElement) -> UtamResult<()> {
        elem.drag_and_drop(target).await
    }

    async fn _test_drag_and_drop_with_duration<T: Draggable>(
        elem: &T,
        target: &WebElement,
        duration: Duration,
    ) -> UtamResult<()> {
        elem.drag_and_drop_with_duration(target, duration).await
    }

    async fn _test_drag_and_drop_by_offset<T: Draggable>(
        elem: &T,
        x: i64,
        y: i64,
    ) -> UtamResult<()> {
        elem.drag_and_drop_by_offset(x, y).await
    }
}

#[test]
fn test_actionable_method_signatures() {
    // Verify that Actionable trait methods have the correct signatures

    async fn _test_focus<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.focus().await
    }

    async fn _test_blur<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.blur().await
    }

    async fn _test_scroll_into_view<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.scroll_into_view().await
    }

    async fn _test_move_to<T: Actionable>(elem: &T) -> UtamResult<()> {
        elem.move_to().await
    }
}

#[test]
fn test_clickable_method_signatures() {
    // Verify that Clickable trait methods have the correct signatures

    async fn _test_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.click().await
    }

    async fn _test_double_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.double_click().await
    }

    async fn _test_right_click<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.right_click().await
    }

    async fn _test_click_and_hold<T: Clickable>(elem: &T) -> UtamResult<()> {
        elem.click_and_hold().await
    }
}

#[test]
fn test_editable_method_signatures() {
    // Verify that Editable trait methods have the correct signatures

    async fn _test_clear<T: Editable>(elem: &T) -> UtamResult<()> {
        elem.clear().await
    }

    async fn _test_clear_and_type<T: Editable>(elem: &T, text: &str) -> UtamResult<()> {
        elem.clear_and_type(text).await
    }

    async fn _test_set_text<T: Editable>(elem: &T, text: &str) -> UtamResult<()> {
        elem.set_text(text).await
    }

    async fn _test_press<T: Editable>(elem: &T, key: &str) -> UtamResult<()> {
        elem.press(key).await
    }
}
