//! Salesforce live integration tests — declarative coverage across all
//! page objects that match the live DOM.
//!
//! This test:
//!   1. Authenticates against a real Salesforce org (busbar-sf-api)
//!   2. Seeds test data (Account + Contact + Opportunity + Lead + Case)
//!   3. Navigates through Home → Account detail → Setup pages
//!   4. On each page, runs `find_known_page_objects` to discover every
//!      registered page object whose root selector matches the current DOM
//!   5. For each matched page object, the generic runner loads it and
//!      exercises EVERY method and EVERY public element, producing one
//!      Allure test result per page object with nested steps
//!
//! This turns the 1500 declarative UTAM page objects into real test cases
//! without per-page-object test code.  Known method/element arguments that
//! need specific values are curated in `sf_live::synth::override_args`.
//!
//! The run fails ONLY if discovery itself breaks — individual page object
//! method failures are reported in Allure as legitimate findings (stale
//! selectors, missing elements, version mismatches).

mod sf_live;

use utam_test::allure::AllureStatus;

#[tokio::test]
async fn test_salesforce_live() {
    let Some(session) = sf_live::session::SalesforceSession::setup().await else {
        return;
    };

    // All Allure results accumulate here.  Written to disk at end.
    let mut all_results = Vec::new();
    let mut summaries = Vec::new();
    let mut any_discovery_broken = false;

    // ── Phase 1: Home page (already loaded after auth) ─────────────────
    let coverage = sf_live::coverage::discover_and_test(&session, "home").await;
    if coverage.summary.status == AllureStatus::Broken {
        any_discovery_broken = true;
    }
    all_results.extend(coverage.results);
    summaries.push(coverage.summary);

    // ── Phase 2: Account detail ────────────────────────────────────────
    if let Some((_, account_id)) = session.seeded_records.iter().find(|(t, _)| t == "Account") {
        let url = format!("{}/lightning/r/Account/{account_id}/view", session.instance_url);
        session.navigate(&url).await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let coverage =
            sf_live::coverage::discover_and_test(&session, "account_detail").await;
        if coverage.summary.status == AllureStatus::Broken {
            any_discovery_broken = true;
        }
        all_results.extend(coverage.results);
        summaries.push(coverage.summary);
    }

    // ── Phase 3: Setup page ────────────────────────────────────────────
    let setup_url = format!("{}/lightning/setup/SetupOneHome/home", session.instance_url);
    session.navigate(&setup_url).await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let coverage = sf_live::coverage::discover_and_test(&session, "setup").await;
    if coverage.summary.status == AllureStatus::Broken {
        any_discovery_broken = true;
    }
    all_results.extend(coverage.results);
    summaries.push(coverage.summary);

    // ── Write all Allure results ───────────────────────────────────────
    eprintln!("\n=== Write Allure Results ===");
    for result in &all_results {
        if let Err(e) = session.allure.write_result(result) {
            eprintln!("  ERROR writing {}: {e}", result.name);
        }
    }
    for summary in &summaries {
        if let Err(e) = session.allure.write_result(summary) {
            eprintln!("  ERROR writing summary {}: {e}", summary.name);
        }
    }
    eprintln!(
        "  Wrote {} page object results + {} page summaries",
        all_results.len(),
        summaries.len()
    );

    // ── Aggregate stats ────────────────────────────────────────────────
    let total_pos = all_results.len();
    let passed = all_results.iter().filter(|r| r.status == AllureStatus::Passed).count();
    let failed = all_results.iter().filter(|r| r.status == AllureStatus::Failed).count();
    let broken = all_results.iter().filter(|r| r.status == AllureStatus::Broken).count();
    let skipped = all_results.iter().filter(|r| r.status == AllureStatus::Skipped).count();

    eprintln!("\n=== Final Summary ===");
    eprintln!("  Total page objects tested: {total_pos}");
    eprintln!("  Passed: {passed} ({}%)", percent(passed, total_pos));
    eprintln!("  Failed: {failed}");
    eprintln!("  Broken: {broken}");
    eprintln!("  Skipped: {skipped}");
    for s in &summaries {
        eprintln!("  Summary: {}", s.name);
    }

    // ── Cleanup ────────────────────────────────────────────────────────
    session.cleanup().await;

    // The test fails only if the discovery infrastructure itself broke.
    // Individual page object results (pass/fail/skip) are reported in
    // Allure — that's the whole point of coverage testing.
    assert!(
        !any_discovery_broken,
        "Discovery infrastructure failed on one or more pages — check the summary results"
    );

    // Also fail if we covered zero page objects — that means something is
    // catastrophically wrong with auth, navigation, or discovery.
    assert!(
        total_pos > 0,
        "Zero page objects matched on any page — auth or navigation failed"
    );

    eprintln!("\n=== Coverage complete: {passed}/{total_pos} page objects fully passed ===");
}

fn percent(n: usize, total: usize) -> String {
    if total == 0 {
        return "0".into();
    }
    format!("{:.1}", (n as f64) * 100.0 / (total as f64))
}
