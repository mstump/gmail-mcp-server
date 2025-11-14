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
    Json, Router,
};
use axum_prometheus::PrometheusMetricLayer;

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use bytes::Bytes;
use clap::Parser;
use config::{Cli, Commands, Config, HttpConfig, ToolsCmd};
use dotenv::dotenv;
use http_body_util::BodyExt;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use rmcp::transport::{sse_server::SseServerConfig, SseServer};
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, trace, Level};

use crate::server::{
    CreateDraftArgs, DownloadAttachmentArgs, ExtractAttachmentArgs, FetchEmailBodiesArgs,
    ForwardEmailArgs, SearchThreadsArgs, SendDraftArgs,
};

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

/// Middleware to log request bodies at trace level
async fn log_request_body(request: Request, next: Next) -> axum::response::Response {
    let (parts, body) = request.into_parts();

    // Try to collect the body for logging (limit to 1MB to avoid memory issues)
    let body_bytes = match body.collect().await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            // Only log if body is reasonable size (1MB limit)
            if bytes.len() <= 1_048_576 {
                trace!("request body: {:?}", String::from_utf8_lossy(&bytes));
            } else {
                trace!("request body: <too large to log ({} bytes)>", bytes.len());
            }
            bytes
        }
        Err(e) => {
            trace!("failed to read request body: {}", e);
            Bytes::new()
        }
    };

    // Reconstruct the request with the buffered body
    let body = Body::from(body_bytes);
    let request = Request::from_parts(parts, body);

    next.run(request).await
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

    let cli = Cli::parse();
    let config = cli.config;

    match cli.command {
        Commands::Http(http_config) => run_http_server(config, http_config).await,
        Commands::Tools { tool } => run_tools(config, tool).await,
    }
}

