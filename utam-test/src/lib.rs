//! UTAM Test Utilities
//!
//! Testing harness and assertion helpers for UTAM page object tests.
//!
//! This crate provides:
//!
//! - [`TestHarness`] - WebDriver session management with screenshot capture
//!   on failure, retry logic, and parallel test support
//! - [`PageObjectAssertions`] - Trait-based assertions for element state
//! - [`ElementAssertion`] - Fluent builder for async assertions with timeouts
//! - [`CollectionAssertions`] - Helpers for validating element collections
//! - [`utam_test!`] - Macro for concise test definitions
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use utam_test::prelude::*;
//!
//! #[tokio::test]
//! async fn test_salesforce_login() -> UtamResult<()> {
//!     let harness = TestHarness::new(Browser::Chrome).await?;
//!     harness.navigate("https://login.salesforce.com").await?;
//!
//!     let login = LoginPage::load(harness.driver()).await?;
//!     login.get_username().await?.assert_visible().await?;
//!
//!     harness.quit().await
//! }
//! ```

pub mod allure;
pub mod assertions;
pub mod harness;

pub use assertions::{
    assert_element, CollectionAssertions, ElementAssertion, PageObjectAssertions,
};
pub use harness::{Browser, HarnessConfig, TestHarness};

pub mod prelude {
    pub use crate::allure::{
        AllureAttachment, AllureCategory, AllureStatus, AllureStep, AllureWriter, StepBuilder,
        TestResultBuilder,
    };
    pub use crate::assertions::{
        assert_element, CollectionAssertions, ElementAssertion, PageObjectAssertions,
    };
    pub use crate::harness::{Browser, HarnessConfig, TestHarness};
    pub use utam_core::prelude::*;
}
