//! Generic page object test runner.
//!
//! Given a page object name, loads it and exercises every declared method
//! and element against the live DOM.  Produces one `AllureTestResult` per
//! page object with nested steps for each method and element.
//!
//! Method arguments are discovered by walking the compose tree (via
//! `synth::collect_required_args`) and synthesized with `smart_default`
//! based on arg name hints.  Curated overrides take precedence for methods
//! where specific values matter.
//!
//! Return values are validated against the declared `returnType`.
//! Failure messages are classified into `FailureKind` categories so the
//! aggregate report can show systemic patterns across hundreds of POs.

use std::collections::HashMap;

use super::failure::{classify, FailureKind};
use super::session::SalesforceSession;
use super::synth::{
    collect_required_args, override_args, override_element_args, smart_default, synth_args,
    synth_element_args, validate_return,
};
use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::{DynamicPageObject, MethodInfo, PageObjectRuntime};
use utam_test::allure::*;

/// Outcome of testing a single page object — used for summary reporting.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub methods_passed: usize,
    pub methods_failed: usize,
    pub methods_skipped: usize,
    pub elements_passed: usize,
    pub elements_failed: usize,
    pub loaded: bool,
    /// Histogram of failure kinds encountered across methods + elements.
    pub failure_kinds: HashMap<FailureKind, usize>,
}

impl Outcome {
    fn empty() -> Self {
        Self {
            methods_passed: 0,
            methods_failed: 0,
            methods_skipped: 0,
            elements_passed: 0,
            elements_failed: 0,
            loaded: true,
            failure_kinds: HashMap::new(),
        }
    }

    fn broken() -> Self {
        let mut o = Self::empty();
        o.loaded = false;
        o
    }

    fn record_failure(&mut self, kind: FailureKind) {
        *self.failure_kinds.entry(kind).or_insert(0) += 1;
    }
}

/// Exercise a page object generically.
pub async fn test_page_object(
    session: &SalesforceSession,
    po_name: &str,
    page_context: &str,
) -> (AllureTestResult, Outcome) {
    let builder = TestResultBuilder::new(po_name.to_string())
        .full_name(format!("salesforce_live::generic::{page_context}::{po_name}"))
        .description(format!(
            "Generic coverage test for {po_name} on the {page_context} page. \
             Collects every required method argument by walking the compose tree, \
             synthesizes smart defaults from arg names, calls every method, \
             resolves every public element, validates return types."
        ))
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Coverage")
        .label("story", po_name)
        .label("suite", format!("Generic — {page_context}"))
        .label("severity", "normal")
        .parameter("driver", session.driver_name())
        .parameter("page_object", po_name)
        .parameter("page_context", page_context);

    let po = match session.load_page_object(po_name).await {
        Ok(po) => po,
        Err(e) => {
            let kind = classify(&e);
            let mut outcome = Outcome::broken();
            outcome.record_failure(kind);
            let result = builder.finish_err(
                AllureStatus::Broken,
                format!("page object failed to load: {e}"),
                None,
            );
            return (result, outcome);
        }
    };

    let mut builder = builder.step(
        StepBuilder::start("load page object")
            .parameter("method_count", po.method_signatures().len().to_string())
            .parameter("element_count", po.element_names().len().to_string())
            .finish(AllureStatus::Passed),
    );

    let mut outcome = Outcome::empty();

    // ── Exercise every method ──────────────────────────────────────────
    for method_info in po.method_signatures() {
        let step = exercise_method(&po, po_name, &method_info, &mut outcome).await;
        match step.status {
            AllureStatus::Passed => outcome.methods_passed += 1,
            AllureStatus::Skipped => outcome.methods_skipped += 1,
            _ => outcome.methods_failed += 1,
        }
        builder = builder.step(step);
    }

    // ── Exercise every public element ──────────────────────────────────
    for element_name in po.element_names() {
        let step = exercise_element(&po, po_name, element_name, &mut outcome).await;
        match step.status {
            AllureStatus::Passed => outcome.elements_passed += 1,
            _ => outcome.elements_failed += 1,
        }
        builder = builder.step(step);
    }

    (builder.finish_from_steps(), outcome)
}

