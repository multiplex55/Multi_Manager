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
        log_level: settings.log_level.clone(),
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
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;

    let settings = load_settings();
    let level = match settings.log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "off" => LevelFilter::Off,
        _ => LevelFilter::Info,
    };

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}{n}")))
        .append(false)
        .build("multi_manager.log")
        .expect("failed to create log file");

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(logfile)))
        .build(Root::builder().appender("file").build(level))
        .expect("failed to build log configuration");

    if let Err(e) = log4rs::init_config(config) {
        eprintln!("Failed to initialize logging: {}", e);
    }
}
