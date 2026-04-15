//! Process-lifetime shared Salesforce session + runtime for multi-test harness.
//!
//! Each test binary in Rust gets its own OS process, but `#[tokio::test]`
//! functions within a binary each create their own tokio runtime.  Tokio
//! resources (the WebDriver connection holds task handles) created on one
//! runtime cannot be dropped/polled on another, so `tokio::test` breaks
//! any sharing of expensive setup (auth, browser, data seeding).
//!
//! This module solves it: a single `LazyLock<Runtime>` owns one multi-thread
//! runtime for the whole test binary, a mutex serializes tests (single
//! browser → one at a time), and the session is lazy-initialized on first
//! access then leaked into `&'static` so browser resources outlive the
//! test that created them.
//!
//! Integration tests **require** a Salesforce org.  Missing credentials
//! panic with a clear message — there is no silent skip.

use std::sync::{LazyLock, Mutex};

use tokio::runtime::Runtime;

use super::session::SalesforceSession;

/// The one-and-only tokio runtime used by all shared-session tests.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build shared test runtime")
});

/// Serialization mutex — only one test may hold the browser at a time.
static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Lazy-initialized session.  Populated on first access by whichever test
/// runs first; then every subsequent test re-uses the same session.
static SESSION: LazyLock<Mutex<Option<&'static SalesforceSession>>> =
    LazyLock::new(|| Mutex::new(None));

/// Run a test against the shared Salesforce session.
///
/// Serializes all tests via `TEST_LOCK` (single browser), lazy-initializes
/// the session on first access (panics loudly if auth fails), and drives
/// the given future on the shared runtime.
pub fn with_session<F, Fut>(f: F)
where
    F: FnOnce(&'static SalesforceSession) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // Poisoned mutex is unreachable in normal test runs; recover anyway.
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    RUNTIME.block_on(async {
        let session = acquire().await;
        eprintln!("[with_session] session ready — running test");
        // Hard per-test timeout so a hung driver call or infinite loop
        // fails THIS test, not the whole CI job.  The coverage runner
        // iterates the full registry (1454 POs) × DOM checks, which is
        // slow but bounded; anything exceeding 10 min is a real hang.
        let test_fut = f(session);
        if tokio::time::timeout(std::time::Duration::from_secs(600), test_fut)
            .await
            .is_err()
        {
            panic!(
                "test exceeded 10-minute timeout — driver probably hung \
                 (stale element, dead browser session, or runaway selector)"
            );
        }
    });
}

/// Acquire (or initialize) the shared session.  Panics if auth or
/// browser setup fails — integration tests MUST have a real org.
async fn acquire() -> &'static SalesforceSession {
    {
        let slot = SESSION.lock().unwrap();
        if let Some(s) = &*slot {
            return *s;
        }
    }
    // First caller performs setup — TEST_LOCK already serializes callers,
    // so only one setup attempt can be in flight.
    let session = SalesforceSession::setup().await;
    let leaked: &'static SalesforceSession = Box::leak(Box::new(session));
    *SESSION.lock().unwrap() = Some(leaked);
    leaked
}

/// Access the shared session's Allure writer directly.
/// Returns None only if called before any test has initialized the session.
pub fn with_allure<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&utam_test::allure::AllureWriter) -> R,
{
    let slot = SESSION.lock().unwrap();
    slot.map(|s| f(&s.allure))
}

/// Best-effort teardown: delete seeded records + quit the browser.
/// Invoked by the alphabetically-last `zz_teardown` test.
pub fn teardown() {
    let slot = SESSION.lock().unwrap();
    let Some(session) = *slot else {
        // Teardown runs even if no other test initialized the session
        // (e.g. all other tests panicked before reaching `acquire`).
        eprintln!("[teardown] no session was initialized — nothing to clean up");
        return;
    };
    RUNTIME.block_on(async {
        if !session.seeded_records.is_empty() {
            eprintln!("\n=== Teardown: delete seeded records ===");
            for (sobject_type, id) in session.seeded_records.iter().rev() {
                match session.sf_client.delete(sobject_type, id).await {
                    Ok(()) => eprintln!("  Deleted {sobject_type}/{id}"),
                    Err(e) => eprintln!("  Failed to delete {sobject_type}/{id}: {e}"),
                }
            }
        }
        if let Err(e) = session.driver.quit().await {
            eprintln!("  Failed to quit driver: {e}");
        }
    });
}
