use std::path::PathBuf;
use tracing::warn;

/// Get the application data directory
/// - Windows: %APPDATA%\auto-gmail
/// - Mac/Linux: ~/.auto-gmail
pub fn get_app_data_dir() -> PathBuf {
    let app_data_dir = if cfg!(windows) {
        let appdata = std::env::var("APPDATA")
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("auto-gmail")
    } else {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".auto-gmail")
    };

    // Ensure the directory exists
    if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
        warn!("Could not create app data directory: {}", e);
        return PathBuf::from(".");
    }

    app_data_dir
}

/// Get an absolute path in the app data directory
pub fn get_app_file_path(filename: &str) -> PathBuf {
    get_app_data_dir().join(filename)
}
