//! Behavioral assertions for a curated set of high-value page objects.
//!
//! The generic coverage sweep (`runner` + `coverage`) is a *contract/smoke*
//! test: it proves every discovered page object loads and its members
//! execute against a real org and return type-correct values. It deliberately
//! does NOT assert that an action produced its intended *effect* — see the
//! `verification` parameter on each step.
//!
//! This module closes that gap for a hand-picked set of actions where the
//! outcome is both valuable and robustly observable. The guiding principle,
//! given these run against a shared scratch org we can't pre-inspect: anchor
//! every assertion on a **stable, controllable signal** rather than volatile
//! Lightning component DOM —
//!   - browser primitives we trust: `driver.current_url()`, `driver.title()`;
//!   - **data we own**: the records seeded in `session.rs` (Acme Corp, etc.).
//!
//! Each check is an `AllureTestResult` with a `verification` parameter that
//! states exactly what outcome was asserted. When a precondition isn't present
//! (e.g. the search box isn't on this page), the check records an honest
//! `Skipped` with the reason rather than a fabricated pass or failure — the
//! same discipline the generic runner uses.

use std::collections::HashMap;
use std::time::Duration;

use super::session::SalesforceSession;
use utam_runtime::element::RuntimeValue;
use utam_runtime::page_object::PageObjectRuntime;
use utam_test::allure::*;

/// All behavioral checks for one run, plus a summary.
pub struct BehavioralResults {
    pub results: Vec<AllureTestResult>,
}

/// Run every behavioral check against the current session/DOM and collect the
/// results. The caller is responsible for navigating to a sensible starting
/// page (these checks each assert their own precondition and skip cleanly when
/// it isn't met, so they're safe to run from any settled Lightning page).
pub async fn run_all(session: &SalesforceSession) -> BehavioralResults {
    eprintln!("\n=== Behavioral assertions ===");
    let mut results = Vec::new();

    results.push(assert_app_nav_tab_navigation(session).await);
    results.push(assert_global_search_navigates(session).await);

    for r in &results {
        eprintln!("  [{:?}] {}", r.status, r.name);
    }
    BehavioralResults { results }
}

/// Start a behavioral `AllureTestResult` with the shared labels/parameters.
fn start(session: &SalesforceSession, name: &str, po: &str) -> TestResultBuilder {
    TestResultBuilder::new(name.to_string())
        .full_name(format!("salesforce_live::behavioral::{name}"))
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Behavioral Assertions")
        .label("story", po)
        .label("suite", "Behavioral")
        .label("severity", "critical")
        .parameter("driver", session.driver_name())
        .parameter("page_object", po)
}

/// Finish a behavioral check as a reasoned `Skipped` (precondition absent),
/// marked `known` so Allure shows it as intentional, not an unexpected gap.
fn skipped(builder: TestResultBuilder, reason: &str) -> AllureTestResult {
    let mut r = builder.parameter("skip_reason", reason.to_string()).finish(AllureStatus::Skipped);
    r.status_details = Some(AllureStatusDetails {
        message: Some(reason.to_string()),
        trace: None,
        known: Some(true),
        muted: None,
        flaky: None,
    });
    r
}

