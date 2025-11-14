use crate::config::{Config, HttpConfig};
use anyhow::{Context, Result};
use oauth2::reqwest;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointNotSet, EndpointSet, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub created_at: u64,
}

impl OAuthToken {
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.created_at + self.expires_in
    }
}

pub struct OAuthManager {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    token: Arc<Mutex<Option<OAuthToken>>>,
    token_file: PathBuf,
}

impl OAuthManager {
    pub fn new(config: Config, http_config: HttpConfig) -> Result<Self> {
        let client_id = config
            .gmail_client_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("GMAIL_CLIENT_ID not set"))?;
        let client_secret = config
            .gmail_client_secret
            .clone()
            .ok_or_else(|| anyhow::anyhow!("GMAIL_CLIENT_SECRET not set"))?;
        let redirect_url = http_config.oauth_redirect_url();

        let client = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::new(
                "https://accounts.google.com/o/oauth2/auth".to_string(),
            )?)
            .set_token_uri(TokenUrl::new(
                "https://oauth2.googleapis.com/token".to_string(),
            )?)
            .set_redirect_uri(RedirectUrl::new(redirect_url)?);

        let token_file = crate::utils::get_app_file_path(&config, "token.json")?;

        Ok(Self {
            client,
            token: Arc::new(Mutex::new(None)),
            token_file,
        })
    }

    pub fn get_authorization_url(&self) -> Result<(String, String)> {
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.modify".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.readonly".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            ))
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .url();

        Ok((auth_url.to_string(), csrf_token.secret().to_string()))
    }

    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken> {
        let async_http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(&async_http_client)
            .await
            .context("Failed to exchange authorization code")?;

        let oauth_token = OAuthToken {
            access_token: token_response.access_token().secret().to_string(),
            token_type: token_response.token_type().as_ref().to_string(),
            expires_in: token_response.expires_in().unwrap_or_default().as_secs(),
            refresh_token: token_response
                .refresh_token()
                .map(|t| t.secret().to_string()),
            scope: token_response.scopes().map_or("".to_string(), |s| {
                s.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.save_token(&oauth_token).await?;
        *self.token.lock().await = Some(oauth_token.clone());

        Ok(oauth_token)
    }

    #[allow(dead_code)]
    pub async fn refresh_token(&self) -> Result<OAuthToken> {
        let async_http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let old_token = self.get_token().await;
        let refresh_token_str = old_token
            .and_then(|t| t.refresh_token)
            .ok_or_else(|| anyhow::anyhow!("No refresh token found"))?;

        let token_response = self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token_str))
            .request_async(&async_http_client)
            .await
            .context("Failed to refresh token")?;

        let new_oauth_token = OAuthToken {
            access_token: token_response.access_token().secret().to_string(),
            token_type: token_response.token_type().as_ref().to_string(),
            expires_in: token_response.expires_in().unwrap_or_default().as_secs(),
            refresh_token: token_response
                .refresh_token()
                .map(|t| t.secret().to_string()),
            scope: token_response.scopes().map_or("".to_string(), |s| {
                s.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.save_token(&new_oauth_token).await?;
        *self.token.lock().await = Some(new_oauth_token.clone());

        Ok(new_oauth_token)
    }

    pub async fn get_token(&self) -> Option<OAuthToken> {
        self.token.lock().await.clone()
    }

    pub async fn set_token(&self, token: OAuthToken) {
        *self.token.lock().await = Some(token);
    }

    pub async fn save_token(&self, token: &OAuthToken) -> Result<()> {
        let token_json =
            serde_json::to_string_pretty(token).context("Failed to serialize token")?;
        fs::write(&self.token_file, token_json).context("Failed to write token file")?;
        info!("🔑 Token saved to {}", self.token_file.display());
        Ok(())
    }

    pub async fn load_token(&self) -> Result<Option<OAuthToken>> {
        if self.token_file.exists() {
            let token_json =
                fs::read_to_string(&self.token_file).context("Failed to read token file")?;
            let token: OAuthToken =
                serde_json::from_str(&token_json).context("Failed to deserialize token")?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn token_file_path(&self) -> &Path {
        &self.token_file
    }
}
