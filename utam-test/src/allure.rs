//! Allure 2 test result writer
//!
//! Generates Allure result JSON files conforming to the full Allure 2 schema.
//!
//! Supports: parameterized tests, nested steps with timing, labels
//! (epic/feature/story/severity), links, attachments (screenshots, DOM,
//! console logs), and retry/flaky tracking via statusDetails.
//!
//! # Usage
//!
//! ```rust,ignore
//! use utam_test::allure::*;
//!
//! let writer = AllureWriter::from_env();
//!
//! let step = StepBuilder::start("call getNotificationCount")
//!     .parameter("return_type", "String")
//!     .finish(AllureStatus::Passed);
//!
//! let result = TestResultBuilder::new("global/header — all methods")
//!     .full_name("salesforce_live::page_object_methods::global_header")
//!     .label("epic", "Salesforce Browser Testing")
//!     .label("feature", "Page Object Methods")
//!     .label("story", "global/header")
//!     .label("severity", "critical")
//!     .parameter("driver", "webdriver")
//!     .parameter("page_object", "global/header")
//!     .link("Issue #82", "https://github.com/.../issues/82", "issue")
//!     .step(step)
//!     .finish(AllureStatus::Passed);
//!
//! writer.write_result(&result)?;
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Allure schema types
// ---------------------------------------------------------------------------

/// Test execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AllureStatus {
    Passed,
    Failed,
    Broken,
    Skipped,
    Unknown,
}

/// Lifecycle stage of the test
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AllureStage {
    Scheduled,
    Running,
    Finished,
    Pending,
}

/// Key-value label for grouping tests (epic, feature, story, severity, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllureLabel {
    pub name: String,
    pub value: String,
}

/// Parameterized test parameter — creates comparison matrices in the report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllureParameter {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Link to external resource (GitHub issue, Salesforce org, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllureLink {
    pub name: String,
    pub url: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
}

/// Extra status information — retry/flaky tracking, error messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllureStatusDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flaky: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

/// File attachment (screenshot, DOM snapshot, console log, video)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllureAttachment {
    pub name: String,
    pub source: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

/// A single step within a test — supports nesting for sub-steps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllureStep {
    pub name: String,
    pub status: AllureStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<AllureStatusDetails>,
    pub start: u64,
    pub stop: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<AllureStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<AllureParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AllureAttachment>,
}

/// Complete Allure test result — one per logical test, written as JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllureTestResult {
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_html: Option<String>,
    pub status: AllureStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<AllureStatusDetails>,
    pub stage: AllureStage,
    pub start: u64,
    pub stop: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<AllureStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AllureAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<AllureParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<AllureLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<AllureLink>,
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Current time in milliseconds since Unix epoch
fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

/// Monotonic counter for unique ID generation
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique identifier for Allure result files.
///
/// Not a true UUID v4 — uses timestamp + counter + thread hash to guarantee
/// uniqueness within a single process without external dependencies.
fn allure_uuid() -> String {
    let ts = now_ms();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    ts.hash(&mut hasher);
    count.hash(&mut hasher);
    format!("{:?}", std::thread::current().id()).hash(&mut hasher);
    let h = hasher.finish();
    // Format as 32 hex chars (like a UUID without dashes)
    format!("{:016x}{:016x}", ts ^ h, count.wrapping_add(h >> 16))
}

/// Generate a stable history ID from a test's full name.
///
/// Same test name always produces the same history ID, enabling Allure's
/// trend tracking across runs.
fn history_id(full_name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    full_name.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// StepBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing Allure steps with timing.
///
/// Captures start time on creation, stop time on `finish()`.
///
/// ```rust,ignore
/// let step = StepBuilder::start("click setup menu")
///     .sub_step(inner_step)
///     .parameter("element", "setupMenu")
///     .finish(AllureStatus::Passed);
/// ```
pub struct StepBuilder {
    name: String,
    start: u64,
    steps: Vec<AllureStep>,
    parameters: Vec<AllureParameter>,
    attachments: Vec<AllureAttachment>,
}

impl StepBuilder {
    /// Begin a new step, recording the current timestamp.
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: now_ms(),
            steps: Vec::new(),
            parameters: Vec::new(),
            attachments: Vec::new(),
        }
    }

    /// Add a completed sub-step.
    pub fn sub_step(mut self, step: AllureStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add a parameter to this step.
    pub fn parameter(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.push(AllureParameter {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        });
        self
    }

    /// Add an attachment to this step.
    pub fn attachment(mut self, att: AllureAttachment) -> Self {
        self.attachments.push(att);
        self
    }

    /// Finish with a success/failure status, recording the stop timestamp.
    pub fn finish(self, status: AllureStatus) -> AllureStep {
        AllureStep {
            name: self.name,
            status,
            status_details: None,
            start: self.start,
            stop: now_ms(),
            steps: self.steps,
            parameters: self.parameters,
            attachments: self.attachments,
        }
    }

    /// Finish with an error message.
    pub fn finish_err(self, message: impl Into<String>) -> AllureStep {
        AllureStep {
            name: self.name,
            status: AllureStatus::Failed,
            status_details: Some(AllureStatusDetails {
                message: Some(message.into()),
                trace: None,
                known: None,
                muted: None,
                flaky: None,
            }),
            start: self.start,
            stop: now_ms(),
            steps: self.steps,
            parameters: self.parameters,
            attachments: self.attachments,
        }
    }
}

