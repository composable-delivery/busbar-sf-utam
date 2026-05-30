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
    collect_required_args, element_selector_arg_names, member_skip_reason, method_string_arg_names,
    override_args, override_element_args, smart_default, synth_args, synth_element_args,
    validate_return,
};
use utam_runtime::driver::Selector;
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
    pub elements_skipped: usize,
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
            elements_skipped: 0,
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
            "Generic coverage (contract/smoke) test for {po_name} on the {page_context} page. \
             For each method: collects required args by walking the compose tree, uses a curated \
             override when one exists else synthesizes defaults, EXECUTES the method against the \
             live DOM, and type-checks the return value against the declared returnType. For each \
             public element: resolves it against the live DOM and binds its capability. \
             This proves the page-object↔DOM binding works against a real org. It does NOT assert \
             behavioral outcomes (navigation, state changes, value contents); parameterized \
             actions without a curated arg value are Skipped (see each step's `verification` \
             parameter and skip reasons) rather than driven with a placeholder."
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
            let mut result = builder.finish_err(
                AllureStatus::Broken,
                format!("page object failed to load: {e}"),
                None,
            );
            // Screenshot the page state that defeated the load so the
            // Allure report shows what the DOM looked like.
            if let Some(att) = capture(session, &format!("BROKEN {po_name} — load failed")).await
            {
                result.attachments.push(att);
            }
            return (result, outcome);
        }
    };

    let mut builder = builder.step(
        StepBuilder::start("load page object")
            .parameter("method_count", po.method_signatures().len().to_string())
            .parameter("element_count", po.element_names().len().to_string())
            .finish(AllureStatus::Passed),
    );

    // Per-page-object context screenshot: one shot of the live page the
    // moment this PO loaded, attached to the result for visual reference.
    if let Some(att) = capture(session, &format!("{po_name} — loaded")).await {
        builder = builder.attachment(att);
    }

    let mut outcome = Outcome::empty();

    // ── Exercise every method ──────────────────────────────────────────
    for method_info in po.method_signatures() {
        let step = exercise_method(&po, po_name, &method_info, &mut outcome, session).await;
        match step.status {
            AllureStatus::Passed => outcome.methods_passed += 1,
            AllureStatus::Skipped => outcome.methods_skipped += 1,
            _ => outcome.methods_failed += 1,
        }
        builder = builder.step(step);
    }

    // ── Exercise every public element ──────────────────────────────────
    for element_name in po.element_names() {
        let step = exercise_element(&po, po_name, element_name, &mut outcome, session).await;
        match step.status {
            AllureStatus::Passed => outcome.elements_passed += 1,
            AllureStatus::Skipped => outcome.elements_skipped += 1,
            _ => outcome.elements_failed += 1,
        }
        builder = builder.step(step);
    }

    (builder.finish_from_steps(), outcome)
}

/// Capture a screenshot of the current browser state and write it as an
/// Allure attachment.  Returns `None` (and logs a warning) on any failure —
/// screenshot capture must never mask or replace the real test outcome.
async fn capture(session: &SalesforceSession, name: &str) -> Option<AllureAttachment> {
    match session.driver.screenshot_png().await {
        Ok(png) => match session.allure.write_attachment(name, "image/png", &png) {
            Ok(att) => Some(att),
            Err(e) => {
                eprintln!("  WARNING: failed to write screenshot '{name}': {e}");
                None
            }
        },
        Err(e) => {
            eprintln!("  WARNING: failed to capture screenshot '{name}': {e}");
            None
        }
    }
}

/// Finish a step as `Skipped` with a reason, marked `known` so Allure treats
/// it as an intentional skip rather than an unexpected gap.  This is how the
/// harness records "out of scope for a standard scratch org" — visible and
/// reasoned, never a silent pass and never a fabricated failure.
fn skipped_step(step: StepBuilder, reason: &str) -> AllureStep {
    let mut s = step.parameter("skip_reason", reason.to_string()).finish(AllureStatus::Skipped);
    s.status_details = Some(AllureStatusDetails {
        message: Some(reason.to_string()),
        trace: None,
        known: Some(true),
        muted: None,
        flaky: None,
    });
    s
}

