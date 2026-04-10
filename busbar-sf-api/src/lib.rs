//! Salesforce REST API client for test setup and data seeding.
//!
//! Parses SFDX auth URLs, exchanges refresh tokens for access tokens,
//! and provides SObject CRUD + Composite API operations.

mod auth;
mod client;
mod error;
mod sobject;

pub use auth::SfdxAuthUrl;
pub use client::SalesforceClient;
pub use error::{SfApiError, SfApiResult};
pub use sobject::SObjectRecord;
