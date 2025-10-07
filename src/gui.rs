use crate::utils::*;
use crate::window_manager::{
    check_hotkeys,
    send_all_windows_home,
    capture_all_desktops,
    restore_all_desktops,
    move_all_to_origin,
    get_active_window,
    poll_recapture_keys,
    RecaptureAction,
};
use crate::workspace::*;
use crate::settings::{save_settings, Settings};
use eframe::egui::{self, TopBottomPanel, menu};
use eframe::egui::ViewportBuilder;
use eframe::NativeOptions;
use eframe::{self, App as EframeApp};
use log::{info, warn};
use poll_promise::Promise;
use rfd::FileDialog;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

#[derive(Clone)]
pub struct App {
    pub app_title_name: String,
    pub workspaces: Arc<Mutex<Vec<Workspace>>>,
    pub last_hotkey_info: Arc<Mutex<Option<(String, Instant)>>>,
    pub hotkey_promise: Arc<Mutex<Option<Promise<()>>>>,
    pub initial_validation_done: Arc<Mutex<bool>>,
    pub registered_hotkeys: Arc<Mutex<HashMap<String, usize>>>,
    pub rename_dialog: Option<(usize, String)>,
    pub hotkey_dialog: Option<(usize, String)>,
    pub all_expanded: bool,
    pub expand_all_signal: Option<bool>,
    pub show_settings: bool,
    pub auto_save: bool,
    pub unsaved_changes: bool,
    pub save_on_exit: bool,
    pub log_level: String,
    pub last_layout_file: Option<String>,
    pub last_workspace_file: Option<String>,
    pub developer_debugging: bool,
    pub recapture_queue: Vec<(usize, usize)>,
    pub recapture_active: bool,
}

pub struct WorkspaceControlContext<'a> {
    pub workspace_to_delete: &'a mut Option<usize>,
    pub move_up_index: &'a mut Option<usize>,
    pub move_down_index: &'a mut Option<usize>,
    pub workspaces_len: usize,
    pub index: usize,
}

//
/// This function is responsible for:
/// - Loading existing workspace configurations from a JSON file.
/// - Validating and registering hotkeys for the workspaces.
/// - Spawning a background thread to monitor hotkey presses.
/// - Initializing and running the GUI using the `eframe` framework.
///
/// # Example
/// ```rust
/// let app = App {
///     app_title_name: "Multi Manager".to_string(),
///     workspaces: Arc::new(Mutex::new(Vec::new())),
///     last_hotkey_info: Arc::new(Mutex::new(None)),
///     hotkey_promise: Arc::new(Mutex::new(None)),
///     initial_validation_done: Arc::new(Mutex::new(false)),
///     registered_hotkeys: Arc::new(Mutex::new(HashMap::new())),
/// };
/// run_gui(app);
/// ```
///
/// # Dependencies
/// - `eframe` for GUI rendering.
/// - `poll_promise` for asynchronous hotkey monitoring.
/// - `image` for loading the application icon.
///
/// # Parameters
/// - `app: App`: An instance of the `App` struct containing the application's state.
///
/// # Behavior
/// - Loads workspaces from the `workspaces.json` file.
/// - Starts a background thread for checking hotkey presses.
/// - Configures the GUI with a custom application icon and launches it.
///
/// # Side Effects
/// - Reads from the `workspaces.json` file to load saved configurations.
/// - Registers hotkeys and logs any failures during the process.
/// - Spawns a background thread that continuously monitors hotkeys.
///
/// # Error Conditions
/// - Logs and exits if the GUI fails to initialize or run.
/// - Logs errors if the `workspaces.json` file is missing or contains invalid data.
///
/// # Notes
/// - The background thread runs indefinitely, polling for hotkey presses every 100 milliseconds.
/// - Ensure that the `workspaces.json` file exists and is writable to preserve state.
pub fn run_gui(app: App) {
    {
        let mut workspaces = app.workspaces.lock().unwrap();
        let path = app
            .last_workspace_file
            .clone()
            .unwrap_or_else(|| "workspaces.json".to_string());
        *workspaces = load_workspaces(&path, &app);
    }

    app.validate_initial_hotkeys();

    let app_for_promise = app.clone();
    let hotkey_promise = Promise::spawn_thread("Hotkey Checker", move || loop {
        check_hotkeys(&app_for_promise);
        thread::sleep(Duration::from_millis(100));
    });
    *app.hotkey_promise.lock().unwrap() = Some(hotkey_promise);

    let icon_data = include_bytes!("../resources/app_icon.ico");
    let image = image::load_from_memory(icon_data)
        .expect("Failed to load embedded icon")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let icon_rgba = image.into_raw();

    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_icon(egui::IconData {
            rgba: icon_rgba,
            width,
            height,
        }),
        ..Default::default()
    };

    eframe::run_native(
        &app.app_title_name.clone(),
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .expect("Failed to run GUI");
}

