use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

/// Persistent configuration options loaded from and saved to `settings.json`.
///
/// These values control global behavior such as logging verbosity and whether
/// the application should automatically save workspaces on exit.
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    /// If `true`, workspaces are automatically saved when the application exits.
    pub save_on_exit: bool,
    /// If `true`, workspaces are saved automatically whenever changes occur.
    #[serde(default)]
    pub auto_save: bool,
    /// The log level used when initializing the logger (e.g. `"info"`).
    pub log_level: String,
    /// Optional path to the last desktop layout file used.
    #[serde(default)]
    pub last_layout_file: Option<String>,
    /// Optional path to the last workspace file used.
    #[serde(default)]
    pub last_workspace_file: Option<String>,
    /// Optional path to the last saved window bindings file used.
    #[serde(default)]
    pub last_bindings_file: Option<String>,
    /// If `true`, additional developer debugging information is shown.
    #[serde(default)]
    pub developer_debugging: bool,
}

impl Default for Settings {
    /// Returns a `Settings` instance with sensible defaults.
    fn default() -> Self {
        Self {
            save_on_exit: false,
            auto_save: false,
            log_level: "info".to_string(),
            last_layout_file: None,
            last_workspace_file: None,
            last_bindings_file: None,
            developer_debugging: false,
        }
    }
}

/// Load persisted settings from `settings.json` if it exists.
///
/// If the file cannot be read or parsed, default settings are returned.
pub fn load_settings() -> Settings {
    let mut content = String::new();
    if let Ok(mut file) = File::open("settings.json") {
        if file.read_to_string(&mut content).is_ok() {
            if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                return settings;
            }
        }
    }
    Settings::default()
}

/// Save the provided `settings` struct to `settings.json` in a human
/// readable format.
pub fn save_settings(settings: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        if let Err(e) =
            File::create("settings.json").and_then(|mut file| file.write_all(json.as_bytes()))
        {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn cleanup() {
        if Path::new("settings.json").exists() {
            let _ = fs::remove_file("settings.json");
        }
    }

    #[test]
    fn round_trip_true() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup();
        let settings = Settings {
            save_on_exit: true,
            auto_save: true,
            log_level: "debug".to_string(),
            last_layout_file: Some("file.json".into()),
            last_workspace_file: Some("work.json".into()),
            last_bindings_file: Some("bindings.json".into()),
            developer_debugging: true,
        };
        save_settings(&settings);
        let loaded = load_settings();
        cleanup();
        assert_eq!(loaded.save_on_exit, true);
        assert_eq!(loaded.auto_save, true);
        assert_eq!(loaded.log_level, "debug");
        assert_eq!(loaded.last_layout_file.as_deref(), Some("file.json"));
        assert_eq!(loaded.last_workspace_file.as_deref(), Some("work.json"));
        assert_eq!(loaded.last_bindings_file.as_deref(), Some("bindings.json"));
        assert_eq!(loaded.developer_debugging, true);
    }

    #[test]
    fn round_trip_false() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup();
        let settings = Settings {
            save_on_exit: false,
            auto_save: false,
            log_level: "info".to_string(),
            last_layout_file: None,
            last_workspace_file: None,
            last_bindings_file: None,
            developer_debugging: false,
        };
        save_settings(&settings);
        let loaded = load_settings();
        cleanup();
        assert_eq!(loaded.save_on_exit, false);
        assert_eq!(loaded.auto_save, false);
        assert_eq!(loaded.log_level, "info");
        assert_eq!(loaded.last_layout_file, None);
        assert_eq!(loaded.last_workspace_file, None);
        assert_eq!(loaded.last_bindings_file, None);
        assert_eq!(loaded.developer_debugging, false);
    }
}
