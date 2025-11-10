use std::path::PathBuf;
use anyhow::{Context, Result};

use crate::config::Config;

/// Get the application data directory from Config and ensure it exists
/// Returns an error if the directory cannot be created
pub fn get_app_data_dir(config: &Config) -> Result<PathBuf> {
    let app_data_dir = config.app_data_dir();

    // Ensure the directory exists
    std::fs::create_dir_all(&app_data_dir)
        .with_context(|| format!("Could not create app data directory at {}", app_data_dir.display()))?;

    Ok(app_data_dir)
}

/// Get an absolute path in the app data directory
pub fn get_app_file_path(config: &Config, filename: &str) -> Result<PathBuf> {
    Ok(get_app_data_dir(config)?.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_app_data_dir_uses_config_value() {
        let temp_dir = TempDir::new().unwrap();
        let custom_dir = temp_dir.path().to_path_buf();

        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
            app_data_dir: Some(custom_dir.clone()),
        };

        let result = get_app_data_dir(&config).unwrap();
        assert_eq!(result, custom_dir);
        assert!(result.exists());
    }

    #[test]
    fn test_get_app_data_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new-dir");

        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
            app_data_dir: Some(new_dir.clone()),
        };

        assert!(!new_dir.exists());
        let result = get_app_data_dir(&config).unwrap();
        assert!(result.exists());
        assert_eq!(result, new_dir);
    }

    #[test]
    fn test_get_app_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let custom_dir = temp_dir.path().to_path_buf();

        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
            app_data_dir: Some(custom_dir.clone()),
        };

        let result = get_app_file_path(&config, "token.json").unwrap();
        assert_eq!(result, custom_dir.join("token.json"));
    }

    #[test]
    fn test_get_app_data_dir_returns_error_on_failure() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        // Create a file with the same name as the directory we'll try to create
        let file_path = temp_dir.path().join("conflicting-name");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test").unwrap();
        drop(file);

        // Now try to create a directory with the same name - this should fail
        let config = Config {
            port: 8080,
            gmail_client_id: None,
            gmail_client_secret: None,
            oauth_redirect_url: None,
            metrics_route: "/metrics".to_string(),
            mcp_route: "/mcp".to_string(),
            login_route: "/login".to_string(),
            app_data_dir: Some(file_path),
        };

        let result = get_app_data_dir(&config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Could not create app data directory"));
    }
}