// ---------------------------------------------------------------------------
// TestResultBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a complete Allure test result.
///
/// Captures start time on creation, stop time on `finish()`.
///
/// ```rust,ignore
/// let result = TestResultBuilder::new("global/header methods")
///     .full_name("salesforce::test_header_methods")
///     .label("epic", "Salesforce Browser Testing")
///     .label("severity", "critical")
///     .parameter("driver", "webdriver")
///     .step(step1)
///     .step(step2)
///     .finish(AllureStatus::Passed);
/// ```
pub struct TestResultBuilder {
    name: String,
    full_name: Option<String>,
    description: Option<String>,
    start: u64,
    steps: Vec<AllureStep>,
    parameters: Vec<AllureParameter>,
    labels: Vec<AllureLabel>,
    links: Vec<AllureLink>,
    attachments: Vec<AllureAttachment>,
}

impl TestResultBuilder {
    /// Begin a new test result, recording the current timestamp.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            full_name: None,
            description: None,
            start: now_ms(),
            steps: Vec::new(),
            parameters: Vec::new(),
            labels: Vec::new(),
            links: Vec::new(),
            attachments: Vec::new(),
        }
    }

    /// Set the fully-qualified test name (used for history tracking).
    pub fn full_name(mut self, name: impl Into<String>) -> Self {
        self.full_name = Some(name.into());
        self
    }

    /// Set a human-readable description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a parameterized test parameter (creates matrix views in Allure).
    pub fn parameter(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.push(AllureParameter {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        });
        self
    }

    /// Add a label for grouping (epic, feature, story, severity, suite, etc.).
    pub fn label(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.push(AllureLabel { name: name.into(), value: value.into() });
        self
    }

    /// Add a link to an external resource.
    pub fn link(
        mut self,
        name: impl Into<String>,
        url: impl Into<String>,
        link_type: impl Into<String>,
    ) -> Self {
        self.links.push(AllureLink {
            name: name.into(),
            url: url.into(),
            link_type: Some(link_type.into()),
        });
        self
    }

    /// Add a completed step.
    pub fn step(mut self, step: AllureStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add an attachment to the test result.
    pub fn attachment(mut self, att: AllureAttachment) -> Self {
        self.attachments.push(att);
        self
    }

    /// Derive overall status from step results: failed if any step failed.
    pub fn finish_from_steps(self) -> AllureTestResult {
        let has_failure = self.steps.iter().any(|s| {
            s.status == AllureStatus::Failed || s.status == AllureStatus::Broken
        });
        let status = if has_failure { AllureStatus::Failed } else { AllureStatus::Passed };
        self.finish(status)
    }

    /// Finish with an explicit status.
    pub fn finish(self, status: AllureStatus) -> AllureTestResult {
        let full = self.full_name.clone().unwrap_or_else(|| self.name.clone());
        AllureTestResult {
            uuid: allure_uuid(),
            history_id: Some(history_id(&full)),
            name: self.name,
            full_name: self.full_name,
            description: self.description,
            description_html: None,
            status,
            status_details: None,
            stage: AllureStage::Finished,
            start: self.start,
            stop: now_ms(),
            steps: self.steps,
            attachments: self.attachments,
            parameters: self.parameters,
            labels: self.labels,
            links: self.links,
        }
    }

    /// Finish with an error, setting statusDetails.
    pub fn finish_err(
        self,
        status: AllureStatus,
        message: impl Into<String>,
        trace: Option<String>,
    ) -> AllureTestResult {
        let full = self.full_name.clone().unwrap_or_else(|| self.name.clone());
        AllureTestResult {
            uuid: allure_uuid(),
            history_id: Some(history_id(&full)),
            name: self.name,
            full_name: self.full_name,
            description: self.description,
            description_html: None,
            status,
            status_details: Some(AllureStatusDetails {
                message: Some(message.into()),
                trace,
                known: None,
                muted: None,
                flaky: None,
            }),
            stage: AllureStage::Finished,
            start: self.start,
            stop: now_ms(),
            steps: self.steps,
            attachments: self.attachments,
            parameters: self.parameters,
            labels: self.labels,
            links: self.links,
        }
    }
}

