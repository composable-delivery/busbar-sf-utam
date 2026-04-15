//! Salesforce REST API client.
//!
//! Provides SObject CRUD, Composite API, and frontdoor URL generation.

use serde::Deserialize;

use crate::auth::{SfdxAuthUrl, TokenResponse};
use crate::error::{SfApiError, SfApiResult};
use crate::sobject::SObjectRecord;

const API_VERSION: &str = "v62.0";

/// Authenticated Salesforce REST API client.
pub struct SalesforceClient {
    http: reqwest::Client,
    pub instance_url: String,
    pub access_token: String,
}

impl SalesforceClient {
    /// Create a client from an SFDX auth URL by exchanging the refresh token.
    pub async fn from_auth_url(auth: &SfdxAuthUrl) -> SfApiResult<Self> {
        let token = auth.exchange_token().await?;
        Ok(Self::from_token(token))
    }

    /// Create a client from an already-obtained token response.
    pub fn from_token(token: TokenResponse) -> Self {
        Self {
            http: reqwest::Client::new(),
            instance_url: token.instance_url,
            access_token: token.access_token,
        }
    }

    /// Create a client from raw access token and instance URL.
    pub fn new(instance_url: String, access_token: String) -> Self {
        Self { http: reqwest::Client::new(), instance_url, access_token }
    }

    /// Generate a frontdoor URL for browser authentication.
    pub fn frontdoor_url(&self) -> String {
        format!("{}/secur/frontdoor.jsp?sid={}", self.instance_url, self.access_token)
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/services/data/{}{}", self.instance_url, API_VERSION, path)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    // ── SObject CRUD ────────────────────────────────────────────────────

    /// Create a record. Returns the new record ID.
    pub async fn create(&self, sobject_type: &str, record: &SObjectRecord) -> SfApiResult<String> {
        let url = self.api_url(&format!("/sobjects/{sobject_type}"));
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&record)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let body = resp.text().await?;

        if status != 201 {
            return Err(SfApiError::ApiError { status, body });
        }

        let result: CreateResponse = serde_json::from_str(&body)?;
        Ok(result.id)
    }

    /// Read a record by ID.
    pub async fn read(&self, sobject_type: &str, id: &str) -> SfApiResult<SObjectRecord> {
        let url = self.api_url(&format!("/sobjects/{sobject_type}/{id}"));
        let resp = self.http.get(&url).header("Authorization", self.auth_header()).send().await?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let body = resp.text().await?;
            return Err(SfApiError::ApiError { status, body });
        }

        Ok(resp.json().await?)
    }

    /// Delete a record by ID.
    pub async fn delete(&self, sobject_type: &str, id: &str) -> SfApiResult<()> {
        let url = self.api_url(&format!("/sobjects/{sobject_type}/{id}"));
        let resp =
            self.http.delete(&url).header("Authorization", self.auth_header()).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            return Err(SfApiError::ApiError { status, body });
        }

        Ok(())
    }

    /// Run a SOQL query. Returns the records.
    pub async fn query(&self, soql: &str) -> SfApiResult<Vec<SObjectRecord>> {
        let url = self.api_url("/query");
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .query(&[("q", soql)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            return Err(SfApiError::ApiError { status, body });
        }

        let result: QueryResponse = resp.json().await?;
        Ok(result.records)
    }

    // ── Composite API ───────────────────────────────────────────────────

    /// Execute a composite request with dependent subrequests.
    /// Supports `@{referenceId.field}` expressions between subrequests.
    pub async fn composite(
        &self,
        requests: Vec<CompositeSubrequest>,
        all_or_none: bool,
    ) -> SfApiResult<Vec<CompositeSubresponse>> {
        let url = self.api_url("/composite");
        let body = serde_json::json!({
            "allOrNone": all_or_none,
            "compositeRequest": requests
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            return Err(SfApiError::ApiError { status, body });
        }

        let result: CompositeResponse = resp.json().await?;
        Ok(result.composite_response)
    }

    /// Create multiple related records in one call using composite API.
    /// Returns a map of referenceId → created record ID.
    pub async fn create_related(
        &self,
        records: Vec<(&str, &str, SObjectRecord)>,
    ) -> SfApiResult<Vec<String>> {
        let requests: Vec<CompositeSubrequest> = records
            .into_iter()
            .map(|(ref_id, sobject_type, record)| CompositeSubrequest {
                method: "POST".to_string(),
                url: format!("/services/data/{API_VERSION}/sobjects/{sobject_type}"),
                reference_id: ref_id.to_string(),
                body: Some(serde_json::to_value(&record).unwrap_or_default()),
            })
            .collect();

        let responses = self.composite(requests, true).await?;
        let mut ids = Vec::new();
        for resp in &responses {
            if resp.http_status_code >= 300 {
                let body = serde_json::to_string(&resp.body).unwrap_or_default();
                return Err(SfApiError::ApiError { status: resp.http_status_code, body });
            }
            if let Some(id) = resp.body.get("id").and_then(|v| v.as_str()) {
                ids.push(id.to_string());
            }
        }
        Ok(ids)
    }
}

#[derive(Deserialize)]
struct CreateResponse {
    id: String,
}

#[derive(Deserialize)]
struct QueryResponse {
    records: Vec<SObjectRecord>,
}

/// A single subrequest in a composite call.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeSubrequest {
    pub method: String,
    pub url: String,
    pub reference_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// A single subresponse from a composite call.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeSubresponse {
    pub http_status_code: u16,
    pub reference_id: String,
    pub body: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompositeResponse {
    composite_response: Vec<CompositeSubresponse>,
}