impl EframeApp for App {
    /// The **main update callback** for this application, invoked by the eframe framework on each GUI frame.
    ///
    /// # Behavior
    /// - Renders the central panel and its contents using egui, calling:
    ///   - `render_header` for the top header section (title, buttons).
    ///   - `render_workspace_list` for listing and managing individual workspaces.
    /// - Collects user actions (e.g., "Save Workspaces," "Add Workspace," or "Delete Workspace") and processes them:
    ///   - `save_workspaces()` is called if the user clicks "Save Workspaces."
    ///   - `add_workspace(...)` is invoked if they click "Add New Workspace."
    ///   - `delete_workspace(...)` is invoked if they click "Delete Workspace."
    /// - By default, keeps the panel open and re-renders continuously; any user-driven changes are immediately reflected.
    ///
    /// # Side Effects
    /// - Modifies internal state such as the `workspaces` vector when adding or deleting entries.
    /// - Calls `save_workspaces()` to persist changes to disk on demand.
    /// - Responds in real time to user interactions (mouse clicks, text edits, etc.).
    ///
    /// # Example
    /// This method is **automatically** called by eframe at ~60 FPS (or as fast as the GPU can handle), so you typically
    /// don’t call it manually. Instead, you customize how your UI should behave within this callback:
    /// ```rust
    /// impl eframe::App for App {
    ///     fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    ///         // ... custom UI code here ...
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    /// - The `ctx` parameter provides access to egui’s painting and widget APIs.
    /// - The `_frame` parameter can be used to control window-level properties (size, decorations, etc.), though in this
    ///   code it’s not currently used.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut save_flag = false;
        let mut new_workspace: Option<Workspace> = None;
        let mut workspace_to_delete: Option<usize> = None;

        self.render_menu_bar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui, &mut save_flag, &mut new_workspace);
            ui.separator();
            self.render_workspace_list(ui, &mut workspace_to_delete);
        });

        if self.recapture_active {
            self.process_recapture_all(ctx);
        }

        if save_flag {
            self.save_workspaces();
        }
        if let Some(ws) = new_workspace {
            self.add_workspace(ws);
        }
        if let Some(index) = workspace_to_delete {
            self.delete_workspace(index);
        }

        if self.show_settings {
            self.render_settings_window(ctx);
        }

        if self.auto_save && self.unsaved_changes {
            self.save_workspaces();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.save_on_exit {
            self.save_workspaces();
        }
        save_settings(&Settings {
            save_on_exit: self.save_on_exit,
            auto_save: self.auto_save,
            log_level: self.log_level.clone(),
            last_layout_file: self.last_layout_file.clone(),
            last_workspace_file: self.last_workspace_file.clone(),
            developer_debugging: self.developer_debugging,
        });
    }
}

