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
//! browser → one at a time), and a `OnceCell`-like session holds the
//! auth+browser+registry across tests.
//!
//! Usage:
//! ```ignore
//! #[test]
//! fn test_foo() {
//!     shared::with_session(|session| async move {
//!         let coverage = coverage::discover_and_test(&session, "foo").await;
//!         // ...
//!     });
//! }
//! ```

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

/// Lazy-initialized session.  `None` means "no credentials → skip."
/// Populated on first access by whichever test runs first.
static SESSION: LazyLock<Mutex<SessionSlot>> =
    LazyLock::new(|| Mutex::new(SessionSlot::Uninitialized));

enum SessionSlot {
    Uninitialized,
    Skipped,
    Ready(&'static SalesforceSession),
}

/// Run a test against the shared session.
///
/// Serializes all tests via `TEST_LOCK`, lazy-initializes the session on
/// first access, and drives the given future on the shared runtime.
///
/// If credentials are absent, returns without calling the closure (skip).
pub fn with_session<F, Fut>(f: F)
where
    F: FnOnce(&'static SalesforceSession) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // Poisoned mutex is unreachable in normal test runs; unwrap is fine.
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    RUNTIME.block_on(async {
        let session = acquire_or_skip().await;
        match &session {
            Some(_) => eprintln!("[with_session] session ready — running test"),
            None => eprintln!(
                "[with_session] SKIP — no session (SF_AUTH_URL set={}, CHROMEDRIVER_URL set={})",
                std::env::var("SF_AUTH_URL").map(|v| !v.is_empty()).unwrap_or(false),
                std::env::var("CHROMEDRIVER_URL").is_ok(),
            ),
        }
        if let Some(session) = session {
            f(session).await;
        }
    });
}

/// Populate the session lazily on first call; subsequent calls return
/// the same reference.  Returns `None` if credentials aren't configured.
async fn acquire_or_skip() -> Option<&'static SalesforceSession> {
    // Fast path: already initialized.
    {
        let slot = SESSION.lock().unwrap();
        match &*slot {
            SessionSlot::Ready(s) => return Some(*s),
            SessionSlot::Skipped => return None,
            SessionSlot::Uninitialized => {}
        }
    }

    // Slow path: perform the one-time setup.  We release the lock during
    // setup so we can hold it across await points safely via a
    // double-checked pattern.  Tests are already serialized by TEST_LOCK,
    // so only one setup attempt can be in flight.
    let setup = SalesforceSession::setup().await;
    let mut slot = SESSION.lock().unwrap();
    match setup {
        Some(session) => {
            // Leak the session into a 'static reference.  The session owns
            // a browser driver whose cleanup requires a running runtime;
            // we handle cleanup at process exit via `finalize_on_exit`.
            let leaked: &'static SalesforceSession = Box::leak(Box::new(session));
            *slot = SessionSlot::Ready(leaked);
            Some(leaked)
        }
        None => {
            *slot = SessionSlot::Skipped;
            None
        }
    }
}

/// Access the shared session's Allure writer directly (without running a test).
/// Useful for summary writes that aren't tied to a specific page context.
pub fn with_allure<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&utam_test::allure::AllureWriter) -> R,
{
    let slot = SESSION.lock().unwrap();
    match &*slot {
        SessionSlot::Ready(s) => Some(f(&s.allure)),
        _ => None,
    }
}

/// Best-effort cleanup hook: drops seeded records when the process exits.
///
/// Called explicitly by a `ZZ_teardown` test that runs last.  Rust's test
/// harness has no true "after all" hook, so we rely on alphabetical
/// ordering within a test binary.
pub fn teardown() {
    let slot = SESSION.lock().unwrap();
    if let SessionSlot::Ready(session) = &*slot {
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
}
