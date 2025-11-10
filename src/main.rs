mod config;
mod email;
mod extract;
mod gmail;
mod metrics;
mod oauth;
mod server;
mod tools;
mod utils;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use clap::Parser;
use config::Config;
use dotenv::dotenv;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber with default log level if RUST_LOG is not set
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load environment variables from .env file if it exists
    if dotenv().is_ok() {
        info!("Loaded .env file");
    }

    let config = Config::parse();

    // Validate required environment variables
    if config.gmail_client_id.is_none() {
        return Err(anyhow::anyhow!(
            "GMAIL_CLIENT_ID environment variable not set"
        ));
    }
    if config.gmail_client_secret.is_none() {
        return Err(anyhow::anyhow!(
            "GMAIL_CLIENT_SECRET environment variable not set"
        ));
    }

    let app_data_dir =
        utils::get_app_data_dir(&config).context("Failed to create app data directory")?;
    let token_file =
        utils::get_app_file_path(&config, "token.json").context("Failed to get token file path")?;
    info!("📁 App data directory: {}", app_data_dir.display());
    info!("🔑 Token file: {}", token_file.display());

    // Initialize Gmail server without OAuth (lazy authentication)
    let gmail_server = Arc::new(gmail::GmailServer::new(&config)?);

    info!(
        "Starting Gmail MCP Server in HTTP mode on port {}...",
        config.port
    );
    info!(
        "✅ Server will run persistently at http://localhost:{}",
        config.port
    );
    info!(
        "   Visit http://localhost:{}{} to authenticate",
        config.port,
        config.login_route()
    );
    info!(
        "   MCP endpoint: http://localhost:{}{}",
        config.port,
        config.mcp_route()
    );
    info!(
        "   Metrics endpoint: http://localhost:{}{}",
        config.port,
        config.metrics_route()
    );
    info!("   (Use Ctrl+C to stop the server)");

    // Create OAuth manager
    let oauth_manager = Arc::new(oauth::OAuthManager::new(config.clone())?);

    // Create metrics
    let oauth_metrics = Arc::new(metrics::OAuthMetrics::new()?);

    // Store CSRF tokens temporarily (in production, use Redis or similar)
    let csrf_tokens: Arc<RwLock<std::collections::HashMap<String, String>>> =
        Arc::new(RwLock::new(std::collections::HashMap::new()));

    // Initialize metrics with current token state
    if let Some(token) = oauth_manager.load_token().await? {
        oauth_manager.set_token(token.clone()).await;
        oauth_metrics.update_token_metrics(Some(&token));
    } else {
        oauth_metrics.update_token_metrics(None);
    }

    // Create MCP server
    let mcp_server = server::GmailMcpServer::new(gmail_server.clone());

    // Create StreamableHttpService for MCP
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Build HTTP server with routes
    let metrics_route = config.metrics_route();
    let mcp_route = config.mcp_route();
    let login_route = config.login_route();
    let app = Router::new()
        .route("/", get(root_handler))
        .route(&login_route, get(login_handler))
        .route("/callback", get(callback_handler))
        .route("/health", get(health_handler))
        .route(&metrics_route, get(metrics_handler))
        .nest_service(&mcp_route, mcp_service)
        .with_state(AppState {
            gmail_server: gmail_server.clone(),
            oauth_manager: oauth_manager.clone(),
            csrf_tokens: csrf_tokens.clone(),
            metrics: oauth_metrics.clone(),
            login_route: login_route.to_string(),
            callback_route: "/callback".to_string(),
            health_route: "/health".to_string(),
            metrics_route: metrics_route.to_string(),
            mcp_route: mcp_route.to_string(),
        });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .context("Failed to bind to port")?;

    info!(
        "🌐 HTTP server starting on http://localhost:{}",
        config.port
    );
    info!("📖 View server info: http://localhost:{}", config.port);
    info!("🔍 Health check: http://localhost:{}/health", config.port);
    info!(
        "📊 Metrics endpoint: http://localhost:{}{}",
        config.port,
        config.metrics_route()
    );
    info!(
        "🔌 MCP endpoint: http://localhost:{}{}",
        config.port,
        config.mcp_route()
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    gmail_server: Arc<gmail::GmailServer>,
    oauth_manager: Arc<oauth::OAuthManager>,
    csrf_tokens: Arc<RwLock<std::collections::HashMap<String, String>>>,
    metrics: Arc<metrics::OAuthMetrics>,
    login_route: String,
    callback_route: String,
    health_route: String,
    metrics_route: String,
    mcp_route: String,
}

/// Render a template with placeholder replacements
fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (placeholder, value) in replacements {
        result = result.replace(placeholder, value);
    }
    result
}

