#![windows_subsystem = "windows"]

mod gui;
mod hotkey;
mod utils;
mod window_manager;
mod workspace;
mod settings;
mod virtual_desktop;
mod desktop_window_info;

use log::info;
use clap::{ArgAction, Parser};
use crate::settings::load_settings;
use crate::window_manager::{
    capture_all_desktops,
    restore_all_desktops,
    move_all_to_origin,
};
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

#[cfg(windows)]
fn ensure_console() {
    use windows::Win32::System::Console::{AttachConsole, AllocConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            let _ = AllocConsole();
        }
    }
}

#[cfg(not(windows))]
fn ensure_console() {}

#[derive(Parser, Debug)]
#[command(author, version, about = "Multi Manager window tool", long_about = None)]
struct CliArgs {
    #[arg(long = "save-desktops", default_missing_value = "desktop_layout.json", num_args = 0..=1)]
    save_desktops: Option<String>,

    #[arg(long = "load-desktops", default_missing_value = "desktop_layout.json", num_args = 0..=1)]
    load_desktops: Option<String>,

    #[arg(long = "move-origin", action = ArgAction::SetTrue)]
    move_origin: bool,

    #[arg(long = "save-workspaces", default_missing_value = "workspaces.json", num_args = 0..=1)]
    save_workspaces: Option<String>,

    #[arg(long = "load-workspaces", default_missing_value = "workspaces.json", num_args = 0..=1)]
    load_workspaces: Option<String>,

    #[arg(long = "open-log-folder", action = ArgAction::SetTrue)]
    open_log_folder: bool,

    #[arg(long = "edit-settings", action = ArgAction::SetTrue)]
    edit_settings: bool,
}

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
    // Count the number of command line arguments. If there is only one
    // (the program name) we skip attaching/allocating a console so that
    // help messages still show correctly when invoked without extra
    // parameters.
    let arg_count = std::env::args_os().len();
    if arg_count > 1 {
        ensure_console();
    }
    let args = CliArgs::parse();

    // Ensure logging is initialized
    ensure_logging_initialized();

    // Backtrace for Debug
    env::set_var("RUST_BACKTRACE", "1");

    info!("Starting Multi Manager application...");

    if let Some(file) = args.save_desktops {
        capture_all_desktops(&file);
        println!("Saved desktops to {}", file);
        return;
    }

    if let Some(file) = args.load_desktops {
        restore_all_desktops(&file);
        println!("Restored desktops from {}", file);
        return;
    }

    if let Some(file) = args.save_workspaces {
        cli_save_workspaces(&file);
        return;
    }

    if let Some(file) = args.load_workspaces {
        cli_load_workspaces(&file);
        return;
    }

    if args.move_origin {
        move_all_to_origin();
        return;
    }

    if args.open_log_folder {
        open_log_folder();
        return;
    }

    if args.edit_settings {
        edit_settings();
        return;
    }

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
        hotkey_dialog: None,
        all_expanded: true,
        expand_all_signal: None,
        show_settings: false,
        auto_save: settings.auto_save,
        unsaved_changes: false,
        save_on_exit: settings.save_on_exit,
        log_level: settings.log_level.clone(),
        last_layout_file: settings.last_layout_file.clone(),
        last_workspace_file: settings.last_workspace_file.clone(),
        developer_debugging: settings.developer_debugging,
    };

    // Launch GUI and set the taskbar icon after creating the window
    gui::run_gui(app);
}

/// Open the folder containing `multi_manager.log` in Windows Explorer.
fn open_log_folder() {
    use crate::utils::show_error_box;

    let log_path = std::fs::canonicalize("multi_manager.log")
        .unwrap_or_else(|_| PathBuf::from("multi_manager.log"));

    if let Err(e) = Command::new("explorer").arg(&log_path).spawn() {
        show_error_box(&format!("Failed to open log folder: {}", e), "Error");
    }
}

/// Launch a text editor to modify `settings.json`.
fn edit_settings() {
    #[cfg(windows)]
    {
        let _ = Command::new("notepad").arg("settings.json").spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg("settings.json").spawn();
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg("settings.json").spawn();
    }
}

fn cli_save_workspaces(path: &str) {
    use std::fs;
    match fs::read_to_string("workspaces.json") {
        Ok(content) => {
            if let Err(e) = fs::write(path, content) {
                eprintln!("Failed to save workspaces: {}", e);
            } else {
                println!("Saved workspaces to {}", path);
            }
        }
        Err(e) => eprintln!("Failed to read workspaces.json: {}", e),
    }
}

fn cli_load_workspaces(path: &str) {
    use std::fs;
    use crate::workspace::Workspace;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", path, e);
            return;
        }
    };

    if serde_json::from_str::<Vec<Workspace>>(&content).is_err() {
        eprintln!("Invalid workspace JSON: {}", path);
        return;
    }

    if let Err(e) = fs::write("workspaces.json", &content) {
        eprintln!("Failed to write workspaces.json: {}", e);
    } else {
        println!("Loaded workspaces from {}", path);
    }
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
