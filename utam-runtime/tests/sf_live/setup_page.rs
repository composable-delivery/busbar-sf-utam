//! Test page objects on the Salesforce Setup page.
//!
//! Navigates to Setup Home, then loads page objects that should be
//! present and exercises their methods/elements.
//!
//! Page objects tested:
//!   - setup/setupNavTree — the left-hand navigation tree

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_runtime::page_object::PageObjectRuntime;
use utam_test::allure::*;

pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder = TestResultBuilder::new("setup/setupNavTree — load + introspect")
        .full_name("salesforce_live::setup_page::test_all_methods")
        .description(
            "Navigate to Setup Home, load setup/setupNavTree, introspect its \
             elements and methods, and resolve elements against the live DOM.",
        )
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Methods")
        .label("story", "setup/setupNavTree")
        .label("severity", "normal")
        .label("suite", "Salesforce Live")
        .parameter("driver", session.driver_name())
        .parameter("page_object", "setup/setupNavTree");

    eprintln!("\n=== Test: setup/setupNavTree ===");

    // ── Navigate to Setup ──────────────────────────────────────────────
    let setup_url = format!("{}/lightning/setup/SetupOneHome/home", session.instance_url);
    let nav_step = {
        let s = StepBuilder::start("navigate to Setup Home");
        session.navigate(&setup_url).await;

        let url = session.driver.current_url().await.unwrap_or_default();
        if url.to_lowercase().contains("setup") {
            eprintln!("  On Setup page: {url}");
            s.parameter("url", url).finish(AllureStatus::Passed)
        } else {
            eprintln!("  WARNING: URL doesn't contain 'setup': {url}");
            s.parameter("url", url)
                .finish_err("URL does not contain 'setup'")
        }
    };
    builder = builder.step(nav_step);

    // ── Load setupNavTree ──────────────────────────────────────────────
    let setup_nav = match session.load_page_object("setup/setupNavTree").await {
        Ok(po) => po,
        Err(e) => {
            eprintln!("  FAIL: could not load setup/setupNavTree: {e}");
            return builder.finish_err(AllureStatus::Broken, format!("load failed: {e}"), None);
        }
    };
    eprintln!("  Loaded setup/setupNavTree");

    // ── Introspect: list methods and elements ──────────────────────────
    let introspect_step = {
        let s = StepBuilder::start("introspect methods and elements");
        let methods = setup_nav.method_signatures();
        let elements = setup_nav.element_names();
        let desc = setup_nav.description().unwrap_or_else(|| "<none>".into());

        eprintln!("  Description: {desc}");
        eprintln!("  Methods ({}): {:?}", methods.len(), methods.iter().map(|m| &m.name).collect::<Vec<_>>());
        eprintln!("  Elements ({}): {:?}", elements.len(), elements);

        s.parameter("method_count", methods.len().to_string())
            .parameter("element_count", elements.len().to_string())
            .parameter("description", desc)
            .finish(AllureStatus::Passed)
    };
    builder = builder.step(introspect_step);

    // ── Resolve all public elements ────────────────────────────────────
    let no_args = HashMap::new();
    let elements_step = {
        let s = StepBuilder::start("resolve elements against live DOM");
        let mut s = s;
        for name in setup_nav.element_names() {
            let sub = run_get_element(&setup_nav, name, &no_args).await;
            eprintln!("  element {name}: {:?}", sub.status);
            s = s.sub_step(sub);
        }
        s.finish(AllureStatus::Passed)
    };
    builder = builder.step(elements_step);

    // ── Call all methods (if any) ──────────────────────────────────────
    let method_sigs = setup_nav.method_signatures();
    if !method_sigs.is_empty() {
        let methods_step = {
            let s = StepBuilder::start("call all methods");
            let mut s = s;
            for method in &method_sigs {
                // Only call no-arg methods automatically; skip methods that need args
                if method.args.is_empty() {
                    let sub = run_method(&setup_nav, &method.name, &no_args, expect_any).await;
                    eprintln!("  method {}: {:?}", method.name, sub.status);
                    s = s.sub_step(sub);
                } else {
                    let sub = StepBuilder::start(format!("SKIP {} (requires args: {:?})", method.name,
                        method.args.iter().map(|a| format!("{}: {}", a.name, a.arg_type)).collect::<Vec<_>>()))
                        .finish(AllureStatus::Skipped);
                    s = s.sub_step(sub);
                }
            }
            s.finish(AllureStatus::Passed)
        };
        builder = builder.step(methods_step);
    }

    builder.finish_from_steps()
}
