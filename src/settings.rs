use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub save_on_exit: bool,
    pub log_level: String,
    #[serde(default)]
    pub last_layout_file: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_on_exit: false,
            log_level: "info".to_string(),
            last_layout_file: None,
        }
    }
}

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

pub fn save_settings(settings: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        if let Err(e) = File::create("settings.json")
            .and_then(|mut file| file.write_all(json.as_bytes()))
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
            log_level: "debug".to_string(),
            last_layout_file: Some("file.json".into()),
        };
        save_settings(&settings);
        let loaded = load_settings();
        cleanup();
        assert_eq!(loaded.save_on_exit, true);
        assert_eq!(loaded.log_level, "debug");
        assert_eq!(loaded.last_layout_file.as_deref(), Some("file.json"));
    }

    #[test]
    fn round_trip_false() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup();
        let settings = Settings {
            save_on_exit: false,
            log_level: "info".to_string(),
            last_layout_file: None,
        };
        save_settings(&settings);
        let loaded = load_settings();
        cleanup();
        assert_eq!(loaded.save_on_exit, false);
        assert_eq!(loaded.log_level, "info");
        assert_eq!(loaded.last_layout_file, None);
    }
}