async fn run_tools(config: Config, tool: ToolsCmd) -> Result<()> {
    let gmail_server = Arc::new(gmail::GmailServer::new(&config)?);
    let result = match tool {
        ToolsCmd::SearchThreads { query, max_results } => {
            tools::search_threads(&gmail_server, &query, max_results).await
        }
        ToolsCmd::CreateDraft {
            to,
            subject,
            body,
            thread_id,
        } => tools::create_draft(&gmail_server, &to, &subject, &body, thread_id.as_deref()).await,
        ToolsCmd::ExtractAttachment {
            message_id,
            filename,
        } => tools::extract_attachment_by_filename(&gmail_server, &message_id, &filename).await,
        ToolsCmd::FetchEmailBodies { thread_ids } => {
            tools::fetch_email_bodies(&gmail_server, &thread_ids).await
        }
        ToolsCmd::DownloadAttachment {
            message_id,
            filename,
            download_dir,
        } => {
            tools::download_attachment(
                &gmail_server,
                &message_id,
                &filename,
                download_dir.as_deref(),
            )
            .await
        }
        ToolsCmd::ForwardEmail {
            message_id,
            to,
            subject,
            body,
        } => tools::forward_email(&gmail_server, &message_id, &to, &subject, &body).await,
        ToolsCmd::SendDraft { draft_id } => tools::send_draft(&gmail_server, &draft_id).await,
    }?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

async fn run_http_server(config: Config, http_config: HttpConfig) -> Result<()> {
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
        http_config.port
    );
    info!(
        "✅ Server will run persistently at http://localhost:{}",
        http_config.port
    );
    info!(
        "   Visit http://localhost:{}{} to authenticate",
        http_config.port,
        http_config.login_route()
    );
    info!(
        "   HTTP stream endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.http_stream_route()
    );
    info!(
        "   SSE endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.sse_route()
    );
    info!(
        "   POST endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.sse_post_route()
    );
    info!(
        "   Tools endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.tools_route()
    );
    info!(
        "   Metrics endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.metrics_route()
    );
    info!("   (Use Ctrl+C to stop the server)");

    // Create OAuth manager
    let oauth_manager = Arc::new(oauth::OAuthManager::new(
        config.clone(),
        http_config.clone(),
    )?);

    // Initialize Prometheus metrics recorder (axum-prometheus uses metrics-exporter-prometheus
    // which installs a global recorder that all metrics will use)
    let (metric_layer, metric_handle) = PrometheusMetricLayer::pair();

    // Create OAuth metrics - they will automatically use the global recorder installed by axum-prometheus
    let oauth_metrics = Arc::new(metrics::OAuthMetrics::new());

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

    // Create StreamableHttpService for HTTP streaming
    let http_stream_route = http_config.http_stream_route();
    let mcp_server_for_http = mcp_server.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp_server_for_http.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Set up SSE server configuration
    let addr: SocketAddr = format!("0.0.0.0:{}", http_config.port)
        .parse()
        .context("Failed to parse bind address")?;
    let ct = CancellationToken::new();
    // SSE routes are fixed: /sse for SSE endpoint, /message for POST endpoint
    // These are relative paths within the SSE router (nested under sse_prefix)
    // Final routes will be: {sse_prefix}/sse and {sse_prefix}/message
    let sse_relative_path = http_config.sse_route().to_string(); // Fixed to "/sse"
    let post_relative_path = http_config.sse_post_route().to_string(); // Fixed to "/message"
    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: sse_relative_path.to_string(),
        post_path: post_relative_path.to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Create SSE server
    let (sse_server, sse_router) = SseServer::new(sse_config);

    // Start SSE server with MCP service
    sse_server.with_service(move || mcp_server.clone());

    // Build HTTP server with routes
    let root_route = http_config.root_route();
    let metrics_route = http_config.metrics_route();
    let login_route = http_config.login_route();
    let callback_route = http_config.callback_route();
    let health_route = http_config.health_route();
    let tools_route = http_config.tools_route();
    let app_state = AppState {
        gmail_server: gmail_server.clone(),
        oauth_manager: oauth_manager.clone(),
        csrf_tokens: csrf_tokens.clone(),
        metrics: oauth_metrics.clone(),
        prometheus_handle: metric_handle.clone(),
        http_config: http_config.clone(),
    };
    // Build HTTP server with routes
    // SSE router has its own routes configured via SseServerConfig
    // Nest the SSE router under the configured prefix to avoid route conflicts
    let sse_prefix = http_config.sse_prefix();

    // Configure tracing middleware to log request headers and bodies at debug/trace level
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            tracing::span!(
                Level::DEBUG,
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
            )
        })
        .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
            // Log all request headers at debug level
            debug!("request headers: {:?}", request.headers());

            // Log body metadata at trace level
            if let Some(content_type) = request.headers().get(header::CONTENT_TYPE) {
                trace!("request content-type: {:?}", content_type);
            }
            if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
                trace!("request content-length: {:?}", content_length);
            }
        })
        .on_response(
            |response: &axum::http::Response<_>,
             latency: std::time::Duration,
             _span: &tracing::Span| {
                debug!(
                    "response status: {}, latency: {:?}",
                    response.status(),
                    latency
                );
                trace!("response headers: {:?}", response.headers());
            },
        );

    let app = Router::new()
        .route(root_route, get(root_handler))
        .route(login_route, get(login_handler))
        .route(callback_route, get(callback_handler))
        .route(health_route, get(health_handler))
        .route(metrics_route, get(metrics_handler))
        .nest(tools_route, tools_router())
        .nest_service(sse_prefix, sse_router)
        .nest_service(http_stream_route, mcp_service)
        .layer(axum::middleware::from_fn(log_request_body))
        .layer(ServiceBuilder::new().layer(trace_layer))
        .layer(metric_layer)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", http_config.port))
        .await
        .context("Failed to bind to port")?;

    info!(
        "🌐 HTTP server starting on http://localhost:{}",
        http_config.port
    );
    info!(
        "📖 View server info: http://localhost:{}{}",
        http_config.port,
        http_config.root_route()
    );
    info!(
        "🔍 Health check: http://localhost:{}{}",
        http_config.port,
        http_config.health_route()
    );
    info!(
        "📊 Metrics endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.metrics_route()
    );
    info!(
        "🔌 HTTP stream endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.http_stream_route()
    );
    info!(
        "🔌 SSE endpoint: http://localhost:{}{}{}",
        http_config.port,
        http_config.sse_prefix(),
        http_config.sse_route()
    );
    info!(
        "📨 SSE POST endpoint: http://localhost:{}{}{}",
        http_config.port,
        http_config.sse_prefix(),
        http_config.sse_post_route()
    );
    info!(
        "🛠️ Tools endpoint: http://localhost:{}{}",
        http_config.port,
        http_config.tools_route()
    );

    // Handle signals for graceful shutdown
    let cancel_token = ct.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Received Ctrl+C, shutting down server...");
        cancel_token.cancel();
    });

    // Replace axum::serve with a custom implementation that awaits shutdown
    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(async move {
        ct.cancelled().await;
        info!("Server is shutting down...");
    });

    if let Err(e) = graceful.await {
        error!("Server error: {}", e);
    }

    Ok(())
}

