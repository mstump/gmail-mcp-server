use prometheus::{Encoder, Gauge, Registry, TextEncoder};

/// Prometheus metrics for OAuth token status
pub struct OAuthMetrics {
    /// Gauge indicating whether an OAuth token exists (1 if exists, 0 if not)
    pub token_exists: Gauge,
    /// Gauge for the expires_in value (seconds until expiration)
    pub expires_in: Gauge,
    /// Gauge for the expires_at value (Unix timestamp)
    pub expires_at: Gauge,
    registry: Registry,
}

impl OAuthMetrics {
    /// Create a new OAuthMetrics instance with registered metrics
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let token_exists = Gauge::with_opts(
            prometheus::Opts::new(
                "gmail_oauth_token_exists",
                "Whether an OAuth token exists (1 if exists, 0 if not)",
            ),
        )?;

        let expires_in = Gauge::with_opts(
            prometheus::Opts::new(
                "gmail_oauth_token_expires_in",
                "Seconds until the OAuth token expires",
            ),
        )?;

        let expires_at = Gauge::with_opts(
            prometheus::Opts::new(
                "gmail_oauth_token_expires_at",
                "Unix timestamp when the OAuth token expires",
            ),
        )?;

        registry.register(Box::new(token_exists.clone()))?;
        registry.register(Box::new(expires_in.clone()))?;
        registry.register(Box::new(expires_at.clone()))?;

        Ok(Self {
            token_exists,
            expires_in,
            expires_at,
            registry,
        })
    }

    /// Update metrics based on token state
    pub fn update_token_metrics(&self, token: Option<&crate::oauth::Token>) {
        if let Some(token) = token {
            self.token_exists.set(1.0);
            if let Some(expires_in) = token.expires_in {
                self.expires_in.set(expires_in as f64);
            } else {
                self.expires_in.set(-1.0); // Use -1 to indicate "not set"
            }
            if let Some(expires_at) = token.expires_at {
                self.expires_at.set(expires_at as f64);
            } else {
                self.expires_at.set(-1.0); // Use -1 to indicate "not set"
            }
        } else {
            self.token_exists.set(0.0);
            self.expires_in.set(-1.0);
            self.expires_at.set(-1.0);
        }
    }

    /// Get the Prometheus metrics in text format
    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}

impl Default for OAuthMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create OAuthMetrics")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::Token;

    #[test]
    fn test_oauth_metrics_creation() {
        let metrics = OAuthMetrics::new();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_update_token_metrics_with_token() {
        let metrics = OAuthMetrics::new().unwrap();
        let token = Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            expires_at: Some(1234567890),
        };

        metrics.update_token_metrics(Some(&token));

        assert_eq!(metrics.token_exists.get(), 1.0);
        assert_eq!(metrics.expires_in.get(), 3600.0);
        assert_eq!(metrics.expires_at.get(), 1234567890.0);
    }

    #[test]
    fn test_update_token_metrics_without_token() {
        let metrics = OAuthMetrics::new().unwrap();
        metrics.update_token_metrics(None);

        assert_eq!(metrics.token_exists.get(), 0.0);
        assert_eq!(metrics.expires_in.get(), -1.0);
        assert_eq!(metrics.expires_at.get(), -1.0);
    }

    #[test]
    fn test_update_token_metrics_with_partial_token() {
        let metrics = OAuthMetrics::new().unwrap();
        let token = Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            expires_at: None,
        };

        metrics.update_token_metrics(Some(&token));

        assert_eq!(metrics.token_exists.get(), 1.0);
        assert_eq!(metrics.expires_in.get(), 3600.0);
        assert_eq!(metrics.expires_at.get(), -1.0);
    }

    #[test]
    fn test_gather_metrics() {
        let metrics = OAuthMetrics::new().unwrap();
        let token = Token {
            access_token: "test_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            expires_at: Some(1234567890),
        };

        metrics.update_token_metrics(Some(&token));
        let output = metrics.gather().unwrap();

        assert!(output.contains("gmail_oauth_token_exists"));
        assert!(output.contains("gmail_oauth_token_expires_in"));
        assert!(output.contains("gmail_oauth_token_expires_at"));
    }
}

