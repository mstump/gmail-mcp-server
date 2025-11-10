use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "gmail-mcp-server")]
#[command(about = "Gmail MCP Server - Rust implementation")]
pub struct Config {
    /// HTTP server port
    #[arg(long, env = "PORT", default_value = "8080")]
    pub port: u16,

    /// Gmail OAuth Client ID
    #[arg(long, env = "GMAIL_CLIENT_ID")]
    pub gmail_client_id: Option<String>,

    /// Gmail OAuth Client Secret
    #[arg(long, env = "GMAIL_CLIENT_SECRET")]
    pub gmail_client_secret: Option<String>,

    /// OAuth redirect URL (defaults to http://localhost:{port}/callback)
    #[arg(long, env = "OAUTH_REDIRECT_URL")]
    pub oauth_redirect_url: Option<String>,

    /// Prometheus metrics route path (defaults to /metrics)
    #[arg(long, env = "METRICS_ROUTE", default_value = "/metrics")]
    pub metrics_route: String,

    /// MCP route path (defaults to /mcp)
    #[arg(long, env = "MCP_ROUTE", default_value = "/mcp")]
    pub mcp_route: String,

    /// Login route path (defaults to /login)
    #[arg(long, env = "LOGIN_ROUTE", default_value = "/login")]
    pub login_route: String,
}

impl Config {
    pub fn oauth_redirect_url(&self) -> String {
        self.oauth_redirect_url.clone()
            .unwrap_or_else(|| format!("http://localhost:{}/callback", self.port))
    }

    pub fn metrics_route(&self) -> &str {
        &self.metrics_route
    }

    pub fn mcp_route(&self) -> &str {
        &self.mcp_route
    }

    pub fn login_route(&self) -> &str {
        &self.login_route
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_redirect_url_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: Some("https://example.com/callback".to_string()),
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.oauth_redirect_url(), "https://example.com/callback");
    }

    #[test]
    fn test_oauth_redirect_url_falls_back_to_default() {
        let config = Config {
            port: 3000,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.oauth_redirect_url(), "http://localhost:3000/callback");
    }

    #[test]
    fn test_oauth_redirect_url_default_with_different_port() {
        let config = Config {
            port: 9000,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.oauth_redirect_url(), "http://localhost:9000/callback");
    }

    #[test]
    fn test_metrics_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/custom-metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.metrics_route(), "/custom-metrics");
    }

    #[test]
    fn test_metrics_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.metrics_route(), "/metrics");
    }

    #[test]
    fn test_mcp_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/custom-mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.mcp_route(), "/custom-mcp");
    }

    #[test]
    fn test_mcp_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.mcp_route(), "/mcp");
    }

    #[test]
    fn test_login_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/custom-login".to_string(),
        };
        assert_eq!(config.login_route(), "/custom-login");
    }

    #[test]
    fn test_login_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
        };
        assert_eq!(config.login_route(), "/login");
    }
}

