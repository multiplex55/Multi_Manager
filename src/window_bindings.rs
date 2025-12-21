use crate::workspace::Workspace;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

/// Describes a collection of saved window handles for a specific workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceBindingSnapshot {
    pub workspace_index: usize,
    pub workspace_name: String,
    pub windows: Vec<WindowBindingSnapshot>,
}

/// Represents a saved window binding that can be re-applied later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowBindingSnapshot {
    pub window_index: usize,
    pub window_title: String,
    pub hwnd: usize,
}

/// Aggregated statistics describing the result of applying saved bindings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BindingApplicationStats {
    pub restored: usize,
    pub invalidated: usize,
    pub unmatched: usize,
}

/// Errors that can occur when saving or loading bindings.
#[derive(Debug)]
pub enum WindowBindingError {
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl fmt::Display for WindowBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowBindingError::Io(err) => write!(f, "I/O error: {}", err),
            WindowBindingError::Serialize(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl Error for WindowBindingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WindowBindingError::Io(err) => Some(err),
            WindowBindingError::Serialize(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for WindowBindingError {
    fn from(err: std::io::Error) -> Self {
        WindowBindingError::Io(err)
    }
}

impl From<serde_json::Error> for WindowBindingError {
    fn from(err: serde_json::Error) -> Self {
        WindowBindingError::Serialize(err)
    }
}

/// Serialize the currently valid window handles for each workspace to a JSON file.
pub fn save_window_bindings(
    workspaces: &[Workspace],
    path: &str,
) -> Result<usize, WindowBindingError> {
    let mut snapshots = Vec::new();
    let mut saved_handles = 0usize;

    for (workspace_index, workspace) in workspaces.iter().enumerate() {
        let mut windows = Vec::new();

        for (window_index, window) in workspace.windows.iter().enumerate() {
            let hwnd = HWND(window.id as *mut c_void);
            let is_valid = unsafe { IsWindow(hwnd).as_bool() };

            if is_valid {
                windows.push(WindowBindingSnapshot {
                    window_index,
                    window_title: window.title.clone(),
                    hwnd: window.id,
                });
                saved_handles += 1;
            } else {
                warn!(
                    "Skipping invalid window '{}' while saving bindings for workspace '{}'",
                    window.title, workspace.name
                );
            }
        }

        if !windows.is_empty() {
            snapshots.push(WorkspaceBindingSnapshot {
                workspace_index,
                workspace_name: workspace.name.clone(),
                windows,
            });
        }
    }

    let json = serde_json::to_string_pretty(&snapshots)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    info!(
        "Saved {} window handle{} across {} workspace{} to '{}'",
        saved_handles,
        if saved_handles == 1 { "" } else { "s" },
        snapshots.len(),
        if snapshots.len() == 1 { "" } else { "s" },
        path
    );

    Ok(saved_handles)
}

/// Load previously saved window bindings from disk.
pub fn load_window_bindings(
    path: &str,
) -> Result<Vec<WorkspaceBindingSnapshot>, WindowBindingError> {
    let mut content = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut content)?;
    let bindings = serde_json::from_str::<Vec<WorkspaceBindingSnapshot>>(&content)?;
    Ok(bindings)
}

/// Apply previously saved window bindings to the provided workspaces.
pub fn apply_window_bindings(
    workspaces: &mut [Workspace],
    bindings: &[WorkspaceBindingSnapshot],
) -> BindingApplicationStats {
    let mut stats = BindingApplicationStats::default();

    for binding in bindings {
        let workspace_idx = workspaces
            .get(binding.workspace_index)
            .and_then(|ws| {
                if ws.name == binding.workspace_name {
                    Some(binding.workspace_index)
                } else {
                    None
                }
            })
            .or_else(|| {
                workspaces
                    .iter()
                    .position(|ws| ws.name == binding.workspace_name)
            });

        let Some(workspace_idx) = workspace_idx else {
            stats.unmatched += binding.windows.len();
            warn!(
                "No matching workspace found for saved bindings '{}' (index {}).",
                binding.workspace_name, binding.workspace_index
            );
            continue;
        };

        let workspace = &mut workspaces[workspace_idx];

        for window_binding in &binding.windows {
            let target_index = if window_binding.window_index < workspace.windows.len() {
                let mut index = Some(window_binding.window_index);

                if workspace.windows[window_binding.window_index].title
                    != window_binding.window_title
                {
                    index = workspace
                        .windows
                        .iter()
                        .position(|w| w.title == window_binding.window_title);
                }

                index
            } else {
                workspace
                    .windows
                    .iter()
                    .position(|w| w.title == window_binding.window_title)
            };

            let Some(index) = target_index else {
                stats.unmatched += 1;
                warn!(
                    "No matching window found for '{}' in workspace '{}'.",
                    window_binding.window_title, workspace.name
                );
                continue;
            };

            if let Some(window) = workspace.windows.get_mut(index) {
                let hwnd = HWND(window_binding.hwnd as *mut c_void);
                let is_valid = unsafe { IsWindow(hwnd).as_bool() };
                if is_valid {
                    window.id = window_binding.hwnd;
                    window.valid = true;
                    stats.restored += 1;
                } else {
                    window.valid = false;
                    stats.invalidated += 1;
                    warn!(
                        "Saved handle for '{}' in workspace '{}' is no longer valid.",
                        window.title, workspace.name
                    );
                }
            } else {
                stats.unmatched += 1;
            }
        }
    }

    stats
}