async fn root_handler(State(state): State<AppState>) -> Html<String> {
    let template = include_str!("../templates/index.html");
    let html = render_template(
        template,
        &[
            ("{login_route}", &state.login_route),
            ("{callback_route}", &state.callback_route),
            ("{health_route}", &state.health_route),
            ("{metrics_route}", &state.metrics_route),
            ("{mcp_route}", &state.mcp_route),
        ],
    );
    Html(html)
}

async fn login_handler(State(state): State<AppState>) -> Result<Redirect, StatusCode> {
    let (auth_url, csrf_token) = state
        .oauth_manager
        .get_authorization_url()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Store CSRF token
    state
        .csrf_tokens
        .write()
        .await
        .insert(csrf_token.clone(), csrf_token);

    Ok(Redirect::to(auth_url.as_str()))
}

async fn callback_handler(
    State(state): State<AppState>,
    Query(params): Query<CallbackQuery>,
) -> Result<Html<String>, StatusCode> {
    if let Some(error) = params.error {
        let template = include_str!("../templates/error.html");
        let html = render_template(
            template,
            &[
                ("{error_message}", &error),
                ("{login_route}", &state.login_route),
            ],
        );
        return Ok(Html(html));
    }

    let code = params.code.ok_or(StatusCode::BAD_REQUEST)?;

    match state.oauth_manager.exchange_code(&code).await {
        Ok(token) => {
            state.gmail_server.set_authenticated(true).await;
            // Update metrics with the new token
            state.metrics.update_token_metrics(Some(&token));
            let template = include_str!("../templates/success.html");
            Ok(Html(template.to_string()))
        }
        Err(e) => {
            error!("Failed to exchange authorization code: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn metrics_handler(State(state): State<AppState>) -> Result<Response<String>, StatusCode> {
    // Update metrics with current token state
    let token = state.oauth_manager.get_token().await;
    state.metrics.update_token_metrics(token.as_ref());

    match state.metrics.gather() {
        Ok(metrics_text) => {
            let mut response = Response::new(metrics_text);
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("text/plain; version=0.0.4"),
            );
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let template = "Hello {name}, welcome to {place}!";
        let result = render_template(template, &[("{name}", "Alice"), ("{place}", "Wonderland")]);
        assert_eq!(result, "Hello Alice, welcome to Wonderland!");
    }

    #[test]
    fn test_render_template_with_multiple_replacements() {
        let template = "{a} {b} {a}";
        let result = render_template(template, &[("{a}", "foo"), ("{b}", "bar")]);
        assert_eq!(result, "foo bar foo");
    }

    #[test]
    fn test_render_template_no_replacements() {
        let template = "No placeholders here";
        let result = render_template(template, &[]);
        assert_eq!(result, "No placeholders here");
    }

    #[test]
    fn test_render_template_error_page() {
        let template = include_str!("../templates/error.html");
        let result = render_template(
            template,
            &[
                ("{error_message}", "access_denied"),
                ("{login_route}", "/login"),
            ],
        );
        assert!(result.contains("access_denied"));
        assert!(result.contains("/login"));
        assert!(result.contains("Authorization Error"));
    }

    #[test]
    fn test_render_template_success_page() {
        let template = include_str!("../templates/success.html");
        let result = render_template(template, &[]);
        assert!(result.contains("Authorization Successful!"));
        assert!(result.contains("Gmail MCP Server is now configured"));
    }

    #[test]
    fn test_render_template_index_page() {
        let template = include_str!("../templates/index.html");
        let result = render_template(
            template,
            &[
                ("{login_route}", "/login"),
                ("{callback_route}", "/callback"),
                ("{health_route}", "/health"),
                ("{metrics_route}", "/metrics"),
                ("{mcp_route}", "/mcp"),
            ],
        );
        assert!(result.contains("GET /login"));
        assert!(result.contains("GET /callback"));
        assert!(result.contains("GET /health"));
        assert!(result.contains("GET /metrics"));
        assert!(result.contains("POST /mcp"));
        assert!(result.contains("href=\"/login\""));
        assert!(result.contains("<code>/login</code>"));
    }

    #[test]
    fn test_render_template_index_page_with_custom_routes() {
        let template = include_str!("../templates/index.html");
        let result = render_template(
            template,
            &[
                ("{login_route}", "/auth/login"),
                ("{callback_route}", "/auth/callback"),
                ("{health_route}", "/status/health"),
                ("{metrics_route}", "/prometheus/metrics"),
                ("{mcp_route}", "/api/mcp"),
            ],
        );
        assert!(result.contains("GET /auth/login"));
        assert!(result.contains("GET /auth/callback"));
        assert!(result.contains("GET /status/health"));
        assert!(result.contains("GET /prometheus/metrics"));
        assert!(result.contains("POST /api/mcp"));
        assert!(result.contains("href=\"/auth/login\""));
        assert!(result.contains("<code>/auth/login</code>"));
    }
}
