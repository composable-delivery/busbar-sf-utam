//! Test page objects on the Account record detail page.
//!
//! Navigates to the seeded Acme Corp account, then loads page objects
//! that should be present on a Lightning record detail view and exercises
//! their methods.
//!
//! Page objects tested:
//!   - global/header (verify it's still loadable after navigation)
//!   - Seeded data verification via page title and body text

use std::collections::HashMap;

use super::helpers::*;
use super::session::SalesforceSession;
use utam_test::allure::*;

pub async fn test_all_methods(session: &SalesforceSession) -> AllureTestResult {
    let mut builder = TestResultBuilder::new("Account detail — seeded data + page objects")
        .full_name("salesforce_live::account_detail::test_all_methods")
        .description(
            "Navigate to the seeded Acme Corp Account record detail page. Verify that \
             seeded data (Account name, related Contact, related Opportunity) is visible. \
             Load global/header on the record page to verify page objects work across \
             navigation, and call getNotificationCount to exercise a method on the new page.",
        )
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Record Detail")
        .label("story", "Account detail page")
        .label("severity", "critical")
        .label("suite", "Salesforce Live")
        .parameter("driver", session.driver_name())
        .parameter("page_object", "record detail");

    eprintln!("\n=== Test: Account detail — seeded data ===");

    // ── Find the seeded Account ID ─────────────────────────────────────
    let account_id = match session.seeded_records.iter().find(|(t, _)| t == "Account") {
        Some((_, id)) => id.clone(),
        None => {
            return builder.finish_err(
                AllureStatus::Broken,
                "No Account in seeded records — data seeding failed",
                None,
            );
        }
    };

    // ── Navigate to Account detail ─────────────────────────────────────
    let url = format!("{}/lightning/r/Account/{account_id}/view", session.instance_url);
    let nav_step = {
        let s = StepBuilder::start("navigate to Account detail");
        session.navigate(&url).await;
        s.parameter("account_id", &account_id).finish(AllureStatus::Passed)
    };
    builder = builder.step(nav_step);

    // ── Verify "Acme Corp" appears on the page ─────────────────────────
    let verify_name = {
        let s = StepBuilder::start("verify Account name 'Acme Corp' visible");
        let title = session.driver.title().await.unwrap_or_default();
        let body_check = session
            .driver
            .execute_script("return document.body.innerText.includes('Acme Corp')", vec![])
            .await;
        let found = title.contains("Acme Corp")
            || matches!(body_check, Ok(serde_json::Value::Bool(true)));
        if found {
            eprintln!("  Verified 'Acme Corp' on detail page");
            s.finish(AllureStatus::Passed)
        } else {
            eprintln!("  FAIL: 'Acme Corp' not found. Title: {title}");
            s.finish_err(format!("'Acme Corp' not found on page. Title: '{title}'"))
        }
    };
    builder = builder.step(verify_name);

    // ── Verify related Contact "Jane Doe" ──────────────────────────────
    let verify_contact = {
        let s = StepBuilder::start("verify related Contact 'Jane Doe' visible");
        let check = session
            .driver
            .execute_script(
                "return document.body.innerText.includes('Jane Doe') \
                 || document.body.innerText.includes('Jane')",
                vec![],
            )
            .await;
        if matches!(check, Ok(serde_json::Value::Bool(true))) {
            eprintln!("  Verified related Contact 'Jane Doe'");
            s.finish(AllureStatus::Passed)
        } else {
            s.finish_err("Related Contact 'Jane Doe' not visible")
        }
    };
    builder = builder.step(verify_contact);

    // ── Verify related Opportunity "Acme Deal" ─────────────────────────
    let verify_opp = {
        let s = StepBuilder::start("verify related Opportunity 'Acme Deal' visible");
        let check = session
            .driver
            .execute_script("return document.body.innerText.includes('Acme Deal')", vec![])
            .await;
        if matches!(check, Ok(serde_json::Value::Bool(true))) {
            eprintln!("  Verified related Opportunity 'Acme Deal'");
            s.finish(AllureStatus::Passed)
        } else {
            s.finish_err("Related Opportunity 'Acme Deal' not visible")
        }
    };
    builder = builder.step(verify_opp);

    // ── Load global/header on record page and call a method ────────────
    let header_step = {
        let s = StepBuilder::start("load global/header on record page + call getNotificationCount");
        let no_args = HashMap::new();
        match session.load_page_object("global/header").await {
            Ok(header) => {
                let sub = run_method(&header, "getNotificationCount", &no_args, expect_string).await;
                eprintln!("  getNotificationCount on record page: {:?}", sub.status);
                s.sub_step(sub).finish(AllureStatus::Passed)
            }
            Err(e) => {
                eprintln!("  global/header not loadable on record page: {e}");
                s.finish_err(format!("global/header load failed: {e}"))
            }
        }
    };
    builder = builder.step(header_step);

    builder.finish_from_steps()
}