impl App {
    /// Renders the application's menu bar with a "File" menu.
    ///
    /// The menu contains a single "Settings" item that sets
    /// `self.show_settings` to `true` when selected.
    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.menu_button("Desktop Management", |ui| {
                        if ui.button("Save All Desktops").clicked() {
                            let default_path = self
                                .last_layout_file
                                .clone()
                                .unwrap_or_else(|| "desktop_layout.json".to_string());
                            let chosen = rfd::FileDialog::new()
                                .set_file_name(&default_path)
                                .save_file()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or(default_path);
                            capture_all_desktops(&chosen);
                            self.last_layout_file = Some(chosen.clone());
                            save_settings(&Settings {
                                save_on_exit: self.save_on_exit,
                                auto_save: self.auto_save,
                                log_level: self.log_level.clone(),
                                last_layout_file: self.last_layout_file.clone(),
                                last_workspace_file: self.last_workspace_file.clone(),
                                developer_debugging: self.developer_debugging,
                            });
                            show_message_box("Desktops saved", "Save");
                            ui.close_menu();
                        }
                        if ui.button("Restore All Desktops").clicked() {
                            let default_path = self
                                .last_layout_file
                                .clone()
                                .unwrap_or_else(|| "desktop_layout.json".to_string());
                            let chosen = rfd::FileDialog::new()
                                .set_file_name(&default_path)
                                .pick_file()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or(default_path);
                            restore_all_desktops(&chosen);
                            self.last_layout_file = Some(chosen.clone());
                            save_settings(&Settings {
                                save_on_exit: self.save_on_exit,
                                auto_save: self.auto_save,
                                log_level: self.log_level.clone(),
                                last_layout_file: self.last_layout_file.clone(),
                                last_workspace_file: self.last_workspace_file.clone(),
                                developer_debugging: self.developer_debugging,
                            });
                            ui.close_menu();
                        }
                        if ui.button("Move All to Origin").clicked() {
                            move_all_to_origin();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Workspace Management", |ui| {
                        if ui.button("Save Workspaces...").clicked() {
                            self.save_workspaces();
                            show_message_box("Workspaces saved successfully!", "Save");
                            ui.close_menu();
                        }
                        if ui.button("Save Workspaces As...").clicked() {
                            let default_path = self
                                .last_workspace_file
                                .clone()
                                .unwrap_or_else(|| "workspaces.json".to_string());
                            if let Some(chosen) = rfd::FileDialog::new()
                                .set_file_name(&default_path)
                                .save_file()
                                .map(|p| p.to_string_lossy().to_string())
                            {
                                self.save_workspaces_to_file(&chosen);
                                show_message_box("Workspaces saved successfully!", "Save");
                            }
                            ui.close_menu();
                        }
                        if ui.button("Load Workspaces...").clicked() {
                            let default_path = self
                                .last_workspace_file
                                .clone()
                                .unwrap_or_else(|| "workspaces.json".to_string());
                            if let Some(chosen) = rfd::FileDialog::new()
                                .set_file_name(&default_path)
                                .pick_file()
                                .map(|p| p.to_string_lossy().to_string())
                            {
                                self.load_workspaces_from_file(&chosen);
                            }
                            ui.close_menu();
                        }
                    });
                    if ui.button("Open Log Folder").clicked() {
                        self.open_log_folder();
                        ui.close_menu();
                    }
                    if ui.button("Recapture All").clicked() {
                        self.start_recapture_all();
                        ui.close_menu();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }
    /// Renders the header section of the application's GUI.
    ///
    /// This function displays:
    /// - The application's title.
    /// - Controls for adding a new workspace and managing the workspace list.
    ///
    /// # Behavior
    /// - The "Add New Workspace" button creates a new workspace with a default name and adds it to the list.
    ///
    /// # Example
    /// ```rust
    /// let mut save_flag = false;
    /// let mut new_workspace = None;
    /// let app = App {
    ///     app_title_name: "Multi Manager".to_string(),
    ///     workspaces: Arc::new(Mutex::new(Vec::new())),
    ///     ..Default::default()
    /// };
    /// egui::CentralPanel::default().show(&ctx, |ui| {
    ///     app.render_header(ui, &mut save_flag, &mut new_workspace);
    /// });
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the header.
    /// - `_save_flag: &mut bool`: Reserved for future use.
    /// - `new_workspace: &mut Option<Workspace>`: A mutable reference to store a newly created workspace.
    ///
    /// # Side Effects
    /// - Adds a new workspace to `new_workspace` when the "Add New Workspace" button is clicked.
    ///
    /// # Notes
    /// - The new workspace is initialized with a default name based on the current number of workspaces.
    fn render_header(
        &mut self,
        ui: &mut egui::Ui,
        _save_flag: &mut bool,
        new_workspace: &mut Option<Workspace>,
    ) {
        ui.heading(&self.app_title_name);
        ui.horizontal(|ui| {
            if ui.button("Add New Workspace").clicked() {
                let workspaces = self.workspaces.lock().unwrap();
                *new_workspace = Some(Workspace {
                    name: format!("Workspace {}", workspaces.len() + 1),
                    hotkey: None,
                    windows: Vec::new(),
                    disabled: false,
                    valid: false,
                    rotate: false,
                    current_index: 0,
                });
            }
            if ui.button("Send All Home").clicked() {
                self.send_all_home();
            }
            let label = if self.all_expanded {
                "Collapse All"
            } else {
                "Expand All"
            };
            if ui.button(label).clicked() {
                self.all_expanded = !self.all_expanded;
                self.expand_all_signal = Some(self.all_expanded);
            }
        });
    }
    /// Renders the list of workspaces in the application's GUI.
    ///
    /// This function displays each workspace as a collapsible header, allowing users to view and edit details.
    /// It also provides controls for reordering and deleting workspaces.
    ///
    /// # Behavior
    /// - Displays workspaces in a scrollable area.
    /// - Allows workspaces to be moved up or down in the list.
    /// - Allows individual workspaces to be deleted with confirmation.
    /// - Each workspace's details are rendered using the `Workspace` struct's `render_details` method.
    ///
    /// # Example
    /// ```rust
    /// let mut workspace_to_delete = None;
    /// app.render_workspace_list(ui, &mut workspace_to_delete);
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the workspace list.
    /// - `workspace_to_delete: &mut Option<usize>`: A mutable reference to the index of the workspace to be deleted.
    ///
    /// # Side Effects
    /// - Modifies the workspace list by deleting or reordering items.
    /// - Updates the indices of the workspaces when reordered.
    ///
    /// # Notes
    /// - The list is displayed within a scrollable area to handle large numbers of workspaces.
    /// - Moving a workspace up or down swaps it with the adjacent workspace.
    /// - Deleting a workspace removes it from the list and requires user confirmation.
    fn render_workspace_list(
        &mut self,
        ui: &mut egui::Ui,
        workspace_to_delete: &mut Option<usize>,
    ) {
        let mut move_up_index: Option<usize> = None;
        let mut move_down_index: Option<usize> = None;

        let mut any_changed = false;
        let mut requested_hotkey: Option<usize> = None;
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut workspaces = self.workspaces.lock().unwrap();
                let workspaces_len = workspaces.len();

                for (i, workspace) in workspaces.iter_mut().enumerate() {
                    workspace.validate_workspace();
                    let header_text = workspace.get_header_text();
                    let header_id = egui::Id::new(format!("workspace_{}_header", i));

                    let mut state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            header_id,
                            true,
                        );
                    if let Some(expand) = self.expand_all_signal {
                        state.set_open(expand);
                    }

                    let (_toggle_response, header_inner, _) = state
                        .show_header(ui, |ui| {
                            let label_response = ui.label(header_text);
                            label_response.context_menu(|ui| {
                                if ui.button("Rename").clicked() {
                                    self.rename_dialog = Some((i, workspace.name.clone()));
                                    ui.close_menu();
                                }
                            });
                        })
                        .body(|ui| {
                            let (changed, open_dialog) = workspace.render_details(ui, self);
                            if changed {
                                any_changed = true;
                            }
                            if open_dialog {
                                requested_hotkey = Some(i);
                            }

                            let mut context = WorkspaceControlContext {
                                workspace_to_delete,
                                move_up_index: &mut move_up_index,
                                move_down_index: &mut move_down_index,
                                workspaces_len,
                                index: i,
                            };

                            if self.render_workspace_controls(ui, workspace, &mut context) {
                                any_changed = true;
                            }
                        });

                    // Attach right-click context menu to the header for renaming
                    header_inner.response.context_menu(|ui| {
                        if ui.button("Rename").clicked() {
                            self.rename_dialog = Some((i, workspace.name.clone()));
                            ui.close_menu();
                        }
                    });
                }
            });
        if any_changed {
            self.unsaved_changes = true;
        }

        // Reset expand_all_signal after use
        self.expand_all_signal = None;

        // Move workspace up/down if requested
        if let Some(i) = move_up_index {
            let mut workspaces = self.workspaces.lock().unwrap();
            if i > 0 {
                workspaces.swap(i, i - 1);
                self.unsaved_changes = true;
            }
        }
        if let Some(i) = move_down_index {
            let mut workspaces = self.workspaces.lock().unwrap();
            if i < workspaces.len() - 1 {
                workspaces.swap(i, i + 1);
                self.unsaved_changes = true;
            }
        }

        if let Some(idx) = requested_hotkey {
            self.hotkey_dialog = Some((idx, String::new()));
        }

        // Take the dialog state out to avoid borrow conflicts
        if let Some((index, mut name_buf)) = self.rename_dialog.take() {
            let mut close_dialog = false;
            let mut rename_confirmed = false;

            egui::Window::new("Rename Workspace")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Enter new workspace name:");
                    let text_response = ui.text_edit_singleline(&mut name_buf);

                    if text_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        rename_confirmed = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            rename_confirmed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if rename_confirmed {
                let mut workspaces = self.workspaces.lock().unwrap();
                if let Some(ws) = workspaces.get_mut(index) {
                    ws.name = name_buf;
                    self.unsaved_changes = true;
                }
                // Dialog stays closed
            } else if !close_dialog {
                // User neither confirmed nor cancelled, so put dialog state back
                self.rename_dialog = Some((index, name_buf));
            }
            // else: Dialog cancelled, don't put it back
        }

        // Hotkey capture dialog
        if let Some((index, mut sequence)) = self.hotkey_dialog.take() {
            let mut close_dialog = false;
            let mut confirm = false;

            egui::Window::new("Set Hotkey")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Press combination then press Enter or click OK");
                    ui.label(format!("Current: {}", if sequence.is_empty() { "<none>" } else { &sequence }));

                    ui.ctx().input(|i| {
                        for ev in &i.events {
                            if let egui::Event::Key { key, pressed: true, .. } = ev {
                                if *key == egui::Key::Escape {
                                    close_dialog = true;
                                } else if *key == egui::Key::Enter {
                                    if !sequence.is_empty() { confirm = true; }
                                } else {
                                    let mut parts = Vec::new();
                                    if i.modifiers.ctrl { parts.push("Ctrl"); }
                                    if i.modifiers.alt { parts.push("Alt"); }
                                    if i.modifiers.shift { parts.push("Shift"); }
                                    if i.modifiers.command { parts.push("Win"); }
                                    parts.push(key.name());
                                    sequence = parts.join("+");
                                }
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            if !sequence.is_empty() { confirm = true; }
                        }
                        if ui.button("Cancel").clicked() { close_dialog = true; }
                    });
                });

            if confirm {
                let mut workspaces = self.workspaces.lock().unwrap();
                if let Some(ws) = workspaces.get_mut(index) {
                    match ws.set_hotkey(self, &sequence) {
                        Ok(()) => self.unsaved_changes = true,
                        Err(e) => show_error_box(&e, "Hotkey Error"),
                    }
                }
            } else if !close_dialog {
                self.hotkey_dialog = Some((index, sequence));
            }
        }
    }

    /// Renders the controls for managing individual workspaces.
    ///
    /// This function provides UI elements for:
    /// - Disabling/enabling a workspace.
    /// - Moving a workspace up or down in the list.
    /// - Deleting a workspace with confirmation.
    ///
    /// # Behavior
    /// - Displays a checkbox for toggling the workspace's "disabled" state.
    /// - Provides buttons to move the workspace up or down in the list.
    /// - Provides a "Delete Workspace" button with a confirmation dialog.
    ///
    /// # Example
    /// ```rust
    /// let mut context = WorkspaceControlContext {
    ///     workspace_to_delete: &mut None,
    ///     move_up_index: &mut None,
    ///     move_down_index: &mut None,
    ///     workspaces_len: 3,
    ///     index: 1,
    /// };
    /// app.render_workspace_controls(ui, &mut workspace, &mut context);
    /// ```
    ///
    /// # Parameters
    /// - `ui: &mut egui::Ui`: The UI context for rendering the controls.
    /// - `workspace: &mut Workspace`: A mutable reference to the workspace being managed.
    /// - `context: &mut WorkspaceControlContext`: A struct containing metadata and state for managing the workspace.
    ///
    /// # Side Effects
    /// - Updates the workspace's `disabled` state.
    /// - Modifies the context's `workspace_to_delete`, `move_up_index`, or `move_down_index` based on user actions.
    ///
    /// # Notes
    /// - Disabling a workspace prevents it from being activated via hotkeys.
    /// - Moving a workspace up or down affects its order in the workspace list.
    /// - The "Delete Workspace" button requires user confirmation and updates the `workspace_to_delete` context.
    fn render_workspace_controls(
        &self,
        ui: &mut egui::Ui,
        workspace: &mut Workspace,
        context: &mut WorkspaceControlContext,
    ) -> bool {
        let mut changed = false;
        // Workspace disable checkbox
        ui.horizontal(|ui| {
            if ui.checkbox(&mut workspace.disabled, "Disable Workspace").changed() {
                changed = true;
            }

            if ui.button("Delete Workspace").clicked() {
                let confirmation_message = format!(
                    "Are you sure you want to delete workspace '{}'? This action cannot be undone.",
                    &workspace.name
                );
                if show_confirmation_box(&confirmation_message, "Confirm Deletion") {
                    *context.workspace_to_delete = Some(context.index);
                    changed = true;
                }
            }
        });

        ui.horizontal(|ui| {
            if context.index > 0 && ui.button("Move ⏶").clicked() {
                *context.move_up_index = Some(context.index);
                changed = true;
            }
            if context.index < context.workspaces_len - 1 && ui.button("Move ⏷").clicked() {
                *context.move_down_index = Some(context.index);
                changed = true;
            }
        });

        changed
    }

    /// Saves the current list of workspaces to a JSON file.
    ///
    /// This function serializes the list of workspaces and writes it to the specified file.
    /// It is typically called when the "Save Workspaces" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Serializes the `workspaces` into a JSON string using `serde_json`.
    /// - Writes the serialized data to `workspaces.json`.
    /// - Logs a success message upon completion.
    ///
    /// # Example
    /// ```rust
    /// app.save_workspaces();
    /// ```
    ///
    /// # Side Effects
    /// - Creates or overwrites the `workspaces.json` file with the current state of the workspaces.
    ///
    /// # Notes
    /// - This function relies on the `serde_json` crate for serialization.
    /// - Errors during file creation or writing are logged but not returned.
    ///
    /// # Dependencies
    /// - Calls `save_workspaces` function in `workspace.rs` for actual file operations.
    ///
    /// # Logs
    /// - Logs a message when the workspaces are successfully saved.
    /// - Logs an error message if file creation or writing fails.
    fn save_workspaces(&mut self) {
        let default_path = self
            .last_workspace_file
            .clone()
            .unwrap_or_else(|| "workspaces.json".to_string());
        self.save_workspaces_to_file(&default_path);
    }

    /// Save workspaces to the specified path and persist the choice.
    pub fn save_workspaces_to_file(&mut self, path: &str) {
        let workspaces = self.workspaces.lock().unwrap();
        save_workspaces(&workspaces, path);
        self.last_workspace_file = Some(path.to_string());
        self.unsaved_changes = false;
        info!("Workspaces saved successfully.");
        save_settings(&Settings {
            save_on_exit: self.save_on_exit,
            auto_save: self.auto_save,
            log_level: self.log_level.clone(),
            last_layout_file: self.last_layout_file.clone(),
            last_workspace_file: self.last_workspace_file.clone(),
            developer_debugging: self.developer_debugging,
        });
    }

    /// Adds a new workspace to the list of workspaces.
    ///
    /// This function appends a new `Workspace` instance to the list.
    /// Typically used when the "Add New Workspace" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Locks the `workspaces` mutex to modify the list.
    /// - Adds the provided `Workspace` to the end of the list.
    ///
    /// # Example
    /// ```rust
    /// let new_workspace = Workspace {
    ///     name: "New Workspace".to_string(),
    ///     hotkey: None,
    ///     windows: Vec::new(),
    ///     disabled: false,
    ///     valid: false,
    /// };
    /// app.add_workspace(new_workspace);
    /// ```
    ///
    /// # Parameters
    /// - `workspace: Workspace`: The workspace instance to be added.
    ///
    /// # Side Effects
    /// - Modifies the `workspaces` list by adding a new workspace.
    ///
    /// # Notes
    /// - The function does not perform any validation or registration of hotkeys for the new workspace.
    /// - Any changes made to the workspace list are not persisted to disk until `save_workspaces` is called.
    fn add_workspace(&mut self, workspace: Workspace) {
        let mut workspaces = self.workspaces.lock().unwrap();
        workspaces.push(workspace);
        self.unsaved_changes = true;
    }

    /// Deletes a workspace from the list by its index.
    ///
    /// This function removes a workspace from the `workspaces` list, typically called
    /// when the "Delete Workspace" button is clicked in the GUI.
    ///
    /// # Behavior
    /// - Locks the `workspaces` mutex to modify the list.
    /// - Removes the workspace at the specified index from the list.
    ///
    /// # Parameters
    /// - `index: usize`: The zero-based index of the workspace to delete.
    ///
    /// # Example
    /// ```rust
    /// app.delete_workspace(2);
    /// ```
    ///
    /// # Side Effects
    /// - Modifies the `workspaces` list by removing the specified workspace.
    /// - Any changes made to the workspace list are not persisted to disk until `save_workspaces` is called.
    ///
    /// # Notes
    /// - If the `index` is out of bounds, the function will panic as it directly calls `Vec::remove`.
    /// - If the workspace has a registered hotkey, it will be unregistered before removal.
    ///
    /// # Error Conditions
    /// - Panics if the `index` is greater than or equal to the length of the `workspaces` list.
    fn delete_workspace(&mut self, index: usize) {
        let mut workspaces = self.workspaces.lock().unwrap();
        if let Some(workspace) = workspaces.get_mut(index) {
            if let Some(ref hotkey) = workspace.hotkey {
                hotkey.unregister(self);
            }
        }
        workspaces.remove(index);
        self.unsaved_changes = true;
    }

    /// Displays the settings window when `self.show_settings` is `true`.
    ///
    /// The window allows configuration of global application preferences.
    fn render_settings_window(&mut self, ctx: &egui::Context) {
        let center = ctx.available_rect().center();
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(center)
            .show(ctx, |ui| {
                let response = ui.checkbox(&mut self.save_on_exit, "Save on exit");
                if response.changed() {
                    save_settings(&Settings {
                        save_on_exit: self.save_on_exit,
                        auto_save: self.auto_save,
                        log_level: self.log_level.clone(),
                        last_layout_file: None,
                        last_workspace_file: self.last_workspace_file.clone(),
                        developer_debugging: self.developer_debugging,
                    });
                }
                let auto_response = ui.checkbox(&mut self.auto_save, "Auto-save");
                if auto_response.changed() {
                    save_settings(&Settings {
                        save_on_exit: self.save_on_exit,
                        auto_save: self.auto_save,
                        log_level: self.log_level.clone(),
                        last_layout_file: self.last_layout_file.clone(),
                        last_workspace_file: self.last_workspace_file.clone(),
                        developer_debugging: self.developer_debugging,
                    });
                }
                let dev_response = ui.checkbox(&mut self.developer_debugging, "Developer Debugging");
                if dev_response.changed() {
                    save_settings(&Settings {
                        save_on_exit: self.save_on_exit,
                        auto_save: self.auto_save,
                        log_level: self.log_level.clone(),
                        last_layout_file: self.last_layout_file.clone(),
                        last_workspace_file: self.last_workspace_file.clone(),
                        developer_debugging: self.developer_debugging,
                    });
                }
                let mut changed = false;
                egui::ComboBox::from_label("Log Level")
                    .selected_text(&self.log_level)
                    .show_ui(ui, |ui| {
                        for lvl in ["trace", "debug", "info", "warn", "error", "off"] {
                            if ui.selectable_value(&mut self.log_level, lvl.to_string(), lvl).clicked() {
                                changed = true;
                            }
                        }
                    });
                if changed {
                    save_settings(&Settings {
                        save_on_exit: self.save_on_exit,
                        auto_save: self.auto_save,
                        log_level: self.log_level.clone(),
                        last_layout_file: self.last_layout_file.clone(),
                        last_workspace_file: self.last_workspace_file.clone(),
                        developer_debugging: self.developer_debugging,
                    });
                }
                let mut path = self.last_layout_file.clone().unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label("Layout file:");
                    if ui.text_edit_singleline(&mut path).changed() {
                        if path.trim().is_empty() {
                            self.last_layout_file = None;
                        } else {
                            self.last_layout_file = Some(path.clone());
                        }
                        save_settings(&Settings {
                            save_on_exit: self.save_on_exit,
                            auto_save: self.auto_save,
                            log_level: self.log_level.clone(),
                            last_layout_file: self.last_layout_file.clone(),
                            last_workspace_file: self.last_workspace_file.clone(),
                            developer_debugging: self.developer_debugging,
                        });
                    }
                });
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
    }

    /// Sends every window in all workspaces back to its configured home position.
    fn send_all_home(&self) {
        let (actionable_workspaces, actionable_count) = {
            let workspaces = self.workspaces.lock().unwrap();
            let mut actionable_count = 0usize;
            let mut collected = Vec::new();

            for workspace in workspaces.iter() {
                let mut workspace_snapshot = workspace.clone();
                workspace_snapshot.windows.retain(|window| {
                    let hwnd = HWND(window.id as *mut c_void);
                    let is_valid = unsafe { IsWindow(hwnd).as_bool() };
                    if is_valid {
                        actionable_count += 1;
                    }
                    is_valid
                });

                if !workspace_snapshot.windows.is_empty() {
                    collected.push(workspace_snapshot);
                }
            }

            (collected, actionable_count)
        };

        if actionable_count == 0 {
            info!("Send-all-home requested, but no valid windows were found.");
            show_message_box(
                "No captured windows are currently available to send home.",
                "Send Windows Home",
            );
            return;
        }

        send_all_windows_home(&actionable_workspaces);
    }

    /// Open the folder containing `multi_manager.log` using Windows Explorer.
    fn open_log_folder(&self) {
        use std::path::PathBuf;
        use std::process::Command;

        let log_path = std::fs::canonicalize("multi_manager.log")
            .unwrap_or_else(|_| PathBuf::from("multi_manager.log"));

        if let Err(e) = Command::new("explorer").arg(&log_path).spawn() {
            show_error_box(&format!("Failed to open log folder: {}", e), "Error");
        }
    }

    /// Begin recapturing all windows across every workspace.
    fn start_recapture_all(&mut self) {
        self.recapture_queue.clear();
        let workspaces = self.workspaces.lock().unwrap();
        for (wi, ws) in workspaces.iter().enumerate() {
            for (wj, _w) in ws.windows.iter().enumerate() {
                self.recapture_queue.push((wi, wj));
            }
        }
        self.recapture_active = !self.recapture_queue.is_empty();
        // Clear any pending key state to avoid accidental captures
        let _ = poll_recapture_keys();
    }

    /// Handle the recapture-all workflow, displaying a floating panel and
    /// updating window handles when the user confirms.
    fn process_recapture_all(&mut self, ctx: &egui::Context) {
        if !self.recapture_active {
            return;
        }

        if let Some(&(ws_idx, win_idx)) = self.recapture_queue.first() {
            let (ws_name, win_title) = {
                let workspaces = self.workspaces.lock().unwrap();
                let ws_name = workspaces
                    .get(ws_idx)
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                let win_title = workspaces
                    .get(ws_idx)
                    .and_then(|w| w.windows.get(win_idx))
                    .map(|w| w.title.clone())
                    .unwrap_or_default();
                (ws_name, win_title)
            };

            egui::Window::new("Recapture All")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 20.0])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Recapturing workspace '{}' window '{}'",
                        ws_name, win_title
                    ));
                    ui.label("Focus the desired window and press Enter to capture, 'S' to skip, or Esc to cancel.");
                });

            if let Some(action) = poll_recapture_keys() {
                match action {
                    RecaptureAction::Confirm => {
                        if let Some((hwnd, title)) = get_active_window() {
                            let mut workspaces = self.workspaces.lock().unwrap();
                            if let Some(ws) = workspaces.get_mut(ws_idx) {
                                if let Some(win) = ws.windows.get_mut(win_idx) {
                                    win.id = hwnd.0 as usize;
                                    win.title = title;
                                    win.valid = true;
                                    self.unsaved_changes = true;
                                }
                            }
                        }
                        self.recapture_queue.remove(0);
                    }
                    RecaptureAction::Skip => {
                        self.recapture_queue.remove(0);
                    }
                    RecaptureAction::Cancel => {
                        self.recapture_queue.clear();
                        self.recapture_active = false;
                    }
                }
            }

            if self.recapture_queue.is_empty() {
                self.recapture_active = false;
            }

            if !self.recapture_active {
                let _ = poll_recapture_keys();
            }

            ctx.request_repaint();
        } else {
            self.recapture_active = false;
            let _ = poll_recapture_keys();
        }
    }

    /// Validates and registers hotkeys for all workspaces during initialization.
    fn validate_initial_hotkeys(&self) {
        let mut initial_validation_done = self.initial_validation_done.lock().unwrap();
        if !*initial_validation_done {
            let mut workspaces = self.workspaces.lock().unwrap();
            for (i, workspace) in workspaces.iter_mut().enumerate() {
                if workspace.disabled {
                    continue;
                }
                if let Some(ref mut hotkey) = workspace.hotkey {
                    if !hotkey.register(self, i as i32) {
                        warn!(
                            "Failed to register hotkey '{}' for workspace '{}'",
                            hotkey, workspace.name
                        );
                    }
                }
            }
            *initial_validation_done = true;
        }
    }

    /// Load workspaces from the specified file, replacing current ones.
    pub fn load_workspaces_from_file(&mut self, path: &str) {
        {
            let mut workspaces = self.workspaces.lock().unwrap();
            for ws in workspaces.iter_mut() {
                if let Some(ref hotkey) = ws.hotkey {
                    hotkey.unregister(self);
                }
            }
        }

        {
            let mut workspaces = self.workspaces.lock().unwrap();
            *workspaces = load_workspaces(path, self);
        }

        self.last_workspace_file = Some(path.to_string());
        self.unsaved_changes = false;
        save_settings(&Settings {
            save_on_exit: self.save_on_exit,
            auto_save: self.auto_save,
            log_level: self.log_level.clone(),
            last_layout_file: self.last_layout_file.clone(),
            last_workspace_file: self.last_workspace_file.clone(),
            developer_debugging: self.developer_debugging,
        });
    }
}
