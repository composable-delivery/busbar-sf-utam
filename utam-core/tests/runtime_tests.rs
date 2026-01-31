//! Integration tests for UTAM core runtime
//!
//! Tests runtime traits, element wrappers, error types, and module exports.

mod common;

use utam_core::prelude::*;

// ========== Error Type Tests ==========

#[test]
fn test_error_types() {
    let _error = UtamError::ElementNotFound {
        name: "testButton".to_string(),
        selector: ".test".to_string(),
    };
}

#[test]
fn test_error_timeout() {
    let error = UtamError::Timeout { condition: "element to be visible".to_string() };
    assert!(format!("{}", error).contains("Timeout"));
    assert!(format!("{}", error).contains("element to be visible"));
}

#[test]
fn test_error_shadow_root_not_found() {
    let error = UtamError::ShadowRootNotFound { element: "custom-element".to_string() };
    assert!(format!("{}", error).contains("Shadow root not found"));
    assert!(format!("{}", error).contains("custom-element"));
}

#[test]
fn test_error_invalid_selector() {
    let error = UtamError::InvalidSelector { selector: ":::invalid".to_string() };
    assert!(format!("{}", error).contains("Invalid selector"));
    assert!(format!("{}", error).contains(":::invalid"));
}

#[test]
fn test_error_frame_not_found() {
    let error = UtamError::FrameNotFound { name: "myFrame".to_string() };
    assert!(format!("{}", error).contains("Frame not found"));
    assert!(format!("{}", error).contains("myFrame"));
}

#[test]
fn test_error_assertion_failed() {
    let error = UtamError::AssertionFailed {
        expected: "visible".to_string(),
        actual: "hidden".to_string(),
    };
    assert!(format!("{}", error).contains("Assertion failed"));
    assert!(format!("{}", error).contains("visible"));
    assert!(format!("{}", error).contains("hidden"));
}

// ========== Prelude and API Export Tests ==========

#[test]
fn test_prelude_exports() {
    let _result: UtamResult<()> = Ok(());
}

#[test]
fn test_prelude_exports_base_element() {
    fn _check_prelude_has_base_element() {
        use utam_core::prelude::BaseElement;
        let _: Option<BaseElement> = None;
    }
}

#[test]
fn test_prelude_exports_element_rectangle() {
    fn _check_prelude_has_element_rectangle() {
        use utam_core::prelude::ElementRectangle;
        let _rect = ElementRectangle::new(0.0, 0.0, 0.0, 0.0);
    }
}

#[test]
fn test_prelude_exports_error_types() {
    fn _check_prelude_has_error_types() {
        use utam_core::prelude::{UtamError, UtamResult};
        let _err: UtamError = UtamError::Timeout { condition: String::new() };
        let _res: UtamResult<()> = Ok(());
    }
}

#[test]
fn test_prelude_exports_traits() {
    fn _check_traits_exported() {
        use utam_core::prelude::{Actionable, Clickable, Draggable, Editable};
        fn _takes_actionable(_: &dyn Actionable) {}
        fn _takes_clickable(_: &dyn Clickable) {}
        fn _takes_editable(_: &dyn Editable) {}
        fn _takes_draggable(_: &dyn Draggable) {}
    }
}

#[test]
fn test_prelude_exports_page_object_traits() {
    fn _check_page_object_traits() {
        // PageObject and RootPageObject require Sized, so they can't be dyn.
        // Just verify they're importable from the prelude.
        use utam_core::prelude::{PageObject, RootPageObject};
        fn _takes_page_object<T: PageObject>() {}
        fn _takes_root_page_object<T: RootPageObject>() {}
    }
}

#[test]
fn test_prelude_exports_wait_types() {
    fn _check_wait_types() {
        use utam_core::prelude::WaitConfig;
        let config = WaitConfig::default();
        let _ = config.timeout;
        let _ = config.poll_interval;
    }
}

#[test]
fn test_prelude_exports_shadow_types() {
    fn _check_shadow_types() {
        use utam_core::prelude::ShadowRoot;
        let _ = std::any::type_name::<ShadowRoot>();
    }
}

