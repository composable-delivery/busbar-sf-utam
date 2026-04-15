//! SFDX auth URL parsing and OAuth token exchange.
//!
//! Auth URL format: `force://<clientId>:<clientSecret>:<refreshToken>@<instanceUrl>`
//! The client secret may be empty (common for PlatformCLI connected app).

use serde::Deserialize;

use crate::error::{SfApiError, SfApiResult};

/// Parsed SFDX auth URL components.
#[derive(Debug, Clone)]
pub struct SfdxAuthUrl {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: String,
    pub instance_url: String,
}

impl SfdxAuthUrl {
    /// Parse an SFDX auth URL string.
    ///
    /// Format: `force://<clientId>:<clientSecret>:<refreshToken>@<loginUrl>`
    pub fn parse(url: &str) -> SfApiResult<Self> {
        let url = url.trim();
        let body = url
            .strip_prefix("force://")
            .ok_or_else(|| SfApiError::InvalidAuthUrl("must start with force://".into()))?;

        let (creds, host) = body
            .rsplit_once('@')
            .ok_or_else(|| SfApiError::InvalidAuthUrl("missing @ separator".into()))?;

        // Split on first two colons: clientId:clientSecret:refreshToken
        let parts: Vec<&str> = creds.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Err(SfApiError::InvalidAuthUrl(
                "expected clientId:clientSecret:refreshToken".into(),
            ));
        }

        let client_id = parts[0].to_string();
        let client_secret = if parts[1].is_empty() { None } else { Some(parts[1].to_string()) };
        let refresh_token = parts[2].to_string();

        if refresh_token.is_empty() {
            return Err(SfApiError::InvalidAuthUrl("refresh token is empty".into()));
        }

        Ok(Self {
            client_id,
            client_secret,
            refresh_token,
            instance_url: format!("https://{host}"),
        })
    }

    /// Parse from the `SF_AUTH_URL` environment variable.
    pub fn from_env() -> SfApiResult<Self> {
        let url = std::env::var("SF_AUTH_URL")
            .map_err(|_| SfApiError::InvalidAuthUrl("SF_AUTH_URL not set".into()))?;
        Self::parse(&url)
    }

    /// Exchange the refresh token for an access token.
    pub async fn exchange_token(&self) -> SfApiResult<TokenResponse> {
        let client = reqwest::Client::new();

        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("client_id", &self.client_id),
            ("refresh_token", &self.refresh_token),
        ];

        let secret_ref;
        if let Some(ref secret) = self.client_secret {
            secret_ref = secret.as_str();
            form.push(("client_secret", secret_ref));
        }

        let url = format!("{}/services/oauth2/token", self.instance_url);
        let resp = client.post(&url).form(&form).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(SfApiError::TokenExchange(format!("HTTP {status}: {body}")));
        }

        let token: TokenResponse = resp.json().await?;
        Ok(token)
    }
}

/// Response from the OAuth2 token endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub instance_url: String,
    pub id: String,
    pub token_type: String,
    pub issued_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_url_with_empty_secret() {
        let auth =
            SfdxAuthUrl::parse("force://PlatformCLI::5Aep861mdFkrefreshtoken@test.salesforce.com")
                .unwrap();
        assert_eq!(auth.client_id, "PlatformCLI");
        assert!(auth.client_secret.is_none());
        assert_eq!(auth.refresh_token, "5Aep861mdFkrefreshtoken");
        assert_eq!(auth.instance_url, "https://test.salesforce.com");
    }

    #[test]
    fn parse_auth_url_with_secret() {
        let auth = SfdxAuthUrl::parse("force://myapp:mysecret:myrefreshtoken@login.salesforce.com")
            .unwrap();
        assert_eq!(auth.client_id, "myapp");
        assert_eq!(auth.client_secret.as_deref(), Some("mysecret"));
        assert_eq!(auth.refresh_token, "myrefreshtoken");
        assert_eq!(auth.instance_url, "https://login.salesforce.com");
    }

    #[test]
    fn parse_auth_url_with_my_domain() {
        let auth =
            SfdxAuthUrl::parse("force://PlatformCLI::token123@mycompany.scratch.my.salesforce.com")
                .unwrap();
        assert_eq!(auth.instance_url, "https://mycompany.scratch.my.salesforce.com");
    }

    #[test]
    fn parse_invalid_url() {
        assert!(SfdxAuthUrl::parse("https://bad").is_err());
        assert!(SfdxAuthUrl::parse("force://nocolons@host").is_err());
        assert!(SfdxAuthUrl::parse("force://a:b:@host").is_err()); // empty token
    }
}
