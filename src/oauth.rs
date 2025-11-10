use anyhow::{Context, Result};
use oauth2::{
    basic::BasicClient,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl, Scope, TokenResponse, TokenUrl,
    EndpointSet,
};
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use url::Url;

use crate::config::Config;
use crate::utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub expires_at: Option<u64>,
}

impl Token {
    pub fn from_token_response<TR: TokenResponse>(token: &TR) -> Self {
        let expires_at = token.expires_in()
            .and_then(|expires_in| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|now| now.as_secs() + expires_in.as_secs())
            });

        Self {
            access_token: token.access_token().secret().clone(),
            refresh_token: token.refresh_token().map(|rt| rt.secret().clone()),
            expires_in: token.expires_in().map(|d| d.as_secs()),
            expires_at,
        }
    }
}

pub struct OAuthManager {
    client: BasicClient<EndpointSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, EndpointSet>,
    #[allow(dead_code)]
    config: Config,
    token: Arc<RwLock<Option<Token>>>,
}

impl OAuthManager {
    pub fn new(config: Config) -> Result<Self> {
        let client_id = ClientId::new(
            config.gmail_client_id.clone()
                .ok_or_else(|| anyhow::anyhow!("GMAIL_CLIENT_ID not set"))?
        );
        let client_secret = ClientSecret::new(
            config.gmail_client_secret.clone()
                .ok_or_else(|| anyhow::anyhow!("GMAIL_CLIENT_SECRET not set"))?
        );
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .context("Invalid authorization URL")?;
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .context("Invalid token URL")?;
        let redirect_url = RedirectUrl::new(config.oauth_redirect_url())
            .context("Invalid redirect URL")?;

        let client = BasicClient::new(client_id)
            .set_client_secret(client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        Ok(Self {
            client,
            config,
            token: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn load_token(&self) -> Result<Option<Token>> {
        let token_file = utils::get_app_file_path("token.json");

        if !token_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&token_file)
            .context("Failed to read token file")?;
        let token: Token = serde_json::from_str(&content)
            .context("Failed to parse token file")?;

        // Validate token by checking expiration
        if let Some(expires_at) = token.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now >= expires_at {
                warn!("Token expired, will need to re-authenticate");
                return Ok(None);
            }
        }

        Ok(Some(token))
    }

    pub async fn save_token(&self, token: &Token) -> Result<()> {
        let token_file = utils::get_app_file_path("token.json");
        let content = serde_json::to_string_pretty(token)
            .context("Failed to serialize token")?;
        std::fs::write(&token_file, content)
            .context("Failed to write token file")?;
        Ok(())
    }

    pub fn get_authorization_url(&self) -> Result<(Url, String)> {
        let (auth_url, csrf_token) = self.client
            .authorize_url(|| oauth2::CsrfToken::new_random())
            .add_scope(Scope::new("https://www.googleapis.com/auth/gmail.readonly".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/gmail.compose".to_string()))
            .url();

        Ok((auth_url, csrf_token.secret().clone()))
    }

    pub async fn exchange_code(&self, code: &str) -> Result<Token> {
        let code = AuthorizationCode::new(code.to_string());
        let http_client = reqwest::Client::new();
        let token_result = self.client
            .exchange_code(code)
            .request_async(&http_client)
            .await
            .context("Failed to exchange authorization code")?;

        let token = Token::from_token_response(&token_result);
        self.save_token(&token).await?;

        *self.token.write().await = Some(token.clone());
        Ok(token)
    }

    pub async fn get_token(&self) -> Option<Token> {
        self.token.read().await.clone()
    }

    pub async fn set_token(&self, token: Token) {
        *self.token.write().await = Some(token);
    }
}