#[test]
fn test_prelude_exports_element_wrappers() {
    fn _check_element_wrappers() {
        use utam_core::prelude::{ClickableElement, DraggableElement, EditableElement};
        let _: Option<ClickableElement> = None;
        let _: Option<EditableElement> = None;
        let _: Option<DraggableElement> = None;
    }
}

// ========== ElementRectangle Tests ==========

#[test]
fn test_element_rectangle_creation() {
    let rect = ElementRectangle::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(rect.x, 10.0);
    assert_eq!(rect.y, 20.0);
    assert_eq!(rect.width, 100.0);
    assert_eq!(rect.height, 50.0);
}

#[test]
fn test_element_rectangle_from_thirtyfour_rect() {
    use thirtyfour::ElementRect;

    let tf_rect = ElementRect { x: 5.0, y: 10.0, width: 200.0, height: 100.0 };

    let rect = ElementRectangle::from(tf_rect);
    assert_eq!(rect.x, 5.0);
    assert_eq!(rect.y, 10.0);
    assert_eq!(rect.width, 200.0);
    assert_eq!(rect.height, 100.0);
}

#[test]
fn test_element_rectangle_partial_eq() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect2 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect3 = ElementRectangle::new(1.0, 2.0, 3.0, 5.0);

    assert_eq!(rect1, rect2);
    assert_ne!(rect1, rect3);
}

#[test]
fn test_element_rectangle_debug_impl() {
    let rect = ElementRectangle::new(10.0, 20.0, 30.0, 40.0);
    let debug = format!("{:?}", rect);
    assert!(debug.contains("ElementRectangle"));
}

#[test]
fn test_element_rectangle_with_floats() {
    let rect = ElementRectangle::new(10.5, 20.5, 100.25, 50.75);
    assert_eq!(rect.x, 10.5);
    assert_eq!(rect.y, 20.5);
    assert_eq!(rect.width, 100.25);
    assert_eq!(rect.height, 50.75);
}

#[test]
fn test_element_rectangle_copy_trait() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect2 = rect1; // Uses Copy

    // Both can be used after copy
    assert_eq!(rect1.x, 1.0);
    assert_eq!(rect2.x, 1.0);
}

// ========== BaseElement API Tests ==========

#[test]
fn test_base_element_api_exists() {
    fn _check_api_exists() {
        #[allow(unreachable_code)]
        #[allow(clippy::diverging_sub_expression)]
        {
            let _element: BaseElement = panic!("not meant to run");
            let _ = _element.inner();
        }
    }
}

// ========== UtamResult Tests ==========

#[test]
fn test_utam_result_ok() {
    let result: UtamResult<i32> = Ok(42);
    assert!(result.is_ok());
    if let Ok(value) = result {
        assert_eq!(value, 42);
    }
}

#[test]
fn test_utam_result_err() {
    let result: UtamResult<()> =
        Err(UtamError::ElementNotFound { name: "test".to_string(), selector: ".test".to_string() });
    assert!(result.is_err());
}

#[test]
fn test_utam_result_with_various_types() {
    let result: UtamResult<String> = Ok("test".to_string());
    assert!(result.is_ok());

    let result: UtamResult<bool> = Ok(true);
    assert!(result.is_ok());

    let result: UtamResult<Option<String>> = Ok(Some("value".to_string()));
    assert!(result.is_ok());
}

// ========== Error Display Tests ==========

#[test]
fn test_error_display_element_not_found() {
    let error = UtamError::ElementNotFound {
        name: "submitBtn".to_string(),
        selector: "#submit".to_string(),
    };
    let display = format!("{}", error);
    assert!(display.contains("submitBtn"));
    assert!(display.contains("#submit"));
    assert!(display.contains("not found"));
}