async fn exercise_method(
    po: &DynamicPageObject,
    po_name: &str,
    info: &MethodInfo,
    outcome: &mut Outcome,
    session: &SalesforceSession,
) -> AllureStep {
    let step = StepBuilder::start(format!("method: {}", info.name));

    // Honest scoping (never a silent pass, never a fabricated failure):
    //  1. members of standard POs that need an unavailable feature → Skipped.
    if let Some(reason) = member_skip_reason(po_name, &info.name) {
        return skipped_step(step, reason);
    }
    //  2. methods that need a real string value with no curated override would
    //     be called with "" (a guaranteed not-found that isn't a real test) →
    //     Skipped with the arg(s) we still need to curate.
    if override_args(po_name, &info.name).is_none() {
        let ast = po.ast();
        if let Some(method_ast) = ast.methods.iter().find(|m| m.name == info.name) {
            let needed = method_string_arg_names(method_ast, ast);
            if !needed.is_empty() {
                return skipped_step(step, &format!("needs curated arg(s): {}", needed.join(", ")));
            }
        }
    }

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
                    Ok(()) => step
                        .parameter("returnType", rt.clone())
                        .parameter(
                            "verification",
                            format!(
                                "executed against live DOM; return value type-checked against \
                                 declared returnType '{rt}'. NOT asserted: behavioral outcome \
                                 (e.g. navigation/state change) or the returned value's contents."
                            ),
                        )
                        .finish(AllureStatus::Passed),
                    Err(e) => {
                        outcome.record_failure(FailureKind::ReturnTypeMismatch);
                        step.parameter("returnType", rt.clone())
                            .parameter("failure_kind", "ReturnTypeMismatch")
                            .finish_err(format!("return type mismatch: {e}"))
                    }
                }
            } else {
                step.parameter(
                    "verification",
                    "executed against live DOM without error; method declares no returnType, so \
                     nothing is type-checked. NOT asserted: behavioral outcome or any result value."
                        .to_string(),
                )
                .finish(AllureStatus::Passed)
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
            } else if let Some(att) =
                capture(session, &format!("FAIL {po_name}::{} [{}]", info.name, kind.name())).await
            {
                // Real method failure — attach the page state at failure time.
                finished.attachments.push(att);
            }
            finished
        }
    }
}

/// Elements that can only be resolved after a utility-bar panel is opened.
fn element_needs_open_utility_panel(po_name: &str, element_name: &str) -> bool {
    matches!((po_name, element_name), ("global/utilityBarContainer", "utilityBarItemPanelHeader"))
}

/// Best-effort precondition: open a utility-bar panel so a panel-scoped
/// element can be asserted.
///
/// Returns `Ok(())` once a utility item has been clicked (panel opening),
/// or `Err(reason)` when there's no utility bar to open — the caller turns
/// that into a clean Skip. This keeps the harness honest in both worlds:
/// with a utility bar configured, the panel header is exercised for real;
/// without one, it's a reasoned skip rather than a fabricated failure.
async fn open_utility_panel(session: &SalesforceSession) -> Result<(), String> {
    let buttons = session
        .driver
        .find_elements(&Selector::Css("li.slds-utility-bar__item button".into()))
        .await
        .map_err(|e| format!("{e}"))?;
    let Some(button) = buttons.into_iter().next() else {
        return Err(
            "requires an open utility panel; no utility bar present in this org/app".to_string()
        );
    };
    button.click().await.map_err(|e| format!("failed to click utility item: {e}"))?;
    // Give the panel a moment to render before the element is resolved.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    Ok(())
}

async fn exercise_element(
    po: &DynamicPageObject,
    po_name: &str,
    element_name: &str,
    outcome: &mut Outcome,
    session: &SalesforceSession,
) -> AllureStep {
    let step = StepBuilder::start(format!("element: {element_name}"));

    // Honest scoping, mirroring exercise_method:
    if let Some(reason) = member_skip_reason(po_name, element_name) {
        return skipped_step(step, reason);
    }

    // Some elements only exist after an interaction (e.g. a utility-bar panel
    // header exists only once a panel is open). Drive that interaction first.
    // If the precondition can't be met in this org/context (e.g. there's no
    // utility bar to open), skip cleanly with the reason rather than failing
    // on an element that legitimately can't be present.
    if element_needs_open_utility_panel(po_name, element_name) {
        if let Err(reason) = open_utility_panel(session).await {
            return skipped_step(step, &reason);
        }
    }

    // A parameterized selector with no curated value would resolve to e.g.
    // `[data-id='']` and never match — skip instead of fabricating "".
    if override_element_args(po_name, element_name).is_none() {
        let sel_args = element_selector_arg_names(po.ast(), element_name);
        if !sel_args.is_empty() {
            return skipped_step(
                step,
                &format!("needs curated selector arg(s): {}", sel_args.join(", ")),
            );
        }
    }

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
            .parameter(
                "verification",
                "element resolved against the live DOM (selector matched a present node) and its \
                 declared capability was bound. NOT asserted: visibility, contents, or any \
                 interaction with the element."
                    .to_string(),
            )
            .finish(AllureStatus::Passed),
        Err(e) => {
            let msg = format!("{e}");
            let kind = classify(&msg);

            // NullableAbsent is the spec-correct outcome for `nullable: true`
            // elements that aren't in the DOM — it's NOT a failure.  Report
            // it as Passed with a note.
            if kind == FailureKind::NullableAbsent {
                return step
                    .parameter("args", format_args(&args))
                    .parameter("note", "nullable element absent (expected)")
                    .finish(AllureStatus::Passed);
            }

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
            } else if let Some(att) = capture(
                session,
                &format!("FAIL {po_name} element '{element_name}' [{}]", kind.name()),
            )
            .await
            {
                // Real element failure — attach the page state at failure time.
                finished.attachments.push(att);
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
