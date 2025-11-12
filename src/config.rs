use clap::Parser;
use std::path::PathBuf;

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

    /// HTTP stream route path (defaults to /stream)
    #[arg(long, env = "HTTP_STREAM_ROUTE", default_value = "/stream")]
    pub http_stream_route: String,

    /// SSE configuration
    #[command(flatten)]
    pub sse_config: SseConfig,

    /// Login route path (defaults to /login)
    #[arg(long, env = "LOGIN_ROUTE", default_value = "/login")]
    pub login_route: String,

    /// OAuth callback route path (defaults to /callback)
    #[arg(long, env = "CALLBACK_ROUTE", default_value = "/callback")]
    pub callback_route: String,

    /// Health check route path (defaults to /health)
    #[arg(long, env = "HEALTH_ROUTE", default_value = "/health")]
    pub health_route: String,

    /// Root route path (defaults to /)
    #[arg(long, env = "ROOT_ROUTE", default_value = "/")]
    pub root_route: String,

    /// Application data directory (defaults to platform-specific location)
    #[arg(long, env = "APP_DATA_DIR")]
    pub app_data_dir: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub struct SseConfig {
    /// SSE router prefix path (defaults to /sse)
    #[arg(long, env = "SSE_PREFIX", default_value = "/sse")]
    pub sse_prefix: String,
}

impl SseConfig {
    /// Get the SSE route path (fixed to /sse)
    pub fn sse_route(&self) -> &str {
        "/sse"
    }

    /// Get the SSE POST route path (fixed to /message)
    pub fn sse_post_route(&self) -> &str {
        "/message"
    }

    /// Get the SSE prefix path
    pub fn sse_prefix(&self) -> &str {
        &self.sse_prefix
    }
}

impl Config {
    pub fn oauth_redirect_url(&self) -> String {
        self.oauth_redirect_url
            .clone()
            .unwrap_or_else(|| format!("http://localhost:{}/callback", self.port))
    }

    pub fn metrics_route(&self) -> &str {
        &self.metrics_route
    }

    pub fn http_stream_route(&self) -> &str {
        &self.http_stream_route
    }

    pub fn sse_route(&self) -> &str {
        self.sse_config.sse_route()
    }

    pub fn sse_post_route(&self) -> &str {
        self.sse_config.sse_post_route()
    }

    pub fn sse_prefix(&self) -> &str {
        self.sse_config.sse_prefix()
    }

    pub fn login_route(&self) -> &str {
        &self.login_route
    }

    pub fn callback_route(&self) -> &str {
        &self.callback_route
    }

    pub fn health_route(&self) -> &str {
        &self.health_route
    }

    pub fn root_route(&self) -> &str {
        &self.root_route
    }

    /// Get the application data directory, using configured value or defaulting to platform-specific location
    pub fn app_data_dir(&self) -> PathBuf {
        if let Some(ref dir) = self.app_data_dir {
            return dir.clone();
        }

        // Default platform-specific behavior
        if cfg!(windows) {
            std::env::var("APPDATA")
                .map(|appdata| PathBuf::from(appdata).join("gmail-mcp-server-data"))
                .unwrap_or_else(|_| PathBuf::from(".").join("gmail-mcp-server-data"))
        } else {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".gmail-mcp-server-data"))
                .unwrap_or_else(|_| PathBuf::from(".").join("gmail-mcp-server-data"))
        }
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
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
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
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(
            config.oauth_redirect_url(),
            "http://localhost:3000/callback"
        );
    }

    #[test]
    fn test_oauth_redirect_url_default_with_different_port() {
        let config = Config {
            port: 9000,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(
            config.oauth_redirect_url(),
            "http://localhost:9000/callback"
        );
    }

    #[test]
    fn test_metrics_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/custom-metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
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
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.metrics_route(), "/metrics");
    }


    #[test]
    fn test_login_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/custom-login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
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
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.login_route(), "/login");
    }

    #[test]
    fn test_app_data_dir_uses_configured_value() {
        let custom_dir = PathBuf::from("/custom/path");
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: Some(custom_dir.clone()),
        };
        assert_eq!(config.app_data_dir(), custom_dir);
    }

    #[test]
    fn test_app_data_dir_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        let dir = config.app_data_dir();
        // Should end with "gmail-mcp-server-data" or ".gmail-mcp-server-data" depending on platform
        assert!(dir.to_string_lossy().contains("gmail-mcp-server-data"));
    }

    #[test]
    fn test_http_stream_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/custom-stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.http_stream_route(), "/custom-stream");
    }

    #[test]
    fn test_http_stream_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.http_stream_route(), "/stream");
    }

    #[test]
    fn test_sse_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.sse_route(), "/sse");
    }

    #[test]
    fn test_sse_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.sse_route(), "/sse");
    }

    #[test]
    fn test_sse_post_route_uses_configured_value() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.sse_post_route(), "/message");
    }

    #[test]
    fn test_sse_post_route_falls_back_to_default() {
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            sse_config: SseConfig {
                sse_prefix: "/sse".to_string(),
            },
            login_route: "/login".to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
            app_data_dir: None,
        };
        assert_eq!(config.sse_post_route(), "/message");
    }
}