#[test]
fn test_error_messages_are_human_readable() {
    let error = UtamError::ElementNotFound {
        name: "submitButton".to_string(),
        selector: "button[type='submit']".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("submitButton"));
    assert!(msg.contains("button[type='submit']"));
    assert!(msg.contains("not found"));

    let error = UtamError::Timeout { condition: "element to be visible".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Timeout"));
    assert!(msg.contains("element to be visible"));

    let error = UtamError::ShadowRootNotFound { element: "customElement".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Shadow root not found"));
    assert!(msg.contains("customElement"));

    let error = UtamError::InvalidSelector { selector: "invalid>>selector".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Invalid selector"));
    assert!(msg.contains("invalid>>selector"));

    let error = UtamError::FrameNotFound { name: "paymentFrame".to_string() };
    let msg = error.to_string();
    assert!(msg.contains("Frame not found"));
    assert!(msg.contains("paymentFrame"));

    let error = UtamError::AssertionFailed {
        expected: "visible".to_string(),
        actual: "hidden".to_string(),
    };
    let msg = error.to_string();
    assert!(msg.contains("Assertion failed"));
    assert!(msg.contains("expected visible"));
    assert!(msg.contains("got hidden"));
}

#[test]
fn test_webdriver_error_conversion() {
    use thirtyfour::error::WebDriverError;

    let wd_error = WebDriverError::ParseError("test parse error".to_string());
    let utam_error: UtamError = wd_error.into();

    let msg = utam_error.to_string();
    assert!(msg.contains("WebDriver error"));
    assert!(msg.contains("parse error"));
}

#[test]
fn test_error_context_preserved() {
    let name = "loginButton".to_string();
    let selector = "#login-btn".to_string();

    let error = UtamError::ElementNotFound { name: name.clone(), selector: selector.clone() };

    let msg = error.to_string();
    assert!(msg.contains(&name));
    assert!(msg.contains(&selector));

    let result: UtamResult<String> = Err(error);
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(msg.contains("loginButton"));
        assert!(msg.contains("#login-btn"));
    }
}

// ========== Additional Error Variant Tests ==========

#[test]
fn test_all_error_variants_constructible() {
    let _e1 = UtamError::ElementNotFound { name: String::new(), selector: String::new() };
    let _e2 = UtamError::Timeout { condition: String::new() };
    let _e3 = UtamError::ShadowRootNotFound { element: String::new() };
    let _e4 = UtamError::InvalidSelector { selector: String::new() };
    let _e5 = UtamError::FrameNotFound { name: String::new() };
    let _e6 = UtamError::AssertionFailed { expected: String::new(), actual: String::new() };
}

#[test]
fn test_error_debug_trait() {
    let error =
        UtamError::ElementNotFound { name: "button".to_string(), selector: ".btn".to_string() };
    let debug = format!("{:?}", error);
    assert!(debug.contains("ElementNotFound"));
    assert!(debug.contains("button"));
    assert!(debug.contains(".btn"));
}

// ========== ElementRectangle Trait Tests ==========

#[test]
fn test_element_rectangle_clone_trait() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    #[allow(clippy::clone_on_copy)]
    let rect2 = rect1.clone();
    assert_eq!(rect1, rect2);
}

#[test]
fn test_element_rectangle_eq_reflexive() {
    let rect = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(rect, rect);
}

#[test]
fn test_element_rectangle_eq_symmetric() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect2 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(rect1, rect2);
    assert_eq!(rect2, rect1);
}

#[test]
fn test_element_rectangle_ne_asymmetric() {
    let rect1 = ElementRectangle::new(1.0, 2.0, 3.0, 4.0);
    let rect2 = ElementRectangle::new(5.0, 6.0, 7.0, 8.0);
    assert_ne!(rect1, rect2);
    assert_ne!(rect2, rect1);
}

// ========== Key Enum Tests ==========

