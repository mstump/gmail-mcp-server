use clap::{Args, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "gmail-mcp-server")]
#[command(about = "Gmail MCP Server - Rust implementation")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub config: Config,
}

#[derive(Args, Debug, Clone)]
pub struct AuthConfig {
    /// Login route path (defaults to /auth/login)
    #[arg(long, env = "LOGIN_ROUTE", default_value = "/auth/login")]
    pub login_route: String,

    /// Token refresh route path (defaults to /auth/refresh)
    #[arg(long, env = "REFRESH_ROUTE", default_value = "/auth/refresh")]
    pub refresh_route: String,

    /// OAuth callback route path (defaults to /auth/callback)
    #[arg(long, env = "CALLBACK_ROUTE", default_value = "/auth/callback")]
    pub callback_route: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_route: "/auth/login".to_string(),
            refresh_route: "/auth/refresh".to_string(),
            callback_route: "/auth/callback".to_string(),
        }
    }
}

#[derive(Parser, Debug, Clone)]
pub enum Commands {
    /// Run the HTTP server
    Http(HttpConfig),
    /// Access tools
    Tools {
        #[command(subcommand)]
        tool: ToolsCmd,
    },
}

#[derive(Args, Debug, Clone)]
pub struct HttpConfig {
    /// HTTP server port
    #[arg(long, env = "PORT", default_value = "8080")]
    pub port: u16,

    /// OAuth redirect URL (defaults to http://localhost:{port}/callback)
    #[arg(long, env = "OAUTH_REDIRECT_URL")]
    pub oauth_redirect_url: Option<String>,

    /// Prometheus metrics route path (defaults to /metrics)
    #[arg(long, env = "METRICS_ROUTE", default_value = "/metrics")]
    pub metrics_route: String,

    /// HTTP stream route path (defaults to /stream)
    #[arg(long, env = "HTTP_STREAM_ROUTE", default_value = "/stream")]
    pub http_stream_route: String,

    /// Tools route path (defaults to /tools)
    #[arg(long, env = "TOOLS_ROUTE", default_value = "/tools")]
    pub tools_route: String,

    /// SSE configuration
    #[command(flatten)]
    pub sse_config: SseConfig,

    /// Auth configuration
    #[command(flatten)]
    pub auth_config: AuthConfig,

    /// Health check route path (defaults to /health)
    #[arg(long, env = "HEALTH_ROUTE", default_value = "/health")]
    pub health_route: String,

    /// Root route path (defaults to /)
    #[arg(long, env = "ROOT_ROUTE", default_value = "/")]
    pub root_route: String,
}

#[derive(Args, Debug, Clone, Default)]
pub struct Config {
    /// Gmail OAuth Client ID
    #[arg(long, env = "GMAIL_CLIENT_ID")]
    pub gmail_client_id: Option<String>,

    /// Gmail OAuth Client Secret
    #[arg(long, env = "GMAIL_CLIENT_SECRET")]
    pub gmail_client_secret: Option<String>,

    /// Application data directory (defaults to platform-specific location)
    #[arg(long, env = "APP_DATA_DIR")]
    pub app_data_dir: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub enum ToolsCmd {
    /// Search Gmail threads
    SearchThreads {
        query: String,
        #[arg(long, default_value = "10")]
        max_results: i64,
    },
    /// Create a Gmail draft
    CreateDraft {
        to: String,
        subject: String,
        body: String,
        #[arg(long)]
        thread_id: Option<String>,
    },
    /// Extract attachment text by filename
    ExtractAttachment {
        message_id: String,
        filename: String,
    },
    /// Fetch email bodies for threads
    FetchEmailBodies { thread_ids: Vec<String> },
    /// Download attachment
    DownloadAttachment {
        message_id: String,
        filename: String,
        #[arg(long)]
        download_dir: Option<String>,
    },
    /// Forward email
    ForwardEmail {
        message_id: String,
        to: String,
        subject: String,
        body: String,
    },
    /// Send draft
    SendDraft { draft_id: String },
}

#[derive(Args, Debug, Clone)]
pub struct SseConfig {
    /// SSE router prefix path (defaults to /sse)
    #[arg(long, env = "SSE_PREFIX", default_value = "/sse")]
    pub sse_prefix: String,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            sse_prefix: "/sse".to_string(),
        }
    }
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

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            http_stream_route: "/stream".to_string(),
            tools_route: "/tools".to_string(),
            sse_config: SseConfig::default(),
            auth_config: AuthConfig::default(),
            health_route: "/health".to_string(),
            root_route: "/".to_string(),
        }
    }
}

impl HttpConfig {
    pub fn oauth_redirect_url(&self) -> String {
        self.oauth_redirect_url.clone().unwrap_or_else(|| {
            format!(
                "http://localhost:{}{}",
                self.port, self.auth_config.callback_route
            )
        })
    }

