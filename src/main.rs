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
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
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
use tracing::{info, error};

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
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Load environment variables from .env file if it exists
    if dotenv().is_ok() {
        info!("Loaded .env file");
    }

    let config = Config::parse();

    // Validate required environment variables
    if config.gmail_client_id.is_none() {
        return Err(anyhow::anyhow!("GMAIL_CLIENT_ID environment variable not set"));
    }
    if config.gmail_client_secret.is_none() {
        return Err(anyhow::anyhow!("GMAIL_CLIENT_SECRET environment variable not set"));
    }

    let app_data_dir = utils::get_app_data_dir(&config)
        .context("Failed to create app data directory")?;
    let token_file = utils::get_app_file_path(&config, "token.json")
        .context("Failed to get token file path")?;
    info!("📁 App data directory: {}", app_data_dir.display());
    info!("🔑 Token file: {}", token_file.display());

    // Initialize Gmail server without OAuth (lazy authentication)
    let gmail_server = Arc::new(gmail::GmailServer::new(&config)?);

    info!("Starting Gmail MCP Server in HTTP mode on port {}...", config.port);
    info!("✅ Server will run persistently at http://localhost:{}", config.port);
    info!("   Visit http://localhost:{}{} to authenticate", config.port, config.login_route());
    info!("   MCP endpoint: http://localhost:{}{}", config.port, config.mcp_route());
    info!("   Metrics endpoint: http://localhost:{}{}", config.port, config.metrics_route());
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
        });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .context("Failed to bind to port")?;

    info!("🌐 HTTP server starting on http://localhost:{}", config.port);
    info!("📖 View server info: http://localhost:{}", config.port);
    info!("🔍 Health check: http://localhost:{}/health", config.port);
    info!("📊 Metrics endpoint: http://localhost:{}{}", config.port, config.metrics_route());
    info!("🔌 MCP endpoint: http://localhost:{}{}", config.port, config.mcp_route());

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
}

async fn root_handler() -> Html<&'static str> {
    Html(include_str!("../templates/index.html"))
}

async fn login_handler(State(state): State<AppState>) -> Result<Redirect, StatusCode> {
    let (auth_url, csrf_token) = state.oauth_manager.get_authorization_url()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Store CSRF token
    state.csrf_tokens.write().await.insert(csrf_token.clone(), csrf_token);

    Ok(Redirect::to(auth_url.as_str()))
}

async fn callback_handler(
    State(state): State<AppState>,
    Query(params): Query<CallbackQuery>,
) -> Result<Html<String>, StatusCode> {
    if let Some(error) = params.error {
        return Ok(Html(format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Gmail MCP Server - Authorization Error</title>
    <style>
        body {{ font-family: Arial, sans-serif; text-align: center; margin-top: 50px; }}
        .error {{ color: red; font-size: 18px; }}
    </style>
</head>
<body>
    <h1>Authorization Error</h1>
    <p class="error">❌ {}</p>
    <p>Please try again by visiting <a href="{}">{}</a></p>
</body>
</html>"#,
            error,
            state.login_route,
            state.login_route
        )));
    }

    let code = params.code.ok_or(StatusCode::BAD_REQUEST)?;

    match state.oauth_manager.exchange_code(&code).await {
        Ok(token) => {
            state.gmail_server.set_authenticated(true).await;
            // Update metrics with the new token
            state.metrics.update_token_metrics(Some(&token));
            Ok(Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Gmail MCP Server - Authorization Complete</title>
    <style>
        body { font-family: Arial, sans-serif; text-align: center; margin-top: 50px; }
        .success { color: green; font-size: 18px; }
    </style>
</head>
<body>
    <h1>Authorization Successful!</h1>
    <p class="success">✅ You can now close this browser window and return to your terminal.</p>
    <p>Your Gmail MCP Server is now configured.</p>
</body>
</html>"#.to_string()))
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