#[derive(Clone)]
struct AppState {
    gmail_server: Arc<gmail::GmailServer>,
    oauth_manager: Arc<oauth::OAuthManager>,
    csrf_tokens: Arc<RwLock<std::collections::HashMap<String, String>>>,
    metrics: Arc<metrics::OAuthMetrics>,
    prometheus_handle: axum_prometheus::metrics_exporter_prometheus::PrometheusHandle,
    http_config: HttpConfig,
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
    let sse_route_full = format!(
        "{}{}",
        state.http_config.sse_prefix(),
        state.http_config.sse_route()
    );
    let sse_post_route_full = format!(
        "{}{}",
        state.http_config.sse_prefix(),
        state.http_config.sse_post_route()
    );
    let html = render_template(
        template,
        &[
            ("{root_route}", state.http_config.root_route()),
            ("{login_route}", state.http_config.login_route()),
            ("{callback_route}", state.http_config.callback_route()),
            ("{health_route}", state.http_config.health_route()),
            ("{metrics_route}", state.http_config.metrics_route()),
            ("{http_stream_route}", state.http_config.http_stream_route()),
            ("{tools_route}", state.http_config.tools_route()),
            ("{sse_route}", &sse_route_full),
            ("{sse_post_route}", &sse_post_route_full),
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
                ("{login_route}", state.http_config.login_route()),
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

    // Render all metrics from the global recorder
    // Since OAuth metrics now use the metrics crate, they are automatically included
    // in the global recorder used by axum-prometheus, so we just need to render once
    let metrics_output = state.prometheus_handle.render();

    let mut response = Response::new(metrics_output);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    Ok(response)
}

fn tools_router() -> Router<AppState> {
    Router::new()
        .route("/search_threads", get(search_threads_handler))
        .route("/create_draft", get(create_draft_handler))
        .route(
            "/extract_attachment_by_filename",
            get(extract_attachment_by_filename_handler),
        )
        .route("/fetch_email_bodies", get(fetch_email_bodies_handler))
        .route("/download_attachment", get(download_attachment_handler))
        .route("/forward_email", get(forward_email_handler))
        .route("/send_draft", get(send_draft_handler))
}

async fn search_threads_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchThreadsArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::search_threads(
        &state.gmail_server,
        &params.query,
        params.max_results.unwrap_or(10),
    )
    .await
    .map(Json)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_draft_handler(
    State(state): State<AppState>,
    Query(params): Query<CreateDraftArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::create_draft(
        &state.gmail_server,
        &params.to,
        &params.subject,
        &params.body,
        params.thread_id.as_deref(),
    )
    .await
    .map(Json)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn extract_attachment_by_filename_handler(
    State(state): State<AppState>,
    Query(params): Query<ExtractAttachmentArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::extract_attachment_by_filename(
        &state.gmail_server,
        &params.message_id,
        &params.filename,
    )
    .await
    .map(Json)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn fetch_email_bodies_handler(
    State(state): State<AppState>,
    Query(params): Query<FetchEmailBodiesArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::fetch_email_bodies(&state.gmail_server, &params.thread_ids)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn download_attachment_handler(
    State(state): State<AppState>,
    Query(params): Query<DownloadAttachmentArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::download_attachment(
        &state.gmail_server,
        &params.message_id,
        &params.filename,
        params.download_dir.as_deref(),
    )
    .await
    .map(Json)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn forward_email_handler(
    State(state): State<AppState>,
    Query(params): Query<ForwardEmailArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::forward_email(
        &state.gmail_server,
        &params.message_id,
        &params.to,
        &params.subject,
        &params.body,
    )
    .await
    .map(Json)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn send_draft_handler(
    State(state): State<AppState>,
    Query(params): Query<SendDraftArgs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    tools::send_draft(&state.gmail_server, &params.draft_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
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
                ("{root_route}", "/"),
                ("{login_route}", "/login"),
                ("{callback_route}", "/callback"),
                ("{health_route}", "/health"),
                ("{metrics_route}", "/metrics"),
                ("{http_stream_route}", "/stream"),
                ("{tools_route}", "/tools"),
                ("{sse_route}", "/sse"),
                ("{sse_post_route}", "/message"),
            ],
        );
        assert!(result.contains("GET /login"));
        assert!(result.contains("GET /callback"));
        assert!(result.contains("GET /health"));
        assert!(result.contains("GET /metrics"));
        assert!(result.contains("POST /stream"));
        assert!(result.contains("GET /sse"));
        assert!(result.contains("POST /message"));
        assert!(result.contains("href=\"/login\""));
        assert!(result.contains("<code>/login</code>"));
    }

    #[test]
    fn test_render_template_index_page_with_custom_routes() {
        let template = include_str!("../templates/index.html");
        let result = render_template(
            template,
            &[
                ("{root_route}", "/api"),
                ("{login_route}", "/auth/login"),
                ("{callback_route}", "/auth/callback"),
                ("{health_route}", "/status/health"),
                ("{metrics_route}", "/prometheus/metrics"),
                ("{http_stream_route}", "/api/stream"),
                ("{tools_route}", "/api/tools"),
                ("{sse_route}", "/api/sse"),
                ("{sse_post_route}", "/api/message"),
            ],
        );
        assert!(result.contains("GET /auth/login"));
        assert!(result.contains("GET /auth/callback"));
        assert!(result.contains("GET /status/health"));
        assert!(result.contains("GET /prometheus/metrics"));
        assert!(result.contains("POST /api/stream"));
        assert!(result.contains("GET /api/sse"));
        assert!(result.contains("POST /api/message"));
        assert!(result.contains("href=\"/auth/login\""));
        assert!(result.contains("<code>/auth/login</code>"));
    }

    #[test]
    fn test_app_state_uses_config_for_routes() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = Config {
            gmail_client_id: Some("test-client-id".to_string()),
            gmail_client_secret: Some("test-client-secret".to_string()),
            ..Default::default()
        };
        let http_config = HttpConfig {
            metrics_route: "/custom-metrics".to_string(),
            http_stream_route: "/custom-stream".to_string(),
            sse_config: config::SseConfig {
                sse_prefix: "/custom-sse".to_string(),
            },
            login_route: "/custom-login".to_string(),
            callback_route: "/custom-callback".to_string(),
            health_route: "/custom-health".to_string(),
            root_route: "/custom-root".to_string(),
            tools_route: "/custom-tools".to_string(),
            ..Default::default()
        };

        // Create a minimal AppState with Config
        // Note: We can't use PrometheusMetricLayer::pair() in tests without a Tokio runtime,
        // so we create a handle directly for testing purposes
        use axum_prometheus::metrics_exporter_prometheus::PrometheusBuilder;
        let prometheus_handle = PrometheusBuilder::new()
            .install_recorder()
            .expect("Failed to install Prometheus recorder");
        let app_state = AppState {
            gmail_server: Arc::new(gmail::GmailServer::new(&config).unwrap()),
            oauth_manager: Arc::new(
                oauth::OAuthManager::new(config.clone(), http_config.clone()).unwrap(),
            ),
            csrf_tokens: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(metrics::OAuthMetrics::new()),
            prometheus_handle,
            http_config: http_config.clone(),
        };

        // Verify routes are accessible through config
        assert_eq!(app_state.http_config.root_route(), "/custom-root");
        assert_eq!(app_state.http_config.login_route(), "/custom-login");
        assert_eq!(app_state.http_config.callback_route(), "/custom-callback");
        assert_eq!(app_state.http_config.health_route(), "/custom-health");
        assert_eq!(app_state.http_config.metrics_route(), "/custom-metrics");
        assert_eq!(app_state.http_config.http_stream_route(), "/custom-stream");
        assert_eq!(app_state.http_config.sse_route(), "/sse");
        assert_eq!(app_state.http_config.sse_post_route(), "/message");
        assert_eq!(app_state.http_config.tools_route(), "/custom-tools");
    }
}
