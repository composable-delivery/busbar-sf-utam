//! Generic page object test runner.
//!
//! Given a page object name, loads it and exercises every declared method
//! and element against the live DOM.  Produces one `AllureTestResult` per
//! page object with nested steps for each method and element.
//!
//! Method arguments are synthesized from the declared UTAM type (or
//! overridden from a small curated map for methods that need real values).
//! Return values are validated against the declared `returnType`.
//!
//! This turns the 1500 declarative page objects into ~1500 potential
//! test cases without any bespoke per-page-object test code.

use std::collections::HashMap;

use super::session::SalesforceSession;
use super::synth::{override_args, override_element_args, synth_args, validate_return};
use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::{DynamicPageObject, MethodInfo, PageObjectRuntime};
use utam_test::allure::*;

/// Outcome of testing a single page object — used for summary reporting.
#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub methods_passed: usize,
    pub methods_failed: usize,
    pub methods_skipped: usize,
    pub elements_passed: usize,
    pub elements_failed: usize,
    pub loaded: bool,
}

impl Outcome {
    fn broken() -> Self {
        Self {
            methods_passed: 0,
            methods_failed: 0,
            methods_skipped: 0,
            elements_passed: 0,
            elements_failed: 0,
            loaded: false,
        }
    }
}

/// Exercise a page object generically: load it, call every method, resolve
/// every element, and produce an Allure result with per-method steps.
///
/// Returns `(AllureTestResult, Outcome)`.  The result captures the full
/// trace; the Outcome is a compact summary used to build a coverage report.
pub async fn test_page_object(
    session: &SalesforceSession,
    po_name: &str,
    page_context: &str,
) -> (AllureTestResult, Outcome) {
    let builder = TestResultBuilder::new(format!("{po_name}"))
        .full_name(format!("salesforce_live::generic::{page_context}::{po_name}"))
        .description(format!(
            "Generic coverage test for {po_name} on the {page_context} page. \
             Calls every method with synthesized arguments (or overrides for \
             known cases), resolves every public element, and validates return \
             types against the declared UTAM schema."
        ))
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Coverage")
        .label("story", po_name)
        .label("suite", format!("Generic — {page_context}"))
        .label("severity", "normal")
        .parameter("driver", session.driver_name())
        .parameter("page_object", po_name)
        .parameter("page_context", page_context);

    // ── Load ────────────────────────────────────────────────────────────
    let po = match session.load_page_object(po_name).await {
        Ok(po) => po,
        Err(e) => {
            let result = builder.finish_err(
                AllureStatus::Broken,
                format!("page object failed to load: {e}"),
                None,
            );
            return (result, Outcome::broken());
        }
    };

    let mut builder = builder.step(
        StepBuilder::start("load page object")
            .parameter("method_count", po.method_signatures().len().to_string())
            .parameter("element_count", po.element_names().len().to_string())
            .finish(AllureStatus::Passed),
    );

    let mut outcome = Outcome {
        methods_passed: 0,
        methods_failed: 0,
        methods_skipped: 0,
        elements_passed: 0,
        elements_failed: 0,
        loaded: true,
    };

    // ── Exercise every method ──────────────────────────────────────────
    for method_info in po.method_signatures() {
        let step = exercise_method(&po, po_name, &method_info).await;
        match step.status {
            AllureStatus::Passed => outcome.methods_passed += 1,
            AllureStatus::Skipped => outcome.methods_skipped += 1,
            _ => outcome.methods_failed += 1,
        }
        builder = builder.step(step);
    }

    // ── Exercise every public element ──────────────────────────────────
    for element_name in po.element_names() {
        let step = exercise_element(&po, po_name, element_name).await;
        match step.status {
            AllureStatus::Passed => outcome.elements_passed += 1,
            _ => outcome.elements_failed += 1,
        }
        builder = builder.step(step);
    }

    (builder.finish_from_steps(), outcome)
}

/// Call a single method, synthesizing or overriding its arguments.
async fn exercise_method(
    po: &DynamicPageObject,
    po_name: &str,
    info: &MethodInfo,
) -> AllureStep {
    let step = StepBuilder::start(format!("method: {}", info.name));

    // Use curated overrides first, fall back to synthesized defaults.
    let args =
        override_args(po_name, &info.name).unwrap_or_else(|| synth_args(info));

    let arg_desc = format_args(&args);

    match po.call_method(&info.name, &args).await {
        Ok(value) => {
            let returned_desc = format!("{value}");
            let step = step
                .parameter("args", arg_desc)
                .parameter("returned", returned_desc);

            // Validate return type if declared
            if let Some(rt) = &info.return_type {
                match validate_return(&value, rt) {
                    Ok(()) => step.parameter("returnType", rt.clone()).finish(AllureStatus::Passed),
                    Err(e) => step
                        .parameter("returnType", rt.clone())
                        .finish_err(format!("return type mismatch: {e}")),
                }
            } else {
                step.finish(AllureStatus::Passed)
            }
        }
        Err(e) => {
            // Classify the error so Allure categorization works
            let msg = format!("{e}");
            let mut finished =
                step.parameter("args", arg_desc).finish_err(msg.clone());

            // Mark ArgumentMissing as Skipped rather than Failed — we can't
            // synthesize compose-level args generically.  These need override.
            if msg.contains("ArgumentMissing") || msg.contains("argument") {
                finished.status = AllureStatus::Skipped;
                finished.status_details = Some(AllureStatusDetails {
                    message: Some(format!("Method needs specific args: {msg}")),
                    trace: None,
                    known: Some(true),
                    muted: None,
                    flaky: None,
                });
            }
            finished
        }
    }
}

/// Resolve a single element, synthesizing or overriding its arguments.
async fn exercise_element(
    po: &DynamicPageObject,
    po_name: &str,
    element_name: &str,
) -> AllureStep {
    let step = StepBuilder::start(format!("element: {element_name}"));

    let args = override_element_args(po_name, element_name).unwrap_or_default();

    match po.get_element(element_name, &args).await {
        Ok(el) => step
            .parameter("capability", el.type_name())
            .parameter("args", format_args(&args))
            .finish(AllureStatus::Passed),
        Err(e) => {
            let mut finished =
                step.parameter("args", format_args(&args)).finish_err(format!("{e}"));
            let msg = finished
                .status_details
                .as_ref()
                .and_then(|d| d.message.clone())
                .unwrap_or_default();
            if msg.contains("ArgumentMissing") || msg.contains("argument") {
                finished.status = AllureStatus::Skipped;
                if let Some(sd) = finished.status_details.as_mut() {
                    sd.known = Some(true);
                }
            }
            finished
        }
    }
}

fn format_args(args: &HashMap<String, RuntimeValue>) -> String {
    if args.is_empty() {
        return "<none>".into();
    }
    let mut pairs: Vec<String> =
        args.iter().map(|(k, v)| format!("{k}={v}")).collect();
    pairs.sort();
    pairs.join(", ")
}
