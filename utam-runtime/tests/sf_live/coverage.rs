//! Discovery-driven coverage: find every page object whose root selector
//! matches the current DOM and exercise all of their methods + elements.
//!
//! Aggregates per-page-object outcomes into a single summary result per
//! page context, with a histogram of failure categories so systemic
//! patterns are visible at a glance.

use std::collections::HashMap;

use super::failure::FailureKind;
use super::runner::{test_page_object, Outcome};
use super::session::SalesforceSession;
use utam_runtime::discovery::find_known_page_objects;
use utam_test::allure::*;

/// Aggregated results from a coverage run on a single page.
pub struct CoverageResults {
    pub results: Vec<AllureTestResult>,
    pub summary: AllureTestResult,
}

pub async fn discover_and_test(
    session: &SalesforceSession,
    page_context: &str,
) -> CoverageResults {
    eprintln!("\n=== Discover + test on page: {page_context} ===");

    let matched = match find_known_page_objects(session.driver.as_ref(), &session.registry).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  Discovery failed: {e}");
            return CoverageResults {
                results: Vec::new(),
                summary: TestResultBuilder::new(format!("Coverage summary — {page_context}"))
                    .full_name(format!("salesforce_live::coverage::{page_context}::summary"))
                    .label("suite", format!("Coverage — {page_context}"))
                    .parameter("driver", session.driver_name())
                    .parameter("page_context", page_context)
                    .finish_err(AllureStatus::Broken, format!("discovery failed: {e}"), None),
            };
        }
    };

    eprintln!("  Discovered {} matching page objects", matched.len());

    let mut results = Vec::new();
    let mut totals = Outcome {
        methods_passed: 0,
        methods_failed: 0,
        methods_skipped: 0,
        elements_passed: 0,
        elements_failed: 0,
        loaded: true,
        failure_kinds: HashMap::new(),
    };
    let mut loaded_count = 0;
    let mut broken_count = 0;

    for m in &matched {
        let (result, outcome) = test_page_object(session, &m.name, page_context).await;
        if outcome.loaded {
            loaded_count += 1;
            totals.methods_passed += outcome.methods_passed;
            totals.methods_failed += outcome.methods_failed;
            totals.methods_skipped += outcome.methods_skipped;
            totals.elements_passed += outcome.elements_passed;
            totals.elements_failed += outcome.elements_failed;
        } else {
            broken_count += 1;
        }
        for (kind, n) in &outcome.failure_kinds {
            *totals.failure_kinds.entry(*kind).or_insert(0) += n;
        }

        let status = match result.status {
            AllureStatus::Passed => "PASS",
            AllureStatus::Failed => "FAIL",
            AllureStatus::Broken => "BROKEN",
            AllureStatus::Skipped => "SKIP",
            AllureStatus::Unknown => "?",
        };
        eprintln!(
            "  [{status:6}] {:48} methods: {}p/{}f/{}s  elements: {}p/{}f",
            m.name,
            outcome.methods_passed,
            outcome.methods_failed,
            outcome.methods_skipped,
            outcome.elements_passed,
            outcome.elements_failed,
        );
        results.push(result);
    }

    // Sort failure kinds by count (descending) for the summary
    let mut sorted_kinds: Vec<(FailureKind, usize)> =
        totals.failure_kinds.iter().map(|(k, v)| (*k, *v)).collect();
    sorted_kinds.sort_by(|a, b| b.1.cmp(&a.1));

    eprintln!(
        "\n  Summary [{page_context}]: {} POs matched, {} loaded, {} broken",
        matched.len(),
        loaded_count,
        broken_count
    );
    eprintln!(
        "    Methods: {} passed, {} failed, {} skipped",
        totals.methods_passed, totals.methods_failed, totals.methods_skipped
    );
    eprintln!(
        "    Elements: {} passed, {} failed",
        totals.elements_passed, totals.elements_failed
    );
    if !sorted_kinds.is_empty() {
        eprintln!("    Failure breakdown:");
        for (kind, count) in &sorted_kinds {
            eprintln!("      {:30} {}", kind.name(), count);
        }
    }

    // Build the summary AllureTestResult
    let mut builder = TestResultBuilder::new(format!("Coverage summary — {page_context}"))
        .full_name(format!("salesforce_live::coverage::{page_context}::summary"))
        .description(format!(
            "Aggregate coverage across {} page objects discovered on the {} page. \
             Each matched page object was loaded, every method was called with \
             synthesized or curated arguments, every public element was resolved. \
             Failure kinds are aggregated below to surface systemic patterns.",
            matched.len(),
            page_context
        ))
        .label("epic", "Salesforce Browser Testing")
        .label("feature", "Page Object Coverage")
        .label("story", format!("{page_context} coverage"))
        .label("suite", format!("Coverage — {page_context}"))
        .label("severity", "normal")
        .parameter("driver", session.driver_name())
        .parameter("page_context", page_context)
        .parameter("matched", matched.len().to_string())
        .parameter("loaded", loaded_count.to_string())
        .parameter("broken", broken_count.to_string())
        .parameter("methods_passed", totals.methods_passed.to_string())
        .parameter("methods_failed", totals.methods_failed.to_string())
        .parameter("methods_skipped", totals.methods_skipped.to_string())
        .parameter("elements_passed", totals.elements_passed.to_string())
        .parameter("elements_failed", totals.elements_failed.to_string())
        .step(
            StepBuilder::start("discovery")
                .parameter("page_objects_matched", matched.len().to_string())
                .finish(AllureStatus::Passed),
        );

    // Add one step per failure kind with the count — these show up as
    // sortable/filterable rows in the Allure report.
    for (kind, count) in &sorted_kinds {
        builder = builder.step(
            StepBuilder::start(format!("failure category: {}", kind.name()))
                .parameter("count", count.to_string())
                .finish(AllureStatus::Passed),
        );
        builder = builder.parameter(format!("failures_{}", kind.name()), count.to_string());
    }

    let summary = builder.finish(AllureStatus::Passed);
    CoverageResults { results, summary }
}