async fn exercise_method(
    po: &DynamicPageObject,
    po_name: &str,
    info: &MethodInfo,
    outcome: &mut Outcome,
) -> AllureStep {
    let step = StepBuilder::start(format!("method: {}", info.name));

    // Build args using the systemic approach:
    // 1. If there's a curated override, use it directly.
    // 2. Otherwise, collect required args from the method AST (walking compose
    //    + element selectors) and synthesize smart defaults.
    let args = if let Some(overridden) = override_args(po_name, &info.name) {
        overridden
    } else {
        // Use the page object's AST to find the method by name
        let ast = po.ast();
        if let Some(method_ast) = ast.methods.iter().find(|m| m.name == info.name) {
            let required = collect_required_args(method_ast, ast);
            let mut args = HashMap::new();
            for arg in &required {
                args.insert(arg.name.clone(), smart_default(&arg.name, &arg.arg_type));
            }
            args
        } else {
            synth_args(info)
        }
    };

    let arg_desc = format_args(&args);

    match po.call_method(&info.name, &args).await {
        Ok(value) => {
            let returned_desc = format!("{value}");
            let step = step.parameter("args", arg_desc).parameter("returned", returned_desc);

            if let Some(rt) = &info.return_type {
                match validate_return(&value, rt) {
                    Ok(()) => step.parameter("returnType", rt.clone()).finish(AllureStatus::Passed),
                    Err(e) => {
                        outcome.record_failure(FailureKind::ReturnTypeMismatch);
                        step.parameter("returnType", rt.clone())
                            .parameter("failure_kind", "ReturnTypeMismatch")
                            .finish_err(format!("return type mismatch: {e}"))
                    }
                }
            } else {
                step.finish(AllureStatus::Passed)
            }
        }
        Err(e) => {
            let msg = format!("{e}");
            let kind = classify(&msg);
            outcome.record_failure(kind);
            let mut finished = step
                .parameter("args", arg_desc)
                .parameter("failure_kind", kind.name())
                .finish_err(msg.clone());

            // Argument-missing failures are marked Skipped rather than Failed —
            // they indicate args we still need to override, not real bugs.
            if kind == FailureKind::ArgumentMissing {
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

async fn exercise_element(
    po: &DynamicPageObject,
    po_name: &str,
    element_name: &str,
    outcome: &mut Outcome,
) -> AllureStep {
    let step = StepBuilder::start(format!("element: {element_name}"));

    // Synthesize element args from its declared selector parameters.
    let args = if let Some(overridden) = override_element_args(po_name, element_name) {
        overridden
    } else {
        synth_element_args(po.ast(), po_name, element_name)
    };

    match po.get_element(element_name, &args).await {
        Ok(el) => step
            .parameter("capability", el.type_name())
            .parameter("args", format_args(&args))
            .finish(AllureStatus::Passed),
        Err(e) => {
            let msg = format!("{e}");
            let kind = classify(&msg);
            outcome.record_failure(kind);
            let mut finished = step
                .parameter("args", format_args(&args))
                .parameter("failure_kind", kind.name())
                .finish_err(msg);

            if kind == FailureKind::ArgumentMissing {
                finished.status = AllureStatus::Skipped;
                finished.status_details = Some(AllureStatusDetails {
                    message: Some("Element needs specific args".into()),
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

fn format_args(args: &HashMap<String, RuntimeValue>) -> String {
    if args.is_empty() {
        return "<none>".into();
    }
    let mut pairs: Vec<String> = args.iter().map(|(k, v)| format!("{k}={v}")).collect();
    pairs.sort();
    pairs.join(", ")
}
