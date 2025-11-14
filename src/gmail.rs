use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::oauth;

pub const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

#[derive(Clone)]
pub struct GmailServer {
    user_id: String,
    authenticated: Arc<Mutex<bool>>,
    oauth_manager: Arc<oauth::OAuthManager>,
}

impl GmailServer {
    pub fn new(oauth_manager: Arc<oauth::OAuthManager>) -> Result<Self> {
        Ok(Self {
            user_id: "me".to_string(),
            authenticated: Arc::new(Mutex::new(false)),
            oauth_manager,
        })
    }

    #[allow(dead_code)]
    pub async fn is_authenticated(&self) -> bool {
        *self.authenticated.lock().await
    }

    pub async fn set_authenticated(&self, auth: bool) {
        *self.authenticated.lock().await = auth;
    }

    pub async fn authenticated_client(&self) -> Result<Client> {
        self.check_authentication().await?;
        let token = self
            .oauth_manager
            .get_token()
            .await
            .ok_or_else(|| anyhow::anyhow!("Not authenticated: no token available"))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.access_token).parse().unwrap(),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build authenticated client")?;
        Ok(client)
    }

    pub async fn check_authentication(&self) -> Result<()> {
        if !*self.authenticated.lock().await {
            return Err(anyhow::anyhow!("Not authenticated"));
        }
        Ok(())
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, HttpConfig};

    fn create_test_config() -> Config {
        Config {
            gmail_client_id: Some("test_client_id".to_string()),
            gmail_client_secret: Some("test_client_secret".to_string()),
            app_data_dir: None,
        }
    }

    #[tokio::test]
    async fn test_gmail_server_new() {
        let config = create_test_config();
        let oauth_manager = Arc::new(
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap(),
        );
        let server = GmailServer::new(oauth_manager).unwrap();
        assert_eq!(server.user_id(), "me");
        assert!(!server.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_set_authenticated() {
        let config = create_test_config();
        let oauth_manager = Arc::new(
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap(),
        );
        let server = GmailServer::new(oauth_manager).unwrap();
        server.set_authenticated(true).await;
        assert!(server.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_authenticated_client_not_authenticated() {
        let config = create_test_config();
        let oauth_manager = Arc::new(
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap(),
        );
        let server = GmailServer::new(oauth_manager).unwrap();
        let result = server.authenticated_client().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authenticated_client_authenticated_no_token() {
        let config = create_test_config();
        let oauth_manager = Arc::new(
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap(),
        );
        let server = GmailServer::new(oauth_manager).unwrap();
        server.set_authenticated(true).await;
        let result = server.authenticated_client().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_authentication_not_authenticated() {
        let config = create_test_config();
        let oauth_manager = Arc::new(
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap(),
        );
        let server = GmailServer::new(oauth_manager).unwrap();
        let result = server.check_authentication().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_authentication_authenticated() {
        let config = create_test_config();
        let oauth_manager =
            oauth::OAuthManager::new(config.clone(), HttpConfig::default()).unwrap();
        let token = oauth::OAuthToken {
            access_token: "test_access_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None,
            scope: "test_scope".to_string(),
            created_at: 0,
        };
        oauth_manager.set_token(token).await;
        let server_with_token = GmailServer {
            user_id: "me".to_string(),
            authenticated: Arc::new(Mutex::new(true)),
            oauth_manager: Arc::new(oauth_manager),
        };
        let result = server_with_token.check_authentication().await;
        assert!(result.is_ok());
    }
}
