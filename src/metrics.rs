use crate::oauth::OAuthToken;
use metrics::gauge;
use std::sync::atomic::{AtomicU64, Ordering};

const GAUGE_TOKEN_LAST_REFRESHED_TIMESTAMP: &str = "gmail_mcp_token_last_refreshed_timestamp";

/// Prometheus metrics for OAuth token status
pub struct OAuthMetrics {
    token_last_refreshed_timestamp: AtomicU64,
}

impl OAuthMetrics {
    pub fn new() -> Self {
        gauge!(GAUGE_TOKEN_LAST_REFRESHED_TIMESTAMP).set(0.0);
        Self {
            token_last_refreshed_timestamp: AtomicU64::new(0),
        }
    }

    /// Update metrics with the current token state
    pub fn update_token_metrics(&self, token: Option<&OAuthToken>) {
        if let Some(token) = token {
            self.token_last_refreshed_timestamp
                .store(token.created_at, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_initial_metrics_state() {
        let metrics = OAuthMetrics::new();
        // Initially, the gauges should be set to 0
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_update_token_metrics_with_valid_token() {
        let metrics = OAuthMetrics::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token = OAuthToken {
            access_token: "test_access_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("test_refresh_token".to_string()),
            scope: "test_scope".to_string(),
            created_at: now,
        };

        metrics.update_token_metrics(Some(&token));
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            now
        );
    }

    #[test]
    fn test_update_token_metrics_with_none() {
        let metrics = OAuthMetrics::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        metrics
            .token_last_refreshed_timestamp
            .store(now, Ordering::Relaxed);
        metrics.update_token_metrics(None);
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            now
        );
    }

    #[test]
    fn test_update_token_metrics_with_expired_token() {
        let metrics = OAuthMetrics::new();
        let past_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 4000;
        let token = OAuthToken {
            access_token: "test_access_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("test_refresh_token".to_string()),
            scope: "test_scope".to_string(),
            created_at: past_time,
        };

        metrics.update_token_metrics(Some(&token));
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            past_time
        );
    }

    #[test]
    fn test_update_token_metrics_updates_timestamp() {
        let metrics = OAuthMetrics::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token1 = OAuthToken {
            access_token: "test_access_token1".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("test_refresh_token".to_string()),
            scope: "test_scope".to_string(),
            created_at: now,
        };

        metrics.update_token_metrics(Some(&token1));
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            now
        );

        let new_time = now + 1000;
        let token2 = OAuthToken {
            access_token: "test_access_token2".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("test_refresh_token".to_string()),
            scope: "test_scope".to_string(),
            created_at: new_time,
        };
        metrics.update_token_metrics(Some(&token2));
        assert_eq!(
            metrics
                .token_last_refreshed_timestamp
                .load(Ordering::Relaxed),
            new_time
        );
    }
}
