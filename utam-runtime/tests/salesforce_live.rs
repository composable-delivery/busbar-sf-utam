//! Salesforce live integration tests — declarative coverage across every
//! page object that matches the live DOM on three key pages.
//!
//! Each page context is its own `#[test]` so `cargo test` shows individual
//! pass/fail for home / account_detail / setup.  They share one browser
//! session and one set of seeded records via [`sf_live::shared`], which uses
//! a single `LazyLock<Runtime>` + a mutex to serialize browser access.
//!
//! Tests run in alphabetical order; the final `zz_teardown` test runs last
//! and drops seeded records + quits the browser.  Run a single phase with
//! `cargo test --test salesforce_live -- a_home_coverage --ignored`.
//!
//! Every test is `#[ignore]`d so a plain `cargo test` (no real org) skips
//! them.  CI runs them with `--ignored`; setup panics loudly if
//! `SF_AUTH_URL` is missing or invalid — there is no silent pass.

mod sf_live;

use sf_live::{behavioral, coverage, shared};
use utam_test::allure::AllureStatus;

// ───────────────────────────────────────────────────────────────────────────
// Test 1: Home — the default Lightning landing page
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
fn a_home_coverage() {
    shared::with_session(|session| async move {
        let url = format!("{}/lightning/page/home", session.instance_url);
        session.navigate(&url).await;

        let result = coverage::discover_and_test(session, "home").await;
        write_and_assert(result, "home");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 2: Account detail — navigates to the seeded Acme Corp record
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
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

        let result = coverage::discover_and_test(session, "account_detail").await;
        write_and_assert(result, "account_detail");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 3: Setup page — navigates to Setup Home
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
fn c_setup_coverage() {
    shared::with_session(|session| async move {
        let url = format!("{}/lightning/setup/SetupOneHome/home", session.instance_url);
        session.navigate(&url).await;

        let result = coverage::discover_and_test(session, "setup").await;
        write_and_assert(result, "setup");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 4: Console app — opens the seeded `UTAM_Console` Lightning console app,
// whose utility bar + console nav render the console-only / utility-bar page
// objects (utilityBarContainer, navex console tabs) that a standard desktop
// app never shows.  The app and its utility bar are deployed from force-app by
// the `deploy-baseline-metadata` workflow; if the deploy was skipped (e.g. a
// hand-run against an un-seeded org) the app simply won't load and discovery
// records honest "surface absent" results rather than fabricating coverage.
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
fn d_console_coverage() {
    shared::with_session(|session| async move {
        // Custom Lightning apps are reachable by developer name at
        // /lightning/app/<DeveloperName>.  Landing on the app's default tab
        // renders its utility bar and console chrome.
        let url = format!("{}/lightning/app/UTAM_Console", session.instance_url);
        session.navigate(&url).await;

        // The console workspace manager renders after the global shell (header)
        // is visible.  `navigate` returns as soon as the header appears, so
        // wait explicitly for the console chrome before running discovery.
        if let Err(e) = session
            .wait_for_element(
                utam_runtime::driver::Selector::Css(".navexWorkspaceManager".to_string()),
                "console workspace manager",
                std::time::Duration::from_secs(20),
            )
            .await
        {
            eprintln!("WARNING: console workspace manager did not appear: {e}");
        }

        let result = coverage::discover_and_test(session, "console").await;
        write_and_assert(result, "console");
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Test 5: Behavioral assertions — drives a curated set of real actions and
// asserts their OUTCOME (navigation/state change), not just that the member
// resolved.  Runs on the home page, where the app nav bar and global header
// are both present.  Each check skips cleanly when its precondition isn't met,
// so this test fails only on a genuine behavioral regression.
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
fn e_behavioral_assertions() {
    shared::with_session(|session| async move {
        let url = format!("{}/lightning/page/home", session.instance_url);
        session.navigate(&url).await;

        let results = behavioral::run_all(session).await;
        write_behavioral(results);
    });
}

// ───────────────────────────────────────────────────────────────────────────
// Teardown — alphabetically last, drops seeded records + quits browser
// ───────────────────────────────────────────────────────────────────────────
#[test]
#[ignore = "requires real Salesforce org credentials (SF_AUTH_URL)"]
fn zz_teardown() {
    shared::teardown();
}

// ───────────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────────

/// Write all Allure results from a coverage run and assert the suite
/// passed — ANY page object that is Failed or Broken fails the test.
///
/// The goal of these tests is to find real issues.  Silent passes or
/// majority-based pass rates let real failures rot.  If a page object
/// is declared in the registry, matches the DOM, and our runtime can't
/// load + exercise it cleanly, that's a bug we must surface.
fn write_and_assert(coverage: coverage::CoverageResults, context: &str) {
    let total = coverage.results.len();
    let passed = coverage.results.iter().filter(|r| r.status == AllureStatus::Passed).count();
    let failed = coverage.results.iter().filter(|r| r.status == AllureStatus::Failed).count();
    let broken = coverage.results.iter().filter(|r| r.status == AllureStatus::Broken).count();

    // Write each per-PO result and the summary to Allure.
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

    let dir_desc = shared::with_allure(|w| w.results_dir().to_path_buf())
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<no writer>".into());
    eprintln!(
        "\n=== {context} summary: {passed}/{total} passed, {failed} failed, {broken} broken \
         (results in {dir_desc}) ==="
    );

    // Hard-fail conditions, in order of severity.
    if coverage.summary.status == AllureStatus::Broken {
        panic!("discovery infrastructure failed for {context}");
    }
    if total == 0 {
        panic!(
            "zero page objects matched on {context} — auth or navigation failed, \
             or the registry is empty"
        );
    }

    // Any Failed or Broken PO fails the test.  Build a detailed message
    // that names each failing page object so the cargo output is enough
    // to diagnose without opening Allure.
    if failed > 0 || broken > 0 {
        let mut details = String::new();
        for r in &coverage.results {
            if r.status == AllureStatus::Failed || r.status == AllureStatus::Broken {
                details.push_str(&format!("\n  [{:?}] {}", r.status, r.name));
                if let Some(sd) = &r.status_details {
                    if let Some(msg) = &sd.message {
                        details.push_str(&format!("\n    {}", msg.lines().next().unwrap_or("")));
                    }
                }
            }
        }
        panic!("{context}: {failed} failed, {broken} broken out of {total} page objects:{details}");
    }

    eprintln!("=== {context}: all {total} page objects passed ===\n");
}

/// Write behavioral results to Allure and assert none Failed/Broken.
///
/// A `Skipped` check (precondition absent — e.g. the search box isn't on this
/// page) is acceptable: behavioral checks self-gate and skip honestly rather
/// than fabricate a pass.  Only a real behavioral regression (a driven action
/// whose outcome didn't happen) fails the test, naming each failing check.
fn write_behavioral(results: behavioral::BehavioralResults) {
    let total = results.results.len();
    let passed = results.results.iter().filter(|r| r.status == AllureStatus::Passed).count();
    let skipped = results.results.iter().filter(|r| r.status == AllureStatus::Skipped).count();
    let failed = results.results.iter().filter(|r| r.status == AllureStatus::Failed).count();
    let broken = results.results.iter().filter(|r| r.status == AllureStatus::Broken).count();

    shared::with_allure(|writer| {
        for result in &results.results {
            if let Err(e) = writer.write_result(result) {
                eprintln!("  ERROR writing {}: {e}", result.name);
            }
        }
    });

    eprintln!(
        "\n=== behavioral summary: {passed} passed, {skipped} skipped, {failed} failed, \
         {broken} broken (of {total}) ==="
    );

    if failed > 0 || broken > 0 {
        let mut details = String::new();
        for r in &results.results {
            if r.status == AllureStatus::Failed || r.status == AllureStatus::Broken {
                details.push_str(&format!("\n  [{:?}] {}", r.status, r.name));
                if let Some(sd) = &r.status_details {
                    if let Some(msg) = &sd.message {
                        details.push_str(&format!("\n    {}", msg.lines().next().unwrap_or("")));
                    }
                }
            }
        }
        panic!("behavioral: {failed} failed, {broken} broken of {total} checks:{details}");
    }
}
