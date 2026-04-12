//! Salesforce live integration tests — declarative coverage across every
//! page object that matches the live DOM on three key pages.
//!
//! Each page context is its own `#[test]` so `cargo test` shows individual
//! pass/fail for home / account_detail / setup.  They share one browser
//! session and one set of seeded records via `sf_live::shared`, which uses
//! a single `LazyLock<Runtime>` + a mutex to serialize browser access.
//!
//! Tests run in alphabetical order; the final `zz_teardown` test runs
//! last and drops seeded records + quits the browser.  Run a single
//! phase:
//!
//!     cargo test -p utam-runtime --test salesforce_live -- a_home --test-threads=1
//!     cargo test -p utam-runtime --test salesforce_live -- b_account_detail --test-threads=1
//!     cargo test -p utam-runtime --test salesforce_live -- c_setup --test-threads=1
//!
//! `SF_AUTH_URL` is REQUIRED.  No silent-skip path: tests that "pass"
//! without a real Salesforce org give false confidence.

mod sf_live;

use utam_test::allure::AllureStatus;

use sf_live::{coverage, shared};

// ───────────────────────────────────────────────────────────────────────────
// Test 1: Home page — runs first, leaves browser on Lightning home
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn a_home_coverage() {
    shared::with_session(|session| async move {
        // session is already navigated to /lightning/page/home during setup
        let result = coverage::discover_and_test(session, "home").await;
        write_and_assert(result, "home");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 2: Account detail — navigates to the seeded Acme Corp record
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn b_account_detail_coverage() {
    shared::with_session(|session| async move {
        let account_id = session
            .seeded_records
            .iter()
            .find(|(t, _)| t == "Account")
            .map(|(_, id)| id.clone())
            .expect(
                "seeded Account is required — data seeding failed during setup, \
                 which means the Salesforce org rejected record creation",
            );
        let url = format!("{}/lightning/r/Account/{account_id}/view", session.instance_url);
        session.navigate(&url).await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let result = coverage::discover_and_test(session, "account_detail").await;
        write_and_assert(result, "account_detail");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 3: Setup page — navigates to Setup Home
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn c_setup_coverage() {
    shared::with_session(|session| async move {
        let url = format!("{}/lightning/setup/SetupOneHome/home", session.instance_url);
        session.navigate(&url).await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let result = coverage::discover_and_test(session, "setup").await;
        write_and_assert(result, "setup");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Teardown — alphabetically last, drops seeded records + quits browser
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn zz_teardown() {
    shared::teardown();
}

// ───────────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────────

/// Write all Allure results from a coverage run and assert the suite
/// didn't fail catastrophically (discovery broken or zero POs matched).
fn write_and_assert(coverage: coverage::CoverageResults, context: &str) {
    let writer = shared::with_allure(|w| w.results_dir().to_path_buf());
    let dir_desc = writer
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<no writer>".into());

    let total = coverage.results.len();
    let passed =
        coverage.results.iter().filter(|r| r.status == AllureStatus::Passed).count();
    let failed =
        coverage.results.iter().filter(|r| r.status == AllureStatus::Failed).count();
    let broken =
        coverage.results.iter().filter(|r| r.status == AllureStatus::Broken).count();

    // Write each per-PO result and the summary.
    shared::with_allure(|writer| {
        for result in &coverage.results {
            if let Err(e) = writer.write_result(result) {
                eprintln!("  ERROR writing {}: {e}", result.name);
            }
        }
        if let Err(e) = writer.write_result(&coverage.summary) {
            eprintln!("  ERROR writing summary: {e}");
        }
    });

    eprintln!(
        "\n=== {context} summary: {passed}/{total} passed, {failed} failed, {broken} broken \
         (results in {dir_desc}) ===\n"
    );

    // Fail the cargo test if discovery itself broke (coverage summary is
    // Broken), or if zero page objects matched (auth/navigation failure).
    if coverage.summary.status == AllureStatus::Broken {
        panic!("discovery infrastructure failed for {context}");
    }
    if total == 0 {
        panic!("zero page objects matched on {context} — auth or navigation failed");
    }
}
