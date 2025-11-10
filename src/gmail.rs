use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::oauth::OAuthManager;

pub struct GmailServer {
    oauth_manager: Arc<OAuthManager>,
    user_id: String,
    authenticated: Arc<RwLock<bool>>,
    http_client: Arc<Client>,
}

impl GmailServer {
    pub fn new(config: &Config) -> Result<Self> {
        let oauth_manager = Arc::new(OAuthManager::new(config.clone())?);
        let http_client = Arc::new(Client::new());

        Ok(Self {
            oauth_manager,
            user_id: "me".to_string(),
            authenticated: Arc::new(RwLock::new(false)),
            http_client,
        })
    }

    pub async fn is_authenticated(&self) -> bool {
        *self.authenticated.read().await
    }

    pub async fn check_authentication(&self) -> Result<()> {
        if !self.is_authenticated().await {
            // Try to load existing token
            if let Some(token) = self.oauth_manager.load_token().await? {
                self.oauth_manager.set_token(token).await;
                *self.authenticated.write().await = true;
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "Not authenticated. Please visit /login to authenticate."
            ));
        }
        Ok(())
    }

    pub fn oauth_manager(&self) -> Arc<OAuthManager> {
        self.oauth_manager.clone()
    }

    pub async fn set_authenticated(&self, authenticated: bool) {
        *self.authenticated.write().await = authenticated;
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub async fn get_access_token(&self) -> Result<String> {
        let token = self
            .oauth_manager
            .get_token()
            .await
            .context("No token available")?;
        Ok(token.access_token)
    }

    pub fn http_client(&self) -> Arc<Client> {
        self.http_client.clone()
    }

    /// Create an authenticated HTTP client with bearer token
    pub async fn authenticated_client(&self) -> Result<Client> {
        let token = self.get_access_token().await?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse()?,
        );
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse()?);

        Ok(Client::builder().default_headers(headers).build()?)
    }
}
