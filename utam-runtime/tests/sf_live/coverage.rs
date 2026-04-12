//! Discovery-driven coverage: find every page object whose root selector
//! matches the current DOM and exercise all of their methods + elements.
//!
//! This is how we scale from 5 hand-written tests to every page object that
//! can actually load on each page the browser visits.

use super::runner::{test_page_object, Outcome};
use super::session::SalesforceSession;
use utam_runtime::discovery::find_known_page_objects;
use utam_test::allure::*;

/// Scan the current DOM for all registered page objects that match, then
/// run the generic test runner against each.
///
/// Returns the list of Allure results plus a per-page-object summary step
/// that gets included as a dedicated "coverage summary" test result.
pub async fn discover_and_test(
    session: &SalesforceSession,
    page_context: &str,
) -> CoverageResults {
    eprintln!("\n=== Discover + test on page: {page_context} ===");

    // Scan the DOM for matching page objects
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
    for m in &matched {
        eprintln!(
            "    {} ({} methods, {} elements)",
            m.name, m.method_count, m.element_count
        );
    }

    let mut results = Vec::new();
    let mut total = Outcome {
        methods_passed: 0,
        methods_failed: 0,
        methods_skipped: 0,
        elements_passed: 0,
        elements_failed: 0,
        loaded: true,
    };
    let mut loaded_count = 0;
    let mut broken_count = 0;

    for m in &matched {
        let (result, outcome) = test_page_object(session, &m.name, page_context).await;
        if outcome.loaded {
            loaded_count += 1;
            total.methods_passed += outcome.methods_passed;
            total.methods_failed += outcome.methods_failed;
            total.methods_skipped += outcome.methods_skipped;
            total.elements_passed += outcome.elements_passed;
            total.elements_failed += outcome.elements_failed;
        } else {
            broken_count += 1;
        }
        let status = match result.status {
            AllureStatus::Passed => "PASS",
            AllureStatus::Failed => "FAIL",
            AllureStatus::Broken => "BROKEN",
            AllureStatus::Skipped => "SKIP",
            AllureStatus::Unknown => "?",
        };
        eprintln!(
            "  [{status}] {} — methods: {}p/{}f/{}s, elements: {}p/{}f",
            m.name,
            outcome.methods_passed,
            outcome.methods_failed,
            outcome.methods_skipped,
            outcome.elements_passed,
            outcome.elements_failed,
        );
        results.push(result);
    }

    // Build a summary result that shows aggregate stats for this page context
    let summary = TestResultBuilder::new(format!("Coverage summary — {page_context}"))
        .full_name(format!("salesforce_live::coverage::{page_context}::summary"))
        .description(format!(
            "Aggregate coverage across {} page objects discovered on the {} page. \
             Each matched page object was loaded, every method was called with \
             synthesized or curated arguments, every public element was resolved.",
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
        .parameter("methods_passed", total.methods_passed.to_string())
        .parameter("methods_failed", total.methods_failed.to_string())
        .parameter("methods_skipped", total.methods_skipped.to_string())
        .parameter("elements_passed", total.elements_passed.to_string())
        .parameter("elements_failed", total.elements_failed.to_string())
        .step(
            StepBuilder::start("discovery")
                .parameter("page_objects_matched", matched.len().to_string())
                .finish(AllureStatus::Passed),
        )
        .finish(AllureStatus::Passed);

    eprintln!(
        "\n  Summary [{page_context}]: {} POs matched, {} loaded, {} broken",
        matched.len(),
        loaded_count,
        broken_count
    );
    eprintln!(
        "    Methods: {} passed, {} failed, {} skipped",
        total.methods_passed, total.methods_failed, total.methods_skipped
    );
    eprintln!(
        "    Elements: {} passed, {} failed",
        total.elements_passed, total.elements_failed
    );

    CoverageResults { results, summary }
}

/// Aggregated results from a coverage run on a single page.
pub struct CoverageResults {
    /// One AllureTestResult per tested page object.
    pub results: Vec<AllureTestResult>,
    /// A summary AllureTestResult with aggregate stats.
    pub summary: AllureTestResult,
}
