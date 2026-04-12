//! Test page objects on the Salesforce Setup page.
//!
//! Navigates to Setup Home, then loads page objects that should be
//! present and exercises their methods/elements.
//!
//! Page objects tested:
//!   - setup/setupNavTree — the left-hand navigation tree
//!     - getAndWaitForNavTreeNodeByName(ariaLabel) — waitFor predicate + parameterized element
//!     - waitForUrl(url) — waitFor predicate + document URL matcher
//!     - navTreeNodeByName(ariaLabel) — parameterized element resolution

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::PageObjectRuntime;
use utam_test::allure::*;

pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder = TestResultBuilder::new("setup/setupNavTree — methods + parameterized elements")
        .full_name("salesforce_live::setup_page::test_all_methods")
        .description(
            "Navigate to Setup Home, load setup/setupNavTree, then exercise both \
             methods with real arguments: getAndWaitForNavTreeNodeByName searches for \
             a nav node by aria-label, waitForUrl waits for the URL to contain a \
             substring.  Also resolves the parameterized navTreeNodeByName element.",
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
            s.parameter("url", url).finish_err("URL does not contain 'setup'")
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

    // ── Introspect ─────────────────────────────────────────────────────
    let introspect_step = {
        let methods = setup_nav.method_signatures();
        let elements = setup_nav.element_names();
        let desc = setup_nav.description().unwrap_or_else(|| "<none>".into());
        eprintln!("  Description: {desc}");
        eprintln!(
            "  Methods ({}): {:?}",
            methods.len(),
            methods.iter().map(|m| &m.name).collect::<Vec<_>>()
        );
        eprintln!("  Elements ({}): {:?}", elements.len(), elements);

        StepBuilder::start("introspect methods and elements")
            .parameter("method_count", methods.len().to_string())
            .parameter("element_count", elements.len().to_string())
            .parameter("description", desc)
            .finish(AllureStatus::Passed)
    };
    builder = builder.step(introspect_step);

    // ── Element: navTreeNodeByName with ariaLabel="Users" ──────────────
    // Selector: .onesetupNavTreeNode[aria-label*='%s']
    // The "Users" node should be present on Setup Home.
    let element_step = {
        let s = StepBuilder::start("resolve navTreeNodeByName(ariaLabel=\"Users\")");
        let mut args = HashMap::new();
        args.insert("ariaLabel".into(), RuntimeValue::String("Users".into()));
        match setup_nav.get_element("navTreeNodeByName", &args).await {
            Ok(el) => {
                eprintln!("  element navTreeNodeByName(\"Users\"): PASS ({})", el.type_name());
                s.parameter("capability", el.type_name()).finish(AllureStatus::Passed)
            }
            Err(e) => {
                eprintln!("  element navTreeNodeByName(\"Users\"): FAIL ({e})");
                s.finish_err(format!("{e}"))
            }
        }
    };
    builder = builder.step(element_step);

    // ── Method: getAndWaitForNavTreeNodeByName ──────────────────────────
    // This method references the navTreeNodeByName element, which requires
    // ariaLabel.  The arg flows through the compose predicate → element
    // selector substitution.
    let method1_step = {
        let s = StepBuilder::start("call_method(\"getAndWaitForNavTreeNodeByName\") with ariaLabel=\"Users\"");
        let mut args = HashMap::new();
        args.insert("ariaLabel".into(), RuntimeValue::String("Users".into()));
        match setup_nav.call_method("getAndWaitForNavTreeNodeByName", &args).await {
            Ok(value) => {
                eprintln!("  getAndWaitForNavTreeNodeByName(\"Users\"): PASS = {value}");
                s.parameter("returned", format!("{value}")).finish(AllureStatus::Passed)
            }
            Err(e) => {
                eprintln!("  getAndWaitForNavTreeNodeByName(\"Users\"): FAIL = {e}");
                s.finish_err(format!("{e}"))
            }
        }
    };
    builder = builder.step(method1_step);

    // ── Method: waitForUrl ─────────────────────────────────────────────
    // The compose predicate checks document.getUrl contains the "url" arg.
    // We're already on the Setup page, so "Setup" should match.
    let method2_step = {
        let s = StepBuilder::start("call_method(\"waitForUrl\") with url=\"Setup\"");
        let mut args = HashMap::new();
        args.insert("url".into(), RuntimeValue::String("Setup".into()));
        match setup_nav.call_method("waitForUrl", &args).await {
            Ok(value) => {
                eprintln!("  waitForUrl(\"Setup\"): PASS = {value}");
                s.parameter("returned", format!("{value}")).finish(AllureStatus::Passed)
            }
            Err(e) => {
                eprintln!("  waitForUrl(\"Setup\"): FAIL = {e}");
                s.finish_err(format!("{e}"))
            }
        }
    };
    builder = builder.step(method2_step);

    builder.finish_from_steps()
}
