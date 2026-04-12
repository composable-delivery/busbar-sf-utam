//! Salesforce live integration tests
//!
//! Runs against a real Salesforce org.  Authenticates via busbar-sf-api,
//! seeds test data, then exercises page object **methods** against the
//! live DOM — not just selectors, but every compose action chain.
//!
//! Each page object module produces its own Allure test result with:
//! - Nested steps (one per method call, with timing)
//! - Parameters (driver type, page object name)
//! - Labels (epic / feature / story / severity)
//! - Links to GitHub issues
//!
//! Test modules:
//! - `header`          — global/header: all 9 methods
//! - `desktop_layout`  — navex/desktopLayoutContainer: getAppNav + custom components
//! - `global_create`   — global/globalCreate: clickGlobalActions + parameterized menu item
//! - `account_detail`  — Account record detail: seeded data + page objects
//! - `setup_page`      — setup/setupNavTree: load + introspect + resolve elements
//!
//! Skipped locally (no CHROMEDRIVER_URL), panics in CI if credentials missing.

mod sf_live;

#[tokio::test]
async fn test_salesforce_live() {
    let Some(session) = sf_live::session::SalesforceSession::setup().await else {
        return; // skip — no credentials
    };

    let mut all_results = Vec::new();

    // ── Home page tests ────────────────────────────────────────────────
    // These run on the Lightning home page (already loaded after auth).

    all_results.push(sf_live::header::test_all_methods(&session).await);
    all_results.push(sf_live::desktop_layout::test_all_methods(&session).await);
    all_results.push(sf_live::global_create::test_all_methods(&session).await);

    // ── Record detail page ─────────────────────────────────────────────
    // Navigates to the seeded Account record.

    if !session.seeded_records.is_empty() {
        all_results.push(sf_live::account_detail::test_all_methods(&session).await);
    }

    // ── Setup page ─────────────────────────────────────────────────────
    // Navigates to Setup Home.

    all_results.push(sf_live::setup_page::test_all_methods(&session).await);

    // ── Write Allure results ───────────────────────────────────────────
    eprintln!("\n=== Write Allure Results ===");
    for result in &all_results {
        match session.allure.write_result(result) {
            Ok(path) => eprintln!("  Wrote: {} ({})", path.display(), result.name),
            Err(e) => eprintln!("  ERROR writing {}: {e}", result.name),
        }
    }

    let passed = all_results.iter().filter(|r| r.status == utam_test::allure::AllureStatus::Passed).count();
    let failed = all_results.iter().filter(|r| r.status == utam_test::allure::AllureStatus::Failed).count();
    let broken = all_results.iter().filter(|r| r.status == utam_test::allure::AllureStatus::Broken).count();
    let total = all_results.len();
    eprintln!("\n=== Summary: {passed} passed, {failed} failed, {broken} broken out of {total} ===");

    // ── Cleanup ────────────────────────────────────────────────────────
    session.cleanup().await;

    assert!(
        failed == 0 && broken == 0,
        "{failed} failed, {broken} broken out of {total} — see Allure report for details"
    );

    eprintln!("\n=== All {total} page object test suites passed ===");
}
