//! Test ALL methods and elements of `navex/desktopLayoutContainer`.
//!
//! Page object: `salesforce-pageobjects/navex/desktopLayoutContainer.utam.json`
//! Root selector: `.navexDesktopLayoutContainer`
//!
//! Methods tested (1):
//!   1. getAppNav — waitFor predicate that resolves the appNavPrivate custom
//!      component element.  Tests the waitFor + predicate + custom component
//!      resolution pipeline.
//!
//! Element resolution tested:
//!   - appNavPrivate (custom component: utam-global/pageObjects/appNav)
//!   - dockingPanel  (custom component: utam-force/pageObjects/dockingPanel)

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_test::allure::*;

pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder =
        TestResultBuilder::new("navex/desktopLayoutContainer — getAppNav + custom components")
            .full_name("salesforce_live::desktop_layout::test_all_methods")
            .description(
                "Load navex/desktopLayoutContainer and call getAppNav, which uses a \
                 waitFor predicate to resolve the appNavPrivate custom component element. \
                 Also resolves dockingPanel (another custom component) to test cross-page-object \
                 element resolution.",
            )
            .label("epic", "Salesforce Browser Testing")
            .label("feature", "Page Object Methods")
            .label("story", "navex/desktopLayoutContainer")
            .label("severity", "critical")
            .label("suite", "Salesforce Live")
            .parameter("driver", session.driver_name())
            .parameter("page_object", "navex/desktopLayoutContainer")
            .link(
                "Issue #82",
                "https://github.com/composable-delivery/busbar-sf-utam/issues/82",
                "issue",
            );

    eprintln!("\n=== Test: navex/desktopLayoutContainer ===");

    let nav = match session.load_page_object("navex/desktopLayoutContainer").await {
        Ok(po) => po,
        Err(e) => {
            eprintln!("  FAIL: could not load navex/desktopLayoutContainer: {e}");
            return builder.finish_err(AllureStatus::Broken, format!("load failed: {e}"), None);
        }
    };
    eprintln!("  Loaded navex/desktopLayoutContainer");

    let no_args = HashMap::new();

    // ── 1. getAppNav — waitFor predicate + custom component ────────────
    let step = run_method(&nav, "getAppNav", &no_args, expect_not_null).await;
    eprintln!("  [1/1] getAppNav: {:?}", step.status);
    builder = builder.step(step);

    // ── Element: resolve custom component elements ─────────────────────
    let elements_step = {
        let s = StepBuilder::start("resolve custom component elements");
        let sub1 = run_get_element(&nav, "appNavPrivate", &no_args).await;
        eprintln!("  element appNavPrivate: {:?}", sub1.status);
        let sub2 = run_get_element(&nav, "dockingPanel", &no_args).await;
        eprintln!("  element dockingPanel: {:?}", sub2.status);
        s.sub_step(sub1).sub_step(sub2).finish(AllureStatus::Passed)
    };
    builder = builder.step(elements_step);

    builder.finish_from_steps()
}