// ---------------------------------------------------------------------------
// AllureWriter
// ---------------------------------------------------------------------------

/// Writes Allure result files to the allure-results directory.
///
/// Handles JSON serialization, attachment file writing, and
/// environment.properties generation.
pub struct AllureWriter {
    results_dir: PathBuf,
}

impl AllureWriter {
    /// Create a writer targeting a specific directory.
    pub fn new(results_dir: impl Into<PathBuf>) -> Self {
        Self { results_dir: results_dir.into() }
    }

    /// Create from `ALLURE_RESULTS_DIR` env var, defaulting to `/tmp/allure-results`.
    pub fn from_env() -> Self {
        let dir = std::env::var("ALLURE_RESULTS_DIR")
            .unwrap_or_else(|_| "/tmp/allure-results".to_string());
        Self::new(dir)
    }

    /// Get the results directory path.
    pub fn results_dir(&self) -> &Path {
        &self.results_dir
    }

    /// Write a test result as a JSON file.
    ///
    /// Returns the path to the written file.
    pub fn write_result(&self, result: &AllureTestResult) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.results_dir)?;
        let filename = format!("{}-result.json", result.uuid);
        let path = self.results_dir.join(&filename);
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    /// Write binary data as an attachment file.
    ///
    /// Returns an `AllureAttachment` referencing the written file, ready
    /// to be added to a step or test result.
    pub fn write_attachment(
        &self,
        name: &str,
        content_type: &str,
        data: &[u8],
    ) -> std::io::Result<AllureAttachment> {
        std::fs::create_dir_all(&self.results_dir)?;
        let ext = match content_type {
            "image/png" => "png",
            "text/html" => "html",
            "text/plain" => "txt",
            "application/json" => "json",
            "video/mp4" => "mp4",
            "text/csv" => "csv",
            _ => "bin",
        };
        let source = format!("{}.{}", allure_uuid(), ext);
        let path = self.results_dir.join(&source);
        std::fs::write(&path, data)?;
        Ok(AllureAttachment {
            name: name.to_string(),
            source,
            content_type: content_type.to_string(),
        })
    }

    /// Write environment.properties for the Allure report.
    ///
    /// Properties appear in the report's "Environment" widget.
    pub fn write_environment(&self, props: &[(&str, &str)]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.results_dir)?;
        let content: String =
            props.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n");
        std::fs::write(self.results_dir.join("environment.properties"), content)
    }

    /// Write categories.json for failure classification in the report.
    pub fn write_categories(&self, categories: &[AllureCategory]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.results_dir)?;
        let json = serde_json::to_string_pretty(categories)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(self.results_dir.join("categories.json"), json)
    }

}

