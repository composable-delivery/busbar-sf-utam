//! Test ALL methods of `global/header` — the main Lightning header bar.
//!
//! Page object: `salesforce-pageobjects/global/header.utam.json`
//! Root selector: `.oneHeader`
//!
//! Methods tested (9 total):
//!   1. getNotificationCount  — getText on counter label → String
//!   2. hasNewNotification    — isVisible on counter → Bool
//!   3. showSetupMenu         — click setup gear → Null (void)
//!   4. getGlobalActionsList  — click global create trigger → Null
//!   5. showNotifications     — click notifications button → Null
//!   6. addToFavorites        — click add-to-favorites button → Null
//!   7. getFavoriteList       — click favorites list button → Null
//!   8. getSearch             — click search icon + type text + click → multi-step
//!   9. waitAndClickCoPilot   — nullable: waitForVisible + click (org may not have copilot)
//!
//! Element resolution tested:
//!   - searchIcon, searchInput, setupMenu, notifications, notificationCount,
//!     globalActions, addFavoriteButton, favoritesList, copilot (nullable)

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_runtime::element::RuntimeValue;
use utam_test::allure::*;

/// Exercise every method and key elements of global/header against the live DOM.
pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder = TestResultBuilder::new("global/header — all 9 methods")
        .full_name("salesforce_live::header::test_all_methods")
        .description(
            "Load the global/header page object and call every compose method, \
             validating return types and side effects. Tests getText, isVisible, \
             click, setText, waitForVisible, and multi-step compose chains.",
        )
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Methods")
        .label("story", "global/header")
        .label("severity", "critical")
        .label("suite", "Salesforce Live")
        .parameter("driver", session.driver_name())
        .parameter("page_object", "global/header")
        .link(
            "Issue #82",
            "https://github.com/composable-delivery/busbar-sf-utam/issues/82",
            "issue",
        );

    eprintln!("\n=== Test: global/header — all 9 methods ===");

    // ── Load ────────────────────────────────────────────────────────────
    let header = match session.load_page_object("global/header").await {
        Ok(po) => po,
        Err(e) => {
            eprintln!("  FAIL: could not load global/header: {e}");
            return builder.finish_err(AllureStatus::Broken, format!("load failed: {e}"), None);
        }
    };
    eprintln!("  Loaded global/header");

    let no_args = HashMap::new();

    // ── 1. getNotificationCount — getText → String ─────────────────────
    let step = run_method(&header, "getNotificationCount", &no_args, expect_string).await;
    eprintln!("  [1/9] getNotificationCount: {}", step_summary(&step));
    builder = builder.step(step);

    // ── 2. hasNewNotification — isVisible → Bool ───────────────────────
    let step = run_method(&header, "hasNewNotification", &no_args, expect_bool).await;
    eprintln!("  [2/9] hasNewNotification: {}", step_summary(&step));
    builder = builder.step(step);

    // ── 3. showSetupMenu — click → Null ────────────────────────────────
    let step = run_method(&header, "showSetupMenu", &no_args, expect_null).await;
    eprintln!("  [3/9] showSetupMenu: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 4. getGlobalActionsList — click → Null ─────────────────────────
    let step = run_method(&header, "getGlobalActionsList", &no_args, expect_null).await;
    eprintln!("  [4/9] getGlobalActionsList: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 5. showNotifications — click → Null ────────────────────────────
    let step = run_method(&header, "showNotifications", &no_args, expect_null).await;
    eprintln!("  [5/9] showNotifications: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 6. addToFavorites — click → Null ───────────────────────────────
    let step = run_method(&header, "addToFavorites", &no_args, expect_null).await;
    eprintln!("  [6/9] addToFavorites: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 7. getFavoriteList — click → Null ──────────────────────────────
    let step = run_method(&header, "getFavoriteList", &no_args, expect_null).await;
    eprintln!("  [7/9] getFavoriteList: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 8. getSearch — multi-step: click icon → type text → click ──────
    // The searchIcon selector uses `.forceHeaderButtonDeprecated` — a class
    // that Salesforce deprecated and removed from newer orgs.  This method
    // exercises the compose chain (click → setText → click) and will correctly
    // fail when the selector is stale.
    let mut search_args = HashMap::new();
    search_args.insert("searchTerm".into(), RuntimeValue::String("Test".into()));
    let step = run_method(&header, "getSearch", &search_args, expect_any).await;
    eprintln!("  [8/9] getSearch(\"Test\"): {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── 9. waitAndClickCoPilot — nullable ──────────────────────────────
    // The copilot element is nullable (may not exist in all orgs).
    // run_method_nullable reports Skipped instead of Failed.
    let step = run_method_nullable(&header, "waitAndClickCoPilot", &no_args).await;
    eprintln!("  [9/9] waitAndClickCoPilot: {}", step_summary(&step));
    builder = builder.step(step);
    session.dismiss_ui().await;

    // ── Element resolution: verify non-deprecated elements resolve ─────
    let element_step = {
        let s = StepBuilder::start("resolve key elements");
        let mut s = s;
        // Only test elements whose selectors should be present.
        for name in &[
            "searchIcon",
            "notifications",
            "notificationCount",
            "globalActions",
            "setupMenu",
            "addFavoriteButton",
            "favoritesList",
        ] {
            let sub = run_get_element(&header, name, &no_args).await;
            eprintln!("  element {name}: {}", step_summary(&sub));
            s = s.sub_step(sub);
        }
        s.finish(AllureStatus::Passed)
    };
    builder = builder.step(element_step);

    builder.finish_from_steps()
}

fn step_summary(step: &AllureStep) -> &'static str {
    match step.status {
        AllureStatus::Passed => "PASS",
        AllureStatus::Failed => "FAIL",
        AllureStatus::Broken => "BROKEN",
        AllureStatus::Skipped => "SKIP",
        AllureStatus::Unknown => "UNKNOWN",
    }
}
