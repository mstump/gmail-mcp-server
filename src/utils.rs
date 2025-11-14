use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn get_app_data_dir(config: &Config) -> Result<PathBuf> {
    let app_data_dir = config.app_data_dir();
    fs::create_dir_all(&app_data_dir)
        .with_context(|| format!("Failed to create directory at {}", app_data_dir.display()))?;
    Ok(app_data_dir)
}

pub fn get_app_file_path(config: &Config, filename: &str) -> Result<PathBuf> {
    let app_data_dir = get_app_data_dir(config)?;
    Ok(app_data_dir.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_config(app_data_dir: Option<PathBuf>) -> Config {
        Config {
            gmail_client_id: None,
            gmail_client_secret: None,
            app_data_dir,
        }
    }

    #[test]
    fn test_get_app_data_dir_with_custom_path() {
        let dir = tempdir().unwrap();
        let custom_path = dir.path().join("custom_app_data");
        let config = create_test_config(Some(custom_path.clone()));
        let result = get_app_data_dir(&config).unwrap();
        assert_eq!(result, custom_path);
        assert!(custom_path.exists());
    }

    #[test]
    fn test_get_app_data_dir_with_default_path() {
        let config = create_test_config(None);
        let result = get_app_data_dir(&config).unwrap();
        assert!(result.to_string_lossy().contains("gmail-mcp-server-data"));
        assert!(result.exists());
        fs::remove_dir_all(result).unwrap();
    }

    #[test]
    fn test_get_app_file_path() {
        let dir = tempdir().unwrap();
        let custom_path = dir.path().join("test_app_data");
        let config = create_test_config(Some(custom_path.clone()));
        let result = get_app_file_path(&config, "test_file.json").unwrap();
        let expected_path = custom_path.join("test_file.json");
        assert_eq!(result, expected_path);
        assert!(custom_path.exists());
    }

    #[test]
    fn test_get_app_data_dir_creates_dir() {
        let dir = tempdir().unwrap();
        let custom_path = dir.path().join("new_dir");
        assert!(!custom_path.exists());
        let config = create_test_config(Some(custom_path.clone()));
        get_app_data_dir(&config).unwrap();
        assert!(custom_path.exists());
    }

    #[test]
    fn test_get_app_file_path_creates_dir() {
        let dir = tempdir().unwrap();
        let custom_path = dir.path().join("another_new_dir");
        assert!(!custom_path.exists());
        let config = create_test_config(Some(custom_path.clone()));
        get_app_file_path(&config, "another_test_file.txt").unwrap();
        assert!(custom_path.exists());
    }
}
