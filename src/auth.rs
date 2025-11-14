use crate::AppState;

use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use tracing::error;

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_handler))
        .route("/callback", get(callback_handler))
        .route("/refresh", get(refresh_handler))
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
        let html = crate::render_template(
            template,
            &[
                ("{error_message}", &error),
                (
                    "{login_route}",
                    state.http_config.auth_config.login_route.as_str(),
                ),
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

async fn refresh_handler(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match state.oauth_manager.refresh_token().await {
        Ok(token) => {
            state.metrics.update_token_metrics(Some(&token));
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Token refreshed successfully",
                "access_token": token.access_token,
                "expires_in": token.expires_in,
            })))
        }
        Err(e) => {
            error!("Failed to refresh token: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to refresh token: {}", e),
                })),
            ))
        }
    }
}