    pub fn metrics_route(&self) -> &str {
        &self.metrics_route
    }

    pub fn http_stream_route(&self) -> &str {
        &self.http_stream_route
    }

    pub fn tools_route(&self) -> &str {
        &self.tools_route
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
        &self.auth_config.login_route
    }

    #[allow(dead_code)]
    pub fn refresh_route(&self) -> &str {
        &self.auth_config.refresh_route
    }

    pub fn callback_route(&self) -> &str {
        &self.auth_config.callback_route
    }

    pub fn health_route(&self) -> &str {
        &self.health_route
    }

    pub fn root_route(&self) -> &str {
        &self.root_route
    }
}

impl Config {
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
        let http_config = HttpConfig {
            oauth_redirect_url: Some("https://example.com/callback".to_string()),
            ..Default::default()
        };
        assert_eq!(
            http_config.oauth_redirect_url(),
            "https://example.com/callback"
        );
    }

    #[test]
    fn test_oauth_redirect_url_falls_back_to_default() {
        let http_config = HttpConfig {
            port: 3000,
            ..Default::default()
        };
        assert_eq!(
            http_config.oauth_redirect_url(),
            "http://localhost:3000/auth/callback"
        );
    }

    #[test]
    fn test_oauth_redirect_url_default_with_different_port() {
        let http_config = HttpConfig {
            port: 9000,
            ..Default::default()
        };
        assert_eq!(
            http_config.oauth_redirect_url(),
            "http://localhost:9000/auth/callback"
        );
    }

    #[test]
    fn test_metrics_route_uses_configured_value() {
        let http_config = HttpConfig {
            metrics_route: "/custom-metrics".to_string(),
            ..Default::default()
        };
        assert_eq!(http_config.metrics_route(), "/custom-metrics");
    }

    #[test]
    fn test_metrics_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.metrics_route(), "/metrics");
    }

    #[test]
    fn test_login_route_uses_configured_value() {
        let http_config = HttpConfig {
            auth_config: AuthConfig {
                login_route: "/custom-login".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(http_config.login_route(), "/custom-login");
    }

    #[test]
    fn test_login_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.login_route(), "/auth/login");
    }

    #[test]
    fn test_refresh_route_uses_configured_value() {
        let http_config = HttpConfig {
            auth_config: AuthConfig {
                refresh_route: "/custom-refresh".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(http_config.refresh_route(), "/custom-refresh");
    }

    #[test]
    fn test_refresh_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.refresh_route(), "/auth/refresh");
    }

    #[test]
    fn test_callback_route_uses_configured_value() {
        let http_config = HttpConfig {
            auth_config: AuthConfig {
                callback_route: "/custom-callback".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(http_config.callback_route(), "/custom-callback");
    }

    #[test]
    fn test_callback_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.callback_route(), "/auth/callback");
    }

    #[test]
    fn test_app_data_dir_uses_configured_value() {
        let custom_dir = PathBuf::from("/custom/path");
        let config = Config {
            app_data_dir: Some(custom_dir.clone()),
            ..Default::default()
        };
        assert_eq!(config.app_data_dir(), custom_dir);
    }

    #[test]
    fn test_app_data_dir_falls_back_to_default() {
        let config = Config::default();
        let dir = config.app_data_dir();
        // Should end with "gmail-mcp-server-data" or ".gmail-mcp-server-data" depending on platform
        assert!(dir.to_string_lossy().contains("gmail-mcp-server-data"));
    }

    #[test]
    fn test_http_stream_route_uses_configured_value() {
        let http_config = HttpConfig {
            http_stream_route: "/custom-stream".to_string(),
            ..Default::default()
        };
        assert_eq!(http_config.http_stream_route(), "/custom-stream");
    }

    #[test]
    fn test_http_stream_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.http_stream_route(), "/stream");
    }

    #[test]
    fn test_tools_route_uses_configured_value() {
        let http_config = HttpConfig {
            tools_route: "/custom-tools".to_string(),
            ..Default::default()
        };
        assert_eq!(http_config.tools_route(), "/custom-tools");
    }

    #[test]
    fn test_tools_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.tools_route(), "/tools");
    }

    #[test]
    fn test_sse_route_uses_configured_value() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.sse_route(), "/sse");
    }

    #[test]
    fn test_sse_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.sse_route(), "/sse");
    }

    #[test]
    fn test_sse_post_route_uses_configured_value() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.sse_post_route(), "/message");
    }

    #[test]
    fn test_sse_post_route_falls_back_to_default() {
        let http_config = HttpConfig::default();
        assert_eq!(http_config.sse_post_route(), "/message");
    }
}