#[test]
fn test_key_conversion() {
    let _: thirtyfour::Key = Key::Enter.into();
    let _: thirtyfour::Key = Key::Tab.into();
    let _: thirtyfour::Key = Key::Escape.into();
    let _: thirtyfour::Key = Key::Backspace.into();
    let _: thirtyfour::Key = Key::Delete.into();
    let _: thirtyfour::Key = Key::ArrowUp.into();
    let _: thirtyfour::Key = Key::ArrowDown.into();
    let _: thirtyfour::Key = Key::ArrowLeft.into();
    let _: thirtyfour::Key = Key::ArrowRight.into();
    let _: thirtyfour::Key = Key::Home.into();
    let _: thirtyfour::Key = Key::End.into();
    let _: thirtyfour::Key = Key::PageUp.into();
    let _: thirtyfour::Key = Key::PageDown.into();
    let _: thirtyfour::Key = Key::Space.into();
}

#[test]
fn test_key_mappings_distinct() {
    let up: thirtyfour::Key = Key::ArrowUp.into();
    let down: thirtyfour::Key = Key::ArrowDown.into();
    let left: thirtyfour::Key = Key::ArrowLeft.into();
    let right: thirtyfour::Key = Key::ArrowRight.into();

    assert_ne!(up.value(), down.value());
    assert_ne!(left.value(), right.value());
}

#[test]
fn test_key_clone_and_copy() {
    let key = Key::Enter;
    let _cloned = key;
    let _copied = key;
    let _used_again = key;
}

#[test]
fn test_key_debug() {
    let key = Key::Enter;
    let debug_str = format!("{:?}", key);
    assert!(debug_str.contains("Enter"));
}

// ========== WaitConfig Tests ==========

#[test]
fn test_wait_config_default() {
    use std::time::Duration;
    let config = WaitConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(config.poll_interval, Duration::from_millis(500));
}

#[test]
fn test_wait_config_custom() {
    use std::time::Duration;
    let config =
        WaitConfig { timeout: Duration::from_secs(30), poll_interval: Duration::from_millis(100) };
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.poll_interval, Duration::from_millis(100));
}

// ========== Trait Object Safety Tests ==========

#[test]
fn test_actionable_trait_is_object_safe() {
    fn _assert_object_safe(_: &dyn Actionable) {}
}

#[test]
fn test_clickable_trait_is_object_safe() {
    fn _assert_object_safe(_: &dyn Clickable) {}
}

#[test]
fn test_editable_trait_is_object_safe() {
    fn _assert_object_safe(_: &dyn Editable) {}
}

#[test]
fn test_draggable_trait_is_object_safe() {
    fn _assert_object_safe(_: &dyn Draggable) {}
}

// ========== Module Path Tests ==========

#[test]
fn test_direct_module_access() {
    // Verify types can be accessed via direct module paths
    use utam_core::elements::BaseElement;
    use utam_core::elements::ClickableElement;
    use utam_core::elements::DraggableElement;
    use utam_core::elements::EditableElement;
    use utam_core::elements::ElementRectangle;
    use utam_core::error::{UtamError, UtamResult};
    use utam_core::shadow::ShadowRoot;
    use utam_core::traits::{
        Actionable, Clickable, Draggable, Editable, Key, PageObject, RootPageObject,
    };
    use utam_core::wait::WaitConfig;

    let _ = std::any::type_name::<BaseElement>();
    let _ = std::any::type_name::<ClickableElement>();
    let _ = std::any::type_name::<EditableElement>();
    let _ = std::any::type_name::<DraggableElement>();
    let _ = std::any::type_name::<ElementRectangle>();
    let _ = std::any::type_name::<ShadowRoot>();
    let _ = std::any::type_name::<WaitConfig>();
    let _ = std::any::type_name::<UtamError>();
    let _ = std::any::type_name::<Key>();
    let _ = std::any::type_name::<UtamResult<()>>();
    let _ = std::any::type_name::<dyn Actionable>();
    let _ = std::any::type_name::<dyn Clickable>();
    let _ = std::any::type_name::<dyn Editable>();
    let _ = std::any::type_name::<dyn Draggable>();
    fn _takes_page_object<T: PageObject>() {}
    fn _takes_root_page_object<T: RootPageObject>() {}
}
