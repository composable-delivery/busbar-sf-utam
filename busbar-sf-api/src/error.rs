use thiserror::Error;

pub type SfApiResult<T> = Result<T, SfApiError>;

#[derive(Debug, Error)]
pub enum SfApiError {
    #[error("Invalid SFDX auth URL: {0}")]
    InvalidAuthUrl(String),

    #[error("OAuth token exchange failed: {0}")]
    TokenExchange(String),

    #[error("API request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API error {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
