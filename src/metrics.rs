use metrics::gauge;

/// Prometheus metrics for OAuth token status
/// Uses the metrics crate which routes to the global recorder (used by axum-prometheus)
pub struct OAuthMetrics {
    // Note: We don't store Gauge handles since metrics crate uses global registration
    // The metrics are registered with the global recorder and can be accessed via the metrics crate
}

impl OAuthMetrics {
    /// Create a new OAuthMetrics instance
    /// Metrics are registered with the global metrics recorder, which will be picked up
    /// by the Prometheus exporter (metrics-exporter-prometheus) used by axum-prometheus
    pub fn new() -> Self {
        // Describe the metrics for better documentation in Prometheus
        metrics::describe_gauge!(
            "gmail_oauth_token_exists",
            metrics::Unit::Count,
            "Whether an OAuth token exists (1 if exists, 0 if not)"
        );
        metrics::describe_gauge!(
            "gmail_oauth_token_expires_in",
            metrics::Unit::Seconds,
            "Seconds until the OAuth token expires"
        );
        metrics::describe_gauge!(
            "gmail_oauth_token_expires_at",
            metrics::Unit::Count,
            "Unix timestamp when the OAuth token expires"
        );

        Self {}
    }

    /// Update metrics based on token state
    /// Metrics are updated in the global recorder and will be included in Prometheus output
    pub fn update_token_metrics(&self, token: Option<&crate::oauth::Token>) {
        if let Some(token) = token {
            gauge!("gmail_oauth_token_exists").set(1.0);
            if let Some(expires_in) = token.expires_in {
                gauge!("gmail_oauth_token_expires_in").set(expires_in as f64);
            } else {
                gauge!("gmail_oauth_token_expires_in").set(-1.0); // Use -1 to indicate "not set"
            }
            if let Some(expires_at) = token.expires_at {
                gauge!("gmail_oauth_token_expires_at").set(expires_at as f64);
            } else {
                gauge!("gmail_oauth_token_expires_at").set(-1.0); // Use -1 to indicate "not set"
            }
        } else {
            gauge!("gmail_oauth_token_exists").set(0.0);
            gauge!("gmail_oauth_token_expires_in").set(-1.0);
            gauge!("gmail_oauth_token_expires_at").set(-1.0);
        }
    }
}

impl Default for OAuthMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::Token;

    #[test]
    fn test_oauth_metrics_creation() {
        // Just verify OAuthMetrics can be created
        // The metrics will be registered with the global recorder when it's installed
        let metrics = OAuthMetrics::new();
        drop(metrics);
    }

    #[test]
    fn test_update_token_metrics_with_token() {
        // Initialize a test recorder if one doesn't exist
        use metrics_exporter_prometheus::PrometheusBuilder;
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .ok(); // It's ok if recorder is already installed

        let metrics = OAuthMetrics::new();
        let token = Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            expires_at: Some(1234567890),
        };

        // Verify the update doesn't panic
        metrics.update_token_metrics(Some(&token));

        // If we have a handle, verify metrics are in the output
        if let Some(handle) = handle {
            let output = handle.render();
            assert!(output.contains("gmail_oauth_token_exists"));
            assert!(output.contains("gmail_oauth_token_expires_in"));
            assert!(output.contains("gmail_oauth_token_expires_at"));
        }
    }

    #[test]
    fn test_update_token_metrics_without_token() {
        // Initialize a test recorder if one doesn't exist
        use metrics_exporter_prometheus::PrometheusBuilder;
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .ok(); // It's ok if recorder is already installed

        let metrics = OAuthMetrics::new();
        // Verify the update doesn't panic
        metrics.update_token_metrics(None);

        // If we have a handle, verify metrics are in the output
        if let Some(handle) = handle {
            let output = handle.render();
            assert!(output.contains("gmail_oauth_token_exists"));
            assert!(output.contains("gmail_oauth_token_expires_in"));
            assert!(output.contains("gmail_oauth_token_expires_at"));
        }
    }

    #[test]
    fn test_update_token_metrics_with_partial_token() {
        // Initialize a test recorder if one doesn't exist
        use metrics_exporter_prometheus::PrometheusBuilder;
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .ok(); // It's ok if recorder is already installed

        let metrics = OAuthMetrics::new();
        let token = Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            expires_at: None,
        };

        // Verify the update doesn't panic
        metrics.update_token_metrics(Some(&token));

        // If we have a handle, verify metrics are in the output
        if let Some(handle) = handle {
            let output = handle.render();
            assert!(output.contains("gmail_oauth_token_exists"));
            assert!(output.contains("gmail_oauth_token_expires_in"));
            assert!(output.contains("gmail_oauth_token_expires_at"));
        }
    }
}