/// Failure category definition for categories.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllureCategory {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matched_statuses: Vec<AllureStatus>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_builder_timing() {
        let step = StepBuilder::start("test step").finish(AllureStatus::Passed);
        assert_eq!(step.name, "test step");
        assert_eq!(step.status, AllureStatus::Passed);
        assert!(step.stop >= step.start);
    }

    #[test]
    fn test_step_builder_with_sub_steps() {
        let inner = StepBuilder::start("inner").finish(AllureStatus::Passed);
        let outer = StepBuilder::start("outer")
            .sub_step(inner)
            .parameter("key", "value")
            .finish(AllureStatus::Passed);
        assert_eq!(outer.steps.len(), 1);
        assert_eq!(outer.parameters.len(), 1);
        assert_eq!(outer.parameters[0].name, "key");
    }

    #[test]
    fn test_step_builder_finish_err() {
        let step = StepBuilder::start("failing step").finish_err("element not found");
        assert_eq!(step.status, AllureStatus::Failed);
        let details = step.status_details.unwrap();
        assert_eq!(details.message.unwrap(), "element not found");
    }

    #[test]
    fn test_result_builder_full() {
        let step = StepBuilder::start("click").finish(AllureStatus::Passed);
        let result = TestResultBuilder::new("my test")
            .full_name("suite::my_test")
            .description("A test")
            .parameter("driver", "webdriver")
            .label("epic", "Testing")
            .label("severity", "critical")
            .link("Issue", "https://example.com", "issue")
            .step(step)
            .finish(AllureStatus::Passed);

        assert_eq!(result.name, "my test");
        assert_eq!(result.full_name.as_deref(), Some("suite::my_test"));
        assert_eq!(result.status, AllureStatus::Passed);
        assert_eq!(result.stage, AllureStage::Finished);
        assert_eq!(result.parameters.len(), 1);
        assert_eq!(result.labels.len(), 2);
        assert_eq!(result.links.len(), 1);
        assert_eq!(result.steps.len(), 1);
        assert!(result.stop >= result.start);
        assert!(result.history_id.is_some());
        assert!(!result.uuid.is_empty());
    }

    #[test]
    fn test_result_finish_from_steps() {
        let pass = StepBuilder::start("ok").finish(AllureStatus::Passed);
        let fail = StepBuilder::start("bad").finish(AllureStatus::Failed);

        let all_pass =
            TestResultBuilder::new("t1").step(pass.clone()).finish_from_steps();
        assert_eq!(all_pass.status, AllureStatus::Passed);

        let has_fail =
            TestResultBuilder::new("t2").step(pass).step(fail).finish_from_steps();
        assert_eq!(has_fail.status, AllureStatus::Failed);
    }

    #[test]
    fn test_result_serializes_to_valid_json() {
        let result = TestResultBuilder::new("serialize test")
            .parameter("p", "v")
            .label("l", "v")
            .finish(AllureStatus::Passed);

        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("\"name\": \"serialize test\""));
        assert!(json.contains("\"status\": \"passed\""));
        assert!(json.contains("\"stage\": \"finished\""));

        // Verify it round-trips
        let parsed: AllureTestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "serialize test");
    }

    #[test]
    fn test_writer_creates_result_file() {
        let dir = std::env::temp_dir().join("allure-test-write");
        let _ = std::fs::remove_dir_all(&dir);

        let writer = AllureWriter::new(&dir);
        let result = TestResultBuilder::new("write test").finish(AllureStatus::Passed);
        let path = writer.write_result(&result).unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with("-result.json"));

        // Verify the file contains valid JSON
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: AllureTestResult = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.name, "write test");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_writer_creates_attachment() {
        let dir = std::env::temp_dir().join("allure-test-attach");
        let _ = std::fs::remove_dir_all(&dir);

        let writer = AllureWriter::new(&dir);
        let att = writer.write_attachment("shot", "image/png", b"fakepng").unwrap();

        assert_eq!(att.name, "shot");
        assert_eq!(att.content_type, "image/png");
        assert!(att.source.ends_with(".png"));
        assert!(dir.join(&att.source).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_writer_environment_properties() {
        let dir = std::env::temp_dir().join("allure-test-env");
        let _ = std::fs::remove_dir_all(&dir);

        let writer = AllureWriter::new(&dir);
        writer.write_environment(&[("Browser", "Chrome"), ("Driver", "webdriver")]).unwrap();

        let content = std::fs::read_to_string(dir.join("environment.properties")).unwrap();
        assert!(content.contains("Browser=Chrome"));
        assert!(content.contains("Driver=webdriver"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_writer_categories() {
        let dir = std::env::temp_dir().join("allure-test-cats");
        let _ = std::fs::remove_dir_all(&dir);

        let writer = AllureWriter::new(&dir);
        writer
            .write_categories(&[
                AllureCategory {
                    name: "Auth failures".into(),
                    description: Some("Session expired".into()),
                    message_regex: Some(".*login.*".into()),
                    trace_regex: None,
                    matched_statuses: vec![AllureStatus::Failed],
                },
                AllureCategory {
                    name: "Element not found".into(),
                    description: None,
                    message_regex: Some(".*ElementNotDefined.*".into()),
                    trace_regex: None,
                    matched_statuses: vec![AllureStatus::Broken],
                },
            ])
            .unwrap();

        let content = std::fs::read_to_string(dir.join("categories.json")).unwrap();
        assert!(content.contains("Auth failures"));
        assert!(content.contains("Element not found"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_allure_uuid_uniqueness() {
        let ids: Vec<String> = (0..100).map(|_| allure_uuid()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "UUIDs must be unique");
    }

    #[test]
    fn test_history_id_stability() {
        let id1 = history_id("suite::test_name");
        let id2 = history_id("suite::test_name");
        let id3 = history_id("suite::different_test");
        assert_eq!(id1, id2, "Same name must produce same history ID");
        assert_ne!(id1, id3, "Different names must produce different IDs");
    }
}