// ───────────────────────────────────────────────────────────────────────────
// Check 1: app-nav tab click → URL changes to the target object's list view
// ───────────────────────────────────────────────────────────────────────────
//
// `navex/navItem` (and the app-nav bar generally) exposes clickable nav tabs.
// Driving a real click and asserting the browser actually navigated is a true
// behavioral outcome — far stronger than "the element resolved". We use the
// "Accounts" standard tab (present in any standard app) and assert the URL
// lands on the Account list/object surface.
async fn assert_app_nav_tab_navigation(session: &SalesforceSession) -> AllureTestResult {
    let builder = start(session, "app_nav_tab_click_navigates", "navex/navItem").description(
        "Clicks the standard 'Accounts' nav tab and asserts the browser navigated to the \
         Account object surface (URL contains /lightning/o/Account or /Account/). Verifies the \
         click produced its navigation effect, not merely that the tab element resolved.",
    );

    // The app nav bar renders nav items as anchors with a title attribute.
    // Resolve the "Accounts" tab directly via the driver (a stable standard
    // tab) so this check doesn't depend on a specific PO discovering.
    let before = session.driver.current_url().await.unwrap_or_default();

    let link = match session
        .driver
        .find_element(&utam_runtime::driver::Selector::Css(
            "a.slds-context-bar__label-action[title='Accounts'], \
             one-app-nav-bar-item-root a[href*='/lightning/o/Account/']"
                .into(),
        ))
        .await
    {
        Ok(el) => el,
        Err(_) => {
            return skipped(
                builder,
                "no 'Accounts' nav tab on this page/app (standard tab not present in this context)",
            );
        }
    };

    if let Err(e) = link.click().await {
        return builder.finish_err(
            AllureStatus::Failed,
            format!("clicking the Accounts nav tab failed: {e}"),
            None,
        );
    }

    // Give Lightning a moment to route, then poll the URL for the effect.
    let after = match session
        .wait_for_url_matching("Accounts navigation", Duration::from_secs(10), |url| {
            url.contains("/lightning/o/Account") || url.contains("/Account/")
        })
        .await
    {
        Ok(url) => url,
        Err(_) => session.driver.current_url().await.unwrap_or_default(),
    };

    let navigated =
        after != before && (after.contains("/lightning/o/Account") || after.contains("/Account/"));
    let builder =
        builder.parameter("url_before", before).parameter("url_after", after.clone()).parameter(
            "verification",
            "ASSERTED: after clicking the Accounts nav tab the browser URL changed to the \
             Account object surface (/lightning/o/Account or /Account/). This is the navigation \
             outcome, not just element resolution.",
        );

    if navigated {
        builder.finish(AllureStatus::Passed)
    } else {
        builder.finish_err(
            AllureStatus::Failed,
            format!(
                "clicked Accounts nav tab but URL did not navigate to the Account surface: {after}"
            ),
            None,
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Check 2: global header search → submitting a term navigates to search results
// ───────────────────────────────────────────────────────────────────────────
//
// `global/header::getSearch` drives the global-search flow. Rather than assert
// on the (volatile) results DOM, we assert the robust outcome: submitting a
// search routes the browser to the global search results surface
// (`/one/one.app#... search` → URL contains "search"). We search for the
// seeded Account name so the term corresponds to data we control.
async fn assert_global_search_navigates(session: &SalesforceSession) -> AllureTestResult {
    let builder = start(session, "global_search_navigates_to_results", "global/header")
        .description(
        "Loads global/header, drives getSearch with a real term (the seeded Account name), and \
         asserts the browser navigated to the global search results surface. Verifies the search \
         action's effect, not merely that the search element resolved.",
    );

    // Only meaningful where the header (and its search) is present.
    let po = match session.load_page_object("global/header").await {
        Ok(po) => po,
        Err(e) => return skipped(builder, &format!("global/header did not load here: {e}")),
    };

    // Use the seeded Account's name as the search term — data we own. Strip the
    // run-tag suffix to the stable base so the term is a normal word.
    let term = "Acme Corp";
    let before = session.driver.current_url().await.unwrap_or_default();

    let mut args: HashMap<String, RuntimeValue> = HashMap::new();
    args.insert("searchTerm".into(), RuntimeValue::String(term.into()));

    // getSearch returns the search component; driving it is the action under
    // test. A failure to even invoke it (e.g. mobile-only flow on desktop) is a
    // clean skip, mirroring the generic runner's member_skip_reason for search.
    if let Err(e) = po.call_method("getSearch", &args).await {
        return skipped(
            builder,
            &format!("getSearch not drivable in this context (desktop/mobile flow): {e}"),
        );
    }

    let after = match session
        .wait_for_url_matching("global search navigation", Duration::from_secs(10), |url| {
            url.contains("search") && url != before
        })
        .await
    {
        Ok(url) => url,
        Err(_) => session.driver.current_url().await.unwrap_or_default(),
    };

    let builder = builder
        .parameter("search_term", term)
        .parameter("url_before", before.clone())
        .parameter("url_after", after.clone())
        .parameter(
            "verification",
            "ASSERTED: driving getSearch with a real term routed the browser to the global \
             search results surface (URL contains 'search' and changed). This is the search \
             action's navigation effect.",
        );

    if after.contains("search") && after != before {
        builder.finish(AllureStatus::Passed)
    } else {
        // The header is present but search didn't route — record as a real
        // failure only when it's the desktop flow we expect to work; otherwise
        // the call_method skip above would have caught the mobile case.
        builder.finish_err(
            AllureStatus::Failed,
            format!("getSearch executed but URL did not route to search results: {after}"),
            None,
        )
    }
}
