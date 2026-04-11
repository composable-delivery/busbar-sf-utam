//! Test ALL methods and elements of `global/globalCreate`.
//!
//! Page object: `salesforce-pageobjects/global/globalCreate.utam.json`
//! Root selector: `div[class*='globalCreateContainer']`
//! Has `beforeLoad` with waitFor predicate.
//!
//! Methods tested (1):
//!   1. clickGlobalActions — click the "New" button to open the create menu
//!
//! Element resolution tested:
//!   - globalCreateButton — the <a> link that opens the menu
//!   - globalCreateMenuItem(titleString) — parameterized selector to find a
//!     specific menu item by title, e.g. "New Contact"
//!
//! After opening the menu, we resolve `globalCreateMenuItem` with a title
//! argument and click it through the element's action interface.  This
//! validates parameterized selector substitution (%s) against live DOM.

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_runtime::element::{ElementRuntime, RuntimeValue};
use utam_runtime::page_object::PageObjectRuntime;
use utam_test::allure::*;

pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder = TestResultBuilder::new("global/globalCreate — method + parameterized element")
        .full_name("salesforce_live::global_create::test_all_methods")
        .description(
            "Load global/globalCreate (which has a beforeLoad waitFor predicate), \
             call clickGlobalActions to open the menu, then resolve the \
             globalCreateMenuItem element using a parameterized selector (%s title) \
             and click it via the element action interface.",
        )
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Methods")
        .label("story", "global/globalCreate")
        .label("severity", "critical")
        .label("suite", "Salesforce Live")
        .parameter("driver", session.driver_name())
        .parameter("page_object", "global/globalCreate")
        .link(
            "Issue #82",
            "https://github.com/composable-delivery/busbar-sf-utam/issues/82",
            "issue",
        );

    eprintln!("\n=== Test: global/globalCreate ===");

    let global_create = match session.load_page_object("global/globalCreate").await {
        Ok(po) => po,
        Err(e) => {
            eprintln!("  FAIL: could not load global/globalCreate: {e}");
            return builder.finish_err(AllureStatus::Broken, format!("load failed: {e}"), None);
        }
    };
    eprintln!("  Loaded global/globalCreate (beforeLoad predicate passed)");

    let no_args = HashMap::new();

    // ── 1. clickGlobalActions — opens the "New" menu ───────────────────
    let step = run_method(&global_create, "clickGlobalActions", &no_args, expect_null).await;
    eprintln!("  [1/1] clickGlobalActions: {:?}", step.status);
    builder = builder.step(step);

    // Wait for the menu to render
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // ── Element: globalCreateMenuItem with parameterized selector ───────
    // The selector is: [class*=oneGlobalCreateItem] a[title='%s']
    // We substitute %s with "New Contact" to find the menu item.
    let item_step = {
        let s = StepBuilder::start("resolve globalCreateMenuItem(\"New Contact\")");
        let mut title_args = HashMap::new();
        title_args.insert("titleString".into(), RuntimeValue::String("New Contact".into()));
        match global_create.get_element("globalCreateMenuItem", &title_args).await {
            Ok(el) => {
                eprintln!("  Resolved globalCreateMenuItem(\"New Contact\") — clicking via element");
                // Click it through the element's action interface
                let click_sub = StepBuilder::start("execute(\"click\")");
                let no_action_args: &[RuntimeValue] = &[];
                match el.execute("click", no_action_args).await {
                    Ok(_) => {
                        eprintln!("  Clicked 'New Contact' via page object element");
                        s.sub_step(click_sub.finish(AllureStatus::Passed))
                            .parameter("capability", el.type_name())
                            .finish(AllureStatus::Passed)
                    }
                    Err(e) => {
                        s.sub_step(click_sub.finish_err(format!("{e}")))
                            .finish(AllureStatus::Failed)
                    }
                }
            }
            Err(e) => {
                eprintln!("  Could not resolve globalCreateMenuItem: {e}");
                s.finish_err(format!("get_element failed: {e}"))
            }
        }
    };
    builder = builder.step(item_step);

    // Wait for modal/overlay
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // ── Optional: load recordActionWrapper if a modal appeared ──────────
    let modal_step = {
        let s = StepBuilder::start("load global/recordActionWrapper (if modal appeared)");
        match session.load_page_object("global/recordActionWrapper").await {
            Ok(record_modal) => {
                eprintln!("  recordActionWrapper loaded — testing clickFooterButton");
                let mut save_args = HashMap::new();
                save_args.insert(
                    "labelText".into(),
                    RuntimeValue::String("Save".into()),
                );
                let sub = run_method(&record_modal, "clickFooterButton", &save_args, expect_any).await;
                eprintln!("  clickFooterButton('Save'): {:?}", sub.status);
                s.sub_step(sub).finish(AllureStatus::Passed)
            }
            Err(e) => {
                eprintln!("  recordActionWrapper not present (expected if quick action not configured): {e}");
                let mut finished = s.finish(AllureStatus::Skipped);
                finished.status_details = Some(AllureStatusDetails {
                    message: Some(format!("Modal not configured: {e}")),
                    trace: None,
                    known: Some(true),
                    muted: None,
                    flaky: None,
                });
                finished
            }
        }
    };
    builder = builder.step(modal_step);

    session.dismiss_ui().await;

    builder.finish_from_steps()
}
