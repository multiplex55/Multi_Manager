use crate::gui::App;
use crate::hotkey::Hotkey;
use crate::window_manager::get_window_position;
use crate::window_manager::listen_for_keys_with_dialog_and_window;
use crate::window_manager::move_window;
use crate::window_manager::*;
use eframe::egui;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

/// Represents a workspace, which groups multiple windows and allows toggling between specific positions.
///
/// # Fields
/// - `name`: The name of the workspace.
/// - `hotkey`: An optional hotkey assigned to the workspace for activation.
/// - `windows`: A list of windows belonging to this workspace.
/// - `disabled`: A flag indicating whether the workspace is disabled.
#[derive(Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub hotkey: Option<Hotkey>,
    pub windows: Vec<Window>,
    pub disabled: bool,
    pub valid: bool,
    #[serde(default)]
    pub rotate: bool,
    #[serde(skip)]
    pub current_index: usize,
}

impl Workspace {
    /// Sets the hotkey for the workspace.
    ///
    /// Validates the provided hotkey and registers it for the workspace if valid.
    ///
    /// # Arguments
    /// - `hotkey`: The key combination to assign as the workspace hotkey (e.g., "Ctrl+Alt+H").
    ///
    /// # Returns
    /// - `Ok(())` if the hotkey is valid and successfully set.
    /// - `Err` with an error message if the hotkey is invalid.
    ///
    /// # Example
    /// ```
    /// let mut workspace = Workspace::new("Example");
    /// if let Err(e) = workspace.set_hotkey("Ctrl+Shift+P") {
    ///     println!("Failed to set hotkey: {}", e);
    /// }
    /// ```
    pub fn set_hotkey(&mut self, hotkey: &str) -> Result<(), String> {
        match Hotkey::new(hotkey) {
            Ok(new_hotkey) => {
                self.hotkey = Some(new_hotkey);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Produces an egui `RichText` label for the workspace **header**, color-coded to represent its state.
    ///
    /// # Behavior
    /// - Checks the `disabled` and `valid` fields of `self`:
    ///   - **Disabled** workspaces: orange text
    ///   - **Valid** workspaces (i.e., at least one valid window + valid hotkey): green text
    ///   - **Invalid** workspaces: red text
    /// - Returns an `egui::RichText` object, which can be displayed in the GUI (e.g., in a collapsible header).
    ///
    /// # Side Effects
    /// - None. It simply returns a text object; no state is mutated.
    ///
    /// # Example
    /// ```rust
    /// let header_label = workspace.get_header_text();
    /// ui.label(header_label);
    /// ```
    ///
    /// # Notes
    /// - Commonly used in the collapsible headers of each workspace in the UI.
    /// - Helps visually distinguish disabled/invalid workspaces at a glance.
    pub fn get_header_text(&self) -> egui::RichText {
        if self.disabled {
            egui::RichText::new(&self.name).color(egui::Color32::ORANGE)
        } else if self.valid {
            egui::RichText::new(&self.name).color(egui::Color32::GREEN)
        } else {
            egui::RichText::new(&self.name).color(egui::Color32::RED)
        }
    }

    /// Renders the workspace details, such as hotkey and windows.
    pub fn render_details(&mut self, ui: &mut egui::Ui) {
        // Hotkey section
        ui.horizontal(|ui| {
            ui.label("Hotkey:");

            let mut temp_hotkey = self
                .hotkey
                .as_ref()
                .map(|h| h.key_sequence.clone())
                .unwrap_or_else(|| "None".to_string());

            if ui.text_edit_singleline(&mut temp_hotkey).changed() {
                match self.set_hotkey(&temp_hotkey) {
                    Ok(_) => {
                        let valid_label = ui.colored_label(egui::Color32::GREEN, "Valid");
                        Self::attach_context_menu(
                            ui,
                            &valid_label,
                            "Valid Hotkey Options",
                            &temp_hotkey,
                        );
                        info!("Hotkey '{}' is valid and set.", temp_hotkey);
                    }
                    Err(_) => {
                        let invalid_label = ui.colored_label(egui::Color32::RED, "Invalid");
                        Self::attach_context_menu(
                            ui,
                            &invalid_label,
                            "Invalid Hotkey Options",
                            &temp_hotkey,
                        );
                        warn!("Hotkey '{}' is invalid.", temp_hotkey);
                    }
                }
            } else if is_valid_key_combo(&temp_hotkey) {
                let valid_label = ui.colored_label(egui::Color32::GREEN, "Valid");
                Self::attach_context_menu(ui, &valid_label, "Valid Hotkey Options", &temp_hotkey);
            } else {
                let invalid_label = ui.colored_label(egui::Color32::GRAY, "Edit to validate");
                Self::attach_context_menu(
                    ui,
                    &invalid_label,
                    "Invalid Hotkey Options",
                    &temp_hotkey,
                );
            }
        });

        ui.checkbox(&mut self.rotate, "Rotate Windows");

        // Create a copy of windows for iteration
        let windows: Vec<_> = self.windows.iter_mut().collect();
        let mut window_to_delete = None;

        for (i, window) in windows.into_iter().enumerate() {
            ui.horizontal(|ui| {
                // Display window title
                ui.label(&window.title);

                // Add delete button
                if ui.button("Delete").clicked() {
                    window_to_delete = Some(i);
                }

                // Handle HWND validity and right-click menu for individual windows
                let exists =
                    unsafe { IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool() };
                if exists {
                    // Define the label and capture its response
                    let label_response = ui.colored_label(
                        egui::Color32::GREEN,
                        format!("HWND: {:?}", window.id),
                    );
                
                    // Create a unique ID for the popup menu
                    let popup_id = egui::Id::new(format!("hwnd_context_menu_workspace_{}_{}", i,window.id));

                
                    // Handle right-click to toggle popup visibility
                    if label_response.hovered() && ui.input(|i| i.pointer.secondary_clicked()) && !ui.memory(|mem| mem.is_popup_open(popup_id)) {
                        ui.memory_mut(|mem| mem.open_popup(popup_id));
                        }
                
                    // Render the popup menu if it's open
                    egui::popup::popup_below_widget(
                        ui,
                        popup_id,
                        &label_response, // Pass the label_response here
                        egui::PopupCloseBehavior::CloseOnClickOutside, // Auto-close on outside click
                        |ui| {
                            ui.label("Options:");
                
                            // Add the "Force Recapture" button
                            if ui.button("Force Recapture").clicked() {
                                info!("Force Recapture triggered for HWND: {:?}", window.id);
                                if let Some("Enter") = listen_for_keys_with_dialog() {
                                    if let Some((new_hwnd, new_title)) = get_active_window() {
                                        // Update the HWND and title
                                        window.id = new_hwnd.0 as usize;
                                        window.title = new_title;
                                        info!(
                                            "Force Recaptured window '{}', new HWND: {:?}",
                                            window.title, new_hwnd
                                        );
                                    } else {
                                        warn!("Force Recapture canceled or no active window detected.");
                                    }
                                }
                    
                                // Explicitly close the popup after the action
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        },
                    );
                    
        } else {
                ui.colored_label(egui::Color32::RED, format!("HWND: {:?}", window.id));
                if ui.button("Recapture").clicked() {
                    if let Some("Enter") = listen_for_keys_with_dialog() {
                        if let Some((new_hwnd, new_title)) = get_active_window() {
                            // Update the invalid window with the new HWND but retain home/target
                            window.id = new_hwnd.0 as usize;
                            window.title = new_title;
                            info!(
                                "Recaptured window '{}', new HWND: {:?}",
                                window.title, new_hwnd
                                );
                            } else {
                                warn!("Recapture canceled or no active window detected.");
                            }
                        }
                    }
                }
            });
            // Render controls for individual window
            render_window_controls(ui, window);
        }

        if let Some(index) = window_to_delete {
            self.windows.remove(index);
        }

        // Capture active window button
        if ui.button("Capture Active Window").clicked() {
            if let Some(("Enter", hwnd, title)) = listen_for_keys_with_dialog_and_window() {
                self.windows.push(Window {
                    id: hwnd.0 as usize,
                    title,
                    home: (0, 0, 800, 600),
                    target: (0, 0, 800, 600),
                    valid: true,
                });
            }
        }
    }

    /// Attaches a context menu to a UI widget.
    ///
    /// This function creates a context menu (popup) that appears when the user right-clicks
    /// on the provided widget. The menu displays the specified options and can trigger
    /// actions based on the selected option.
    ///
    /// # Parameters
    /// - `ui`: The egui `Ui` instance to render the context menu.
    /// - `widget_response`: The response object of the widget to which the menu is attached.
    /// - `menu_title`: The title of the context menu.
    /// - `context_info`: Additional information to display in the context menu.
    ///
    /// # Example
    /// ```
    /// attach_context_menu(ui, &response, "Hotkey Options", "Ctrl+Shift+P");
    /// ```
    pub fn attach_context_menu(
        ui: &mut egui::Ui,
        widget_response: &egui::Response,
        menu_title: &str,
        context_info: &str,
    ) {
        // Create a unique popup ID based on the menu title and context info
        let popup_id = egui::Id::new(format!("{}_{}", menu_title, context_info));

        // Open the popup when the widget is right-clicked
        if widget_response.hovered() && ui.input(|i| i.pointer.secondary_clicked()) {
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        // Render the popup menu
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            widget_response,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.label(menu_title);
                ui.separator();

                if ui.button("Option 1").clicked() {
                    println!("Option 1 clicked for: {}", context_info);
                    ui.close_menu();
                }
                if ui.button("Option 2").clicked() {
                    println!("Option 2 clicked for: {}", context_info);
                    ui.close_menu();
                }
            },
        );
    }

    /// Validates the state of a workspace.
    ///
    /// This function ensures that a workspace is in a valid state by checking:
    /// - If the assigned hotkey (if any) is in a valid format.
    /// - If the workspace contains at least one valid window (an HWND that corresponds to an existing window).
    ///
    /// The `valid` field of the workspace is updated accordingly.
    ///
    /// # Behavior
    /// - Checks the validity of the hotkey using the `is_valid_key_combo` function.
    /// - Verifies the existence of at least one valid window using the Win32 API `IsWindow`.
    /// - Updates the `valid` field of the `Workspace` struct to `true` if both checks pass.
    ///
    /// # Example
    /// ```rust
    /// let mut workspace = Workspace {
    ///     name: "Example".to_string(),
    ///     hotkey: Some("Ctrl+Alt+H".to_string()),
    ///     windows: vec![Window {
    ///         id: 12345,
    ///         title: "Example Window".to_string(),
    ///         home: (0, 0, 800, 600),
    ///         target: (100, 100, 800, 600),
    ///         valid: true,
    ///     }],
    ///     disabled: false,
    ///     valid: false,
    /// };
    /// workspace.validate_workspace();
    /// assert!(workspace.valid);
    /// ```
    ///
    /// # Dependencies
    /// - Relies on `is_valid_key_combo` for hotkey validation.
    /// - Uses the Win32 API `IsWindow` to check window validity.
    ///
    /// # Parameters
    /// - No parameters. Operates directly on the instance of the `Workspace`.
    ///
    /// # Side Effects
    /// - Updates the `valid` field of the `Workspace` struct.
    ///
    /// # Notes
    /// - This function should be called whenever the state of a workspace changes (e.g., hotkey or windows are modified).
    /// - The `disabled` state does not affect validation; it is treated independently.
    pub fn validate_workspace(&mut self) {
        self.valid = {
            let hotkey_valid = self
                .hotkey
                .as_ref()
                .is_some_and(|hotkey| is_valid_key_combo(&hotkey.key_sequence));
            let any_valid_window = self.windows.iter().any(|window| unsafe {
                IsWindow(HWND(window.id as *mut std::ffi::c_void)).as_bool()
            });

            hotkey_valid && any_valid_window
        };
    }
}
/// Presents egui UI elements for configuring **one** `Window`’s positioning data:
/// its **Home** and **Target** coordinates, plus actions to **capture** or **move** the window.
///
/// # Behavior
/// - Displays two horizontal rows, one for the `home` position and one for the `target` position.
/// - In each row:
///   - Shows editable numeric fields (`DragValue`) for `x`, `y`, `width (w)`, and `height (h)`.
///   - Offers a **“Capture”** button to read the current on-screen position via
///     [`get_window_position`](../../window_manager/fn.get_window_position.html).
///   - Offers a **“Move to ...”** button that calls [`move_window`](../../window_manager/fn.move_window.html)
///     to reposition the window immediately.
/// - Any failures (e.g., the window is invalid or moving fails) are logged as warnings.
///
/// # Side Effects
/// - Potentially moves the window on the user’s desktop if “Move” is clicked.
/// - Updates the `home` or `target` fields in `window` when “Capture” is used.
/// - Logs messages about actions taken (`info!`, `warn!`) via the `log` crate.
///
/// # Example
/// ```rust
/// egui::CentralPanel::default().show(ctx, |ui| {
///     render_window_controls(ui, &mut my_window);
/// });
/// ```
///
/// # Notes
/// - This function is called inside `render_details(...)` to iterate over each `Window` in a `Workspace`.
/// - Relies on Win32 calls under the hood to interact with actual OS-level windows (via `HWND`).
pub fn render_window_controls(ui: &mut egui::Ui, window: &mut Window) {
    // Home position controls
    ui.horizontal(|ui| {
        ui.label("Home:");
        ui.add(egui::DragValue::new(&mut window.home.0).prefix("x: "));
        ui.add(egui::DragValue::new(&mut window.home.1).prefix("y: "));
        ui.add(egui::DragValue::new(&mut window.home.2).prefix("w: "));
        ui.add(egui::DragValue::new(&mut window.home.3).prefix("h: "));
        if ui.button("Capture Home").clicked() {
            if let Ok((x, y, w, h)) = get_window_position(HWND(window.id as *mut _)) {
                window.home = (x, y, w, h);
            }
        }
        if ui.button("Move to Home").clicked() {
            if let Err(e) = move_window(
                HWND(window.id as *mut _),
                window.home.0,
                window.home.1,
                window.home.2,
                window.home.3,
            ) {
                warn!("Failed to move window to home: {}", e);
            }
        }
    });

    // Target position controls
    ui.horizontal(|ui| {
        ui.label("Target:");
        ui.add(egui::DragValue::new(&mut window.target.0).prefix("x: "));
        ui.add(egui::DragValue::new(&mut window.target.1).prefix("y: "));
        ui.add(egui::DragValue::new(&mut window.target.2).prefix("w: "));
        ui.add(egui::DragValue::new(&mut window.target.3).prefix("h: "));
        if ui.button("Capture Target").clicked() {
            if let Ok((x, y, w, h)) = get_window_position(HWND(window.id as *mut _)) {
                window.target = (x, y, w, h);
            }
        }
        if ui.button("Move to Target").clicked() {
            if let Err(e) = move_window(
                HWND(window.id as *mut _),
                window.target.0,
                window.target.1,
                window.target.2,
                window.target.3,
            ) {
                warn!("Failed to move window to target: {}", e);
            }
        }
    });
}

/// A **logical record** of a window managed by the application, linking its **HWND** (`id`)
/// and **title** to two possible positions (`home` and `target`).
///
/// # Fields
/// - `id`: The unique handle (`HWND` cast to `usize`) identifying the window in the OS.
/// - `title`: A user-friendly title string, typically captured from the actual window’s title bar.
/// - `home`: A tuple `(x, y, width, height)` describing the “home” position (and size) for this window.
/// - `target`: A tuple `(x, y, width, height)` describing the “target” position (and size).
/// - `valid`: Indicates whether the window is considered valid (e.g., captured from a real HWND).
///
/// # Behavior
/// - Used within a `Workspace` to toggle windows between `home` and `target` positions.
/// - If `valid` is `false`, the UI and logic may treat this window as non-existent or needing recapture.
///
/// # Example
/// ```rust
/// let window = Window {
///     id: 12345,  // Some valid HWND cast to usize
///     title: "My App".to_string(),
///     home: (0, 0, 800, 600),
///     target: (100, 100, 1024, 768),
///     valid: true,
/// };
/// ```
///
/// # Notes
/// - This struct is [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
///   and [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html),
///   meaning it can be saved to and loaded from JSON or other formats.
/// - The actual OS-specific window handle is stored in `id`; we cast it from/to `HWND` when using Win32 APIs.
#[derive(Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: usize,
    pub title: String,
    pub home: (i32, i32, i32, i32),
    pub target: (i32, i32, i32, i32),
    pub valid: bool,
}

/// Checks whether the provided `input` string (e.g., `"Ctrl+Alt+F5"`, `"Win+Shift+Z"`) matches a valid hotkey pattern.
///
/// # Behavior
/// - Uses a [`regex`](https://crates.io/crates/regex) pattern to match up to four possible modifiers
///   (`Ctrl`, `Alt`, `Shift`, `Win`) followed by a single main key (e.g., `F1`, `A`, `Esc`, `LeftAlt`, etc.).
/// - Returns `true` if the string fully conforms to the recognized hotkey format, otherwise `false`.
///
/// # Side Effects
/// - None. The function only checks against a compiled regex and does not mutate any state.
///
/// # Example
/// ```rust
/// if is_valid_key_combo("Ctrl+Shift+P") {
///     println!("Valid key combo!");
/// } else {
///     println!("Invalid key combo.");
/// }
/// ```
///
/// # Notes
/// - This function does not verify whether the key is actually usable in Windows (for that, see
///   [`virtual_key_from_string`](../../window_manager/fn.virtual_key_from_string.html)).
pub fn is_valid_key_combo(input: &str) -> bool {
    let pattern = r"^(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:(?:Ctrl|Alt|Shift|Win)\+)?(?:F(?:[1-9]|1[0-2]|1[3-9]|2[0-4])|[A-Z]|[0-9]|NUMPAD[0-9]|NUMPAD(?:MULTIPLY|ADD|SEPARATOR|SUBTRACT|DOT|DIVIDE)|UP|DOWN|LEFT|RIGHT|BACKSPACE|TAB|ENTER|PAUSE|CAPSLOCK|ESCAPE|SPACE|PAGEUP|PAGEDOWN|END|HOME|INSERT|DELETE|OEM_(?:PLUS|COMMA|MINUS|PERIOD|[1-7])|PRINTSCREEN|SCROLLLOCK|NUMLOCK|LEFT(?:SHIFT|CTRL|ALT)|RIGHT(?:SHIFT|CTRL|ALT))$";
    let re = Regex::new(pattern).unwrap();
    re.is_match(input)
}

/// Saves a list of workspaces to a JSON file.
///
/// This function serializes the list of `Workspace` objects into a JSON string
/// and writes it to a specified file. If the file does not exist, it is created.
/// If serialization or file writing fails, appropriate error messages are logged.
///
/// # Behavior
/// - Serializes the `workspaces` list into JSON format using `serde_json`.
/// - Writes the JSON string to the specified file path.
/// - Logs success or failure of the operation.
///
/// # Example
/// ```rust
/// let workspaces = vec![Workspace {
///     name: "Workspace 1".to_string(),
///     hotkey: Some("Ctrl+Alt+1".to_string()),
///     windows: vec![],
///     disabled: false,
///     valid: true,
/// }];
///
/// save_workspaces(&workspaces, "workspaces.json");
/// ```
///
/// # Dependencies
/// - Relies on `serde_json` for serialization.
/// - Uses Rust's standard `File` and `Write` traits for file handling.
///
/// # Parameters
/// - `workspaces: &[Workspace]`: A reference to the list of `Workspace` objects to be saved.
/// - `file_path: &str`: The path to the file where the serialized data will be written.
///
/// # Side Effects
/// - Creates or overwrites the specified file with the serialized workspace data.
///
/// # Error Conditions
/// - Logs an error if:
///   - Serialization fails (e.g., due to invalid data).
///   - File creation or writing fails (e.g., due to insufficient permissions).
///
/// # Notes
/// - Ensure the `workspaces` list is properly populated before calling this function.
/// - The function does not return errors but logs them for debugging purposes.
pub fn save_workspaces(workspaces: &[Workspace], file_path: &str) {
    match serde_json::to_string_pretty(workspaces) {
        Ok(json) => {
            if let Err(e) =
                File::create(file_path).and_then(|mut file| file.write_all(json.as_bytes()))
            {
                error!("Failed to save workspaces to '{}': {}", file_path, e);
            } else {
                info!("Workspaces successfully saved to '{}'.", file_path);
            }
        }
        Err(e) => {
            error!("Failed to serialize workspaces: {}", e);
        }
    }
}

/// Loads a list of workspaces from a JSON file.
///
/// This function reads a JSON file containing workspace configurations and deserializes it into a vector of `Workspace` objects.
/// It also attempts to register any associated hotkeys using the provided `App` instance.
///
/// # Behavior
/// - Reads the specified file and parses its contents as JSON.
/// - Registers hotkeys for each workspace if the hotkey is valid and not already registered.
/// - Logs warnings for invalid or unregistered hotkeys.
/// - If the file is missing or invalid, returns an empty list.
///
/// # Example
/// ```rust
/// let app = App {
///     workspaces: Arc::new(Mutex::new(Vec::new())),
///     last_hotkey_info: Arc::new(Mutex::new(None)),
///     hotkey_promise: Arc::new(Mutex::new(None)),
///     initial_validation_done: Arc::new(Mutex::new(false)),
///     registered_hotkeys: Arc::new(Mutex::new(HashMap::new())),
/// };
/// let workspaces = load_workspaces("workspaces.json", &app);
/// ```
///
/// # Dependencies
/// - Relies on `serde_json` for deserialization.
/// - Uses the `register_hotkey` function to attempt hotkey registration.
///
/// # Parameters
/// - `file_path: &str`: The path to the JSON file containing workspace data.
/// - `app: &App`: Reference to the `App` instance for managing hotkey registration.
///
/// # Returns
/// - A `Vec<Workspace>` containing the loaded workspaces, with hotkeys registered where possible.
///
/// # Side Effects
/// - Modifies the `registered_hotkeys` field of the `App` instance by adding valid hotkeys.
///
/// # Error Conditions
/// - Logs warnings if:
///   - The file cannot be read.
///   - The JSON is invalid or cannot be deserialized.
///   - A hotkey cannot be registered.
///
/// # Notes
/// - Ensure the file exists and is in the correct JSON format.
/// - Hotkeys that fail registration are not removed from the workspace but are logged as invalid.
pub fn load_workspaces(file_path: &str, app: &App) -> Vec<Workspace> {
    let mut content = String::new();
    match File::open(file_path) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut content) {
                error!("Failed to read file '{}': {}", file_path, e);
                return Vec::new();
            }
            match serde_json::from_str::<Vec<Workspace>>(&content) {
                Ok(mut workspaces) => {
                    info!("Successfully loaded workspaces from '{}'.", file_path);

                    for (i, workspace) in workspaces.iter_mut().enumerate() {
                        if let Some(ref mut hotkey) = workspace.hotkey {
                            if !hotkey.register(app, i as i32) {
                                warn!(
                                    "Failed to register hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            } else {
                                info!(
                                    "Registered hotkey '{}' for workspace '{}'.",
                                    hotkey, workspace.name
                                );
                            }
                        }
                        workspace.current_index = 0;
                    }

                    workspaces
                }
                Err(e) => {
                    warn!(
                        "Failed to parse JSON in '{}': {}. Returning empty workspace list.",
                        file_path, e
                    );
                    Vec::new()
                }
            }
        }
        Err(e) => {
            warn!(
                "File '{}' not found or cannot be opened: {}. Returning empty workspace list.",
                file_path, e
            );
            Vec::new()
        }
    }
}
