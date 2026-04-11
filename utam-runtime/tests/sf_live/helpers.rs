//! Test helpers that call page object methods and produce Allure steps.
//!
//! Each helper calls the actual page object runtime (not a mock), records
//! timing, validates the return value, and produces a structured Allure step.

use std::collections::HashMap;

use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::{DynamicPageObject, PageObjectRuntime};
use utam_test::allure::{AllureStatus, AllureStep, StepBuilder};

/// Call a compose method on a loaded page object and produce an Allure step.
///
/// - Calls `po.call_method(name, args)` against the live DOM.
/// - Runs `validate` against the returned `RuntimeValue`.
/// - The step records the method name, returned value, and pass/fail.
pub async fn run_method(
    po: &DynamicPageObject,
    name: &str,
    args: &HashMap<String, RuntimeValue>,
    validate: fn(&RuntimeValue) -> Result<(), String>,
) -> AllureStep {
    let step = StepBuilder::start(format!("call_method(\"{name}\")"));
    match po.call_method(name, args).await {
        Ok(value) => {
            let desc = format!("{value}");
            match validate(&value) {
                Ok(()) => step.parameter("returned", desc).finish(AllureStatus::Passed),
                Err(e) => step.parameter("returned", desc).finish_err(e),
            }
        }
        Err(e) => step.finish_err(format!("{e}")),
    }
}

/// Call a compose method that is allowed to fail (nullable element, optional feature).
///
/// Returns `Passed` on success, `Skipped` on expected failure.
pub async fn run_method_nullable(
    po: &DynamicPageObject,
    name: &str,
    args: &HashMap<String, RuntimeValue>,
) -> AllureStep {
    let step = StepBuilder::start(format!("call_method(\"{name}\") [nullable]"));
    match po.call_method(name, args).await {
        Ok(value) => {
            step.parameter("returned", format!("{value}")).finish(AllureStatus::Passed)
        }
        Err(e) => {
            // Expected failure for nullable elements — not a test failure
            let mut finished = step.parameter("error", format!("{e}")).finish(AllureStatus::Skipped);
            finished.status_details = Some(utam_test::allure::AllureStatusDetails {
                message: Some(format!("Nullable element not present: {e}")),
                trace: None,
                known: Some(true),
                muted: None,
                flaky: None,
            });
            finished
        }
    }
}

/// Resolve a named element from a loaded page object and produce an Allure step.
///
/// Calls `po.get_element(name, args)` then runs each action in `actions`
/// against the resolved element, producing sub-steps for each.
pub async fn run_element_with_actions(
    po: &DynamicPageObject,
    element_name: &str,
    args: &HashMap<String, RuntimeValue>,
    actions: &[(&str, &[RuntimeValue], fn(&RuntimeValue) -> Result<(), String>)],
) -> AllureStep {
    let step = StepBuilder::start(format!("get_element(\"{element_name}\") + actions"));

    let el = match po.get_element(element_name, args).await {
        Ok(el) => el,
        Err(e) => return step.finish_err(format!("get_element failed: {e}")),
    };

    let mut step = step.parameter("capability", el.type_name());

    for (action, action_args, validate) in actions {
        use utam_runtime::element::ElementRuntime;
        let sub = StepBuilder::start(format!("execute(\"{action}\")"));
        let sub = match el.execute(action, action_args).await {
            Ok(value) => {
                let desc = format!("{value}");
                match validate(&value) {
                    Ok(()) => sub.parameter("returned", desc).finish(AllureStatus::Passed),
                    Err(e) => sub.parameter("returned", desc).finish_err(e),
                }
            }
            Err(e) => sub.finish_err(format!("{e}")),
        };
        step = step.sub_step(sub);
    }

    step.finish(AllureStatus::Passed)
}

/// Resolve a named element and verify it exists.
pub async fn run_get_element(
    po: &DynamicPageObject,
    element_name: &str,
    args: &HashMap<String, RuntimeValue>,
) -> AllureStep {
    let step = StepBuilder::start(format!("get_element(\"{element_name}\")"));
    match po.get_element(element_name, args).await {
        Ok(el) => step.parameter("capability", el.type_name()).finish(AllureStatus::Passed),
        Err(e) => step.finish_err(format!("{e}")),
    }
}

// ---------------------------------------------------------------------------
// Common validators
// ---------------------------------------------------------------------------

/// Accepts any value (no validation).
pub fn expect_any(_: &RuntimeValue) -> Result<(), String> {
    Ok(())
}

/// Validates the return is a String.
pub fn expect_string(v: &RuntimeValue) -> Result<(), String> {
    match v {
        RuntimeValue::String(_) => Ok(()),
        other => Err(format!("expected String, got {other:?}")),
    }
}

/// Validates the return is a Bool.
pub fn expect_bool(v: &RuntimeValue) -> Result<(), String> {
    match v {
        RuntimeValue::Bool(_) => Ok(()),
        other => Err(format!("expected Bool, got {other:?}")),
    }
}

/// Validates the return is not Null.
pub fn expect_not_null(v: &RuntimeValue) -> Result<(), String> {
    match v {
        RuntimeValue::Null => Err("expected non-null, got Null".into()),
        _ => Ok(()),
    }
}

/// Validates the return is Null (void action).
pub fn expect_null(v: &RuntimeValue) -> Result<(), String> {
    match v {
        RuntimeValue::Null => Ok(()),
        other => Err(format!("expected Null (void action), got {other:?}")),
    }
}
