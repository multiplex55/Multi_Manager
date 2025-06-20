#![windows_subsystem = "windows"]

mod gui;
mod hotkey;
mod utils;
mod window_manager;
mod workspace;
mod settings;

use log::info;
use crate::settings::load_settings;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write; // Fix for write_all error
use std::sync::{Arc, Mutex};

/// The main entry point for the Multi Manager application.
///
/// # Behavior
/// - Initializes logging.
/// - Sets the `RUST_BACKTRACE` environment variable to `1` for debugging.
/// - Creates the application's initial state (e.g., shared `Arc<Mutex<...>>` structures).
/// - Launches the GUI via `gui::run_gui()`.
///
/// # Side Effects
/// - If logging fails to initialize, attempts to create a default `log4rs.yaml` file.
/// - May terminate the process if logging configuration cannot be created.
/// - Spawns the main GUI and blocks until the GUI exits.
///
/// # Notes
/// - This function must be kept at the top level so it can serve as the program's entry point.
/// - Windows subsystem is set to `"windows"`, so no console window will appear by default.
///
/// # Example
/// ```
/// // Launch the Multi Manager application.
/// // Typically invoked by the OS when the user runs the compiled binary.
/// fn main() {
///     // ...
/// }
/// ```
fn main() {
    // Ensure logging is initialized
    ensure_logging_initialized();

    // Backtrace for Debug
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    let settings = load_settings();

    // Initialize the application states
    let app = gui::App {
        app_title_name: "Multi Manager".to_string(),
        workspaces: Arc::new(Mutex::new(Vec::new())),
        last_hotkey_info: Arc::new(Mutex::new(None)), // Initialize to None
        hotkey_promise: Arc::new(Mutex::new(None)),   // Initialize the promise
        initial_validation_done: Arc::new(Mutex::new(false)), // Initialize flag to false
        registered_hotkeys: Arc::new(Mutex::new(HashMap::new())), // Initialize the map
        rename_dialog: None,
        all_expanded: true,
        expand_all_signal: None,
        show_settings: false,
        save_on_exit: settings.save_on_exit,
    };

    // Launch GUI and set the taskbar icon after creating the window
    gui::run_gui(app);
}

/// Ensures that a valid `log4rs.yaml` logging configuration file exists and initializes the logger.
///
/// # Behavior
/// - Attempts to initialize logging using the `log4rs.yaml` file.
/// - If the file is missing or invalid:
///   - Creates a default `log4rs.yaml`
///   - Retries the initialization with the newly created file
/// - If the configuration fails even after creating a default file, the application exits with an error.
///
/// # Side Effects
/// - May create or overwrite `log4rs.yaml` in the current working directory.
/// - Immediately sets up logging for the entire application.
///
/// # Error Conditions
/// - If `log4rs.yaml` cannot be created or opened, the process will terminate.
/// - Logs errors to `stderr` if logging configuration cannot be initialized.
///
/// # Notes
/// - This function is called early in `main()` to ensure logging is available from the start.
/// - The logging level is set to `info` by default, unless changed in `log4rs.yaml`.
///
/// # Example
/// ```
/// ensure_logging_initialized();
/// log::info!("Logging is now initialized and ready.");
/// ```
fn ensure_logging_initialized() {
    // Attempt to initialize logging configuration
    if let Err(err) = log4rs::init_file("log4rs.yaml", Default::default()) {
        eprintln!("Failed to initialize log4rs: {}", err);

        // Create a default log4rs.yaml file
        let default_config = r#"
appenders:
  file:
    kind: file
    path: "multi_manager.log"
    append: false
    encoder:
      pattern: "{d} - {l} - {m}{n}"

root:
  level: info
  appenders:
    - file
"#;

        if let Err(e) = File::create("log4rs.yaml")
            .and_then(|mut file| file.write_all(default_config.as_bytes()))
        {
            eprintln!("Failed to create default log4rs.yaml: {}", e);
            std::process::exit(1); // Exit if we cannot create the default configuration
        }

        // Retry initializing log4rs with the newly created configuration file
        if let Err(e) = log4rs::init_file("log4rs.yaml", Default::default()) {
            eprintln!(
                "Failed to reinitialize log4rs with default configuration: {}",
                e
            );
            std::process::exit(1); // Exit if retry fails
        }
    }
}
