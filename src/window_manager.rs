use crate::gui::App;
use crate::workspace::Workspace;
use crate::utils::{show_confirmation_box, show_message_box};
use log::{info, warn, debug};
use std::time::Instant;
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Determines if a given hotkey combination is currently being pressed on the keyboard.
///
/// # Behavior
/// - Splits the `key_sequence` (e.g. `"Ctrl+Alt+H"`) by `'+'`.
/// - Interprets certain tokens (`"ctrl"`, `"alt"`, `"shift"`, `"win"`) as modifier keys, checking each modifier’s state via `GetAsyncKeyState`.
/// - Identifies the main key (e.g. `"H"`) from `virtual_key_from_string(...)`.
/// - Returns `true` if **all** modifiers **and** the main key are pressed simultaneously, else `false`.
///
/// # Side Effects
/// - Uses the Win32 API call [`GetAsyncKeyState`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getasynckeystate) to check the state of each key (only valid on Windows).
/// - If `virtual_key_from_string` fails (unknown key), the function returns `false`.
///
/// # Example
/// ```no_run
/// if is_hotkey_pressed("Ctrl+Shift+P") {
///     println!("Ctrl+Shift+P is currently held down!");
/// }
/// ```
///
/// # Notes
/// - This function checks **live** key states; it should be polled in a loop or a timer if you’re monitoring for repeated presses.
/// - Frequently used inside the main hotkey checking loop (`check_hotkeys`).
/// - Case-insensitive for the tokens `Ctrl`, `Alt`, `Shift`, `Win`.
pub fn is_hotkey_pressed(key_sequence: &str) -> bool {
    let mut modifiers_pressed = true;
    let mut vk_code: Option<u32> = None;

    for part in key_sequence.split('+') {
        match part.to_lowercase().as_str() {
            "ctrl" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
            },
            "alt" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_MENU.0 as i32) < 0;
            },
            "shift" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
            },
            "win" => unsafe {
                modifiers_pressed &= GetAsyncKeyState(VK_LWIN.0 as i32) < 0
                    || GetAsyncKeyState(VK_RWIN.0 as i32) < 0;
            },
            _ => vk_code = virtual_key_from_string(part),
        }
    }

    if let Some(vk) = vk_code {
        unsafe { modifiers_pressed && GetAsyncKeyState(vk as i32) < 0 }
    } else {
        false
    }
}

/// Toggles the **positions** of all windows in a `Workspace` between their **home** and **target** locations.
///
/// # Behavior
/// - Calls `are_all_windows_at_home(workspace)` to determine if every window is at its home position.
///   - If **all** are at home, the function moves each valid window to its `target`.
///   - Otherwise, it attempts to move each valid window **back to its home** position.
/// - Restores minimized windows using `move_window`, which internally
///   updates the window's restore coordinates and calls `ShowWindow`.
/// - Activates each window (via `SetForegroundWindow`) after movement completes.
///
/// # Side Effects
/// - Issues multiple Win32 API calls for restoring/minimizing, moving, and activating windows.
/// - Logs actions and warnings (e.g., if a window is invalid or fails to move).
///
/// # Example
/// ```rust
/// // If all windows are at home, move them to target; otherwise back to home.
/// toggle_workspace_windows(&mut my_workspace);
/// ```
///
/// # Notes
/// - If a window is minimized (iconic), it is restored before being positioned.
/// - Invalid windows (where `IsWindow(hwnd)` returns false) are skipped and logged with a warning.
/// - This function is often bound to a hotkey press and invoked by `check_hotkeys()`.
pub fn are_all_windows_at_home(workspace: &Workspace) -> bool {
    workspace.windows.iter().filter(|w| w.valid).all(|w| {
        let hwnd = HWND(w.id as *mut std::ffi::c_void);
        unsafe {
            IsWindow(hwnd).as_bool()
                && is_window_at_position(hwnd, w.home.0, w.home.1, w.home.2, w.home.3)
        }
    })
}

/// Toggles workspace windows between their home and target locations.
///
/// # Arguments
/// - `workspace`: The workspace to toggle windows for.
///
/// - If all windows are at their home positions, they are moved to their target positions.
/// - If any window is not at its home or target position, it is moved to its home position.
///
/// # Example
/// ```
/// toggle_workspace_windows(&mut workspace);
/// ```
pub fn toggle_workspace_windows(workspace: &mut Workspace) {
    if workspace.rotate && workspace.windows.len() > 1 {
        let len = workspace.windows.len();
        let target_idx = workspace.current_index % len;

        for (i, window) in workspace.windows.iter().enumerate() {
            let hwnd = HWND(window.id as *mut std::ffi::c_void);

            unsafe {
                if !IsWindow(hwnd).as_bool() {
                    warn!("Skipping invalid window '{}'.", window.title);
                    continue;
                }
            }

            let position = if i == target_idx { window.target } else { window.home };

            if let Err(e) = move_window(hwnd, position.0, position.1, position.2, position.3) {
                warn!("Failed to move window '{}': {}", window.title, e);
            } else {
                info!("Moved window '{}' to position: {:?}", window.title, position);
            }

            if i == target_idx {
                unsafe {
                    if SetForegroundWindow(hwnd).as_bool() {
                        info!("Activated window '{}'", window.title);
                    } else {
                        warn!("Failed to activate window '{}'", window.title);
                    }
                }
            }
        }
        workspace.current_index = (workspace.current_index + 1) % len;
    } else {
        let all_at_home = are_all_windows_at_home(workspace);
        debug!("all_at_home={}", all_at_home);

        for window in &workspace.windows {
            let hwnd = HWND(window.id as *mut std::ffi::c_void);

            unsafe {
                if !IsWindow(hwnd).as_bool() {
                    warn!("Skipping invalid window '{}'.", window.title);
                    continue;
                }
            }

            let target_position = if all_at_home { window.target } else { window.home };

            if let Err(e) = move_window(
                hwnd,
                target_position.0,
                target_position.1,
                target_position.2,
                target_position.3,
            ) {
                warn!("Failed to move window '{}': {}", window.title, e);
            } else {
                info!("Moved window '{}' to position: {:?}", window.title, target_position);
            }

            unsafe {
                if SetForegroundWindow(hwnd).as_bool() {
                    info!("Activated window '{}'", window.title);
                } else {
                    warn!("Failed to activate window '{}'", window.title);
                }
            }
        }
    }
}

/// Moves all valid windows in a `Workspace` to their defined **home** positions.
///
/// # Behavior
/// - Restores minimized windows automatically when moved.
/// - Uses [`move_window`](fn.move_window.html) to reposition each window.
/// - Attempts to activate each window after it has been moved.
///
/// # Parameters
/// - `workspace`: The workspace whose windows should be returned home.
pub fn send_workspace_windows_home(workspace: &Workspace) {
    for window in &workspace.windows {
        let hwnd = HWND(window.id as *mut std::ffi::c_void);

        unsafe {
            if !IsWindow(hwnd).as_bool() {
                warn!(
                    "Skipping invalid window '{}' in workspace '{}'.",
                    window.title, workspace.name
                );
                continue;
            }

        }

        if let Err(e) = move_window(hwnd, window.home.0, window.home.1, window.home.2, window.home.3) {
            warn!("Failed to move window '{}': {}", window.title, e);
        } else {
            info!("Moved window '{}' to home position: {:?}", window.title, window.home);
        }

        unsafe {
            if SetForegroundWindow(hwnd).as_bool() {
                info!("Activated window '{}'", window.title);
            } else {
                warn!("Failed to activate window '{}'", window.title);
            }
        }
    }
}

/// Iterates over every workspace and sends each of their windows to the `home` position.
pub fn send_all_windows_home(workspaces: &mut [Workspace]) {
    for workspace in workspaces.iter() {
        send_workspace_windows_home(workspace);
    }
}

use crate::desktop_window_info::DesktopWindowInfo;
use crate::virtual_desktop;
use std::fs::File;
use std::io::Write;
use serde_json;

/// Capture window positions for all desktops and store them as JSON.
#[cfg(target_os = "windows")]
pub fn capture_all_desktops(file: &str) {
    let mut infos: Vec<DesktopWindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_capture_proc), LPARAM(&mut infos as *mut _ as isize));
    }
    if let Ok(json) = serde_json::to_string_pretty(&infos) {
        if let Err(e) = File::create(file).and_then(|mut f| f.write_all(json.as_bytes())) {
            warn!("Failed to save desktop data: {}", e);
        } else {
            info!("Saved desktop layout to {}", file);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_capture_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let list = &mut *(lparam.0 as *mut Vec<DesktopWindowInfo>);
    if let Ok(desktop) = virtual_desktop::get_desktop_by_window(hwnd) {
        if let Ok(index) = desktop.get_index() {
            let mut buffer = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut buffer);
            let title = String::from_utf16_lossy(&buffer[..len as usize]);
            if let Ok((x, y, w, h)) = get_window_position(hwnd) {
                list.push(DesktopWindowInfo {
                    desktop_index: index,
                    hwnd: hwnd.0 as isize,
                    title,
                    rect: (x, y, w, h),
                });
            }
        }
    }
    BOOL(1)
}

/// Restore window positions across all desktops from a JSON file.
#[cfg(target_os = "windows")]
pub fn restore_all_desktops(file: &str) {
    let data = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to read {}: {}", file, e);
            return;
        }
    };
    let infos: Vec<DesktopWindowInfo> = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse {}: {}", file, e);
            return;
        }
    };
    let desktops = match virtual_desktop::get_desktops() {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to enumerate desktops: {:?}", e);
            return;
        }
    };
    let current = virtual_desktop::get_current_desktop().ok();
    for info in &infos {
        if let Some(target) = desktops.get(info.desktop_index as usize) {
            if let Err(e) = virtual_desktop::switch_desktop(target) {
                warn!("Failed to switch desktop: {:?}", e);
            }
            let hwnd = HWND(info.hwnd as *mut _);
            unsafe {
                if IsWindow(hwnd).as_bool() {
                    move_window(hwnd, info.rect.0, info.rect.1, info.rect.2, info.rect.3).ok();
                }
            }
        }
    }
    if let Some(d) = current {
        let _ = virtual_desktop::switch_desktop(&d);
    }
}

#[cfg(target_os = "windows")]
/// Helper structure passed to `EnumWindows` containing the primary monitor
/// dimensions. The enumeration callback uses these values to calculate the
/// centered coordinates for each window it visits.
struct OriginData {
    /// Width of the primary monitor in physical pixels.
    width: i32,
    /// Height of the primary monitor in physical pixels.
    height: i32,
}

#[cfg(target_os = "windows")]
/// Moves every visible top-level window so that it is centered on the primary
/// monitor. A confirmation dialog is displayed before any action is taken.
///
/// # Behavior
/// - Retrieves the primary monitor's dimensions using
///   [`GetSystemMetrics`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics).
/// - Enumerates all top-level windows via [`EnumWindows`]. For each valid and
///   visible window, the helper callback (`enum_origin_proc`) is invoked.
/// - The callback calculates the centered coordinates for the window based on
///   its size and moves it with [`move_window`].
///
/// # Side Effects
/// - Prompts the user to confirm the action.
/// - Causes all windows on screen to reposition to the center. Minimized or
///   invisible windows are ignored.
/// - Logs a message for each moved window, or a warning if the move fails.
/// - Shows a completion message once all windows have been centered.
///
/// # Example
/// ```no_run
/// move_all_to_origin(); // Centers every visible window on the primary screen
/// ```
pub fn move_all_to_origin() {
    if !show_confirmation_box(
        "Move all windows to the center of the primary monitor?",
        "Confirm",
    ) {
        return;
    }
    unsafe {
        let mut data = OriginData {
            width: GetSystemMetrics(SM_CXSCREEN),
            height: GetSystemMetrics(SM_CYSCREEN),
        };
        // Enumerate every top-level window, passing a pointer to `data` so the
        // callback can compute centered positions.
        let _ = EnumWindows(Some(enum_origin_proc), LPARAM(&mut data as *mut _ as isize));
    }
    show_message_box("All windows have been centered", "Completed");
}

#[cfg(target_os = "windows")]
/// Enumeration callback used by [`move_all_to_origin`]. For each window, it
/// determines whether the window is valid and visible and, if so, moves it to
/// the center of the primary monitor.
///
/// # Parameters
/// - `hwnd`: Handle of the current window provided by `EnumWindows`.
/// - `lparam`: Pointer to an [`OriginData`] instance containing the monitor
///   dimensions.
///
/// # Returns
/// - `BOOL(1)` to continue enumeration regardless of success or failure.
///
/// # Behavior
/// - Skips windows that are invalid or not visible.
/// - Retrieves the window's size using [`get_window_position`].
/// - Calculates centered coordinates and calls [`move_window`].
/// - Logs the outcome of the move for debugging purposes.
unsafe extern "system" fn enum_origin_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Skip invalid or hidden windows.
    if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    // Extract the screen dimensions from lparam.
    let data = &*(lparam.0 as *const OriginData);

    if let Ok((_, _, w, h)) = get_window_position(hwnd) {
        // Compute centered coordinates.
        let x = (data.width - w) / 2;
        let y = (data.height - h) / 2;
        match move_window(hwnd, x, y, w, h) {
            Ok(_) => info!("Moved window {:?} to center ({}, {})", hwnd, x, y),
            Err(e) => warn!("Failed to move window {:?}: {}", hwnd, e),
        }
    }
    BOOL(1)
}

#[cfg(target_os = "windows")]
/// Move a specific window to the center of the primary monitor.
///
/// This function validates the provided window handle, restores the window if
/// it is minimized, retrieves the current monitor size and the window's
/// dimensions, then repositions the window so it is centered on the screen.
pub fn move_window_to_origin(hwnd: HWND) {
    unsafe {
        if !IsWindow(hwnd).as_bool() {
            warn!("Invalid window handle: {:?}", hwnd);
            return;
        }

    }

    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    if let Ok((_, _, w, h)) = get_window_position(hwnd) {
        let x = (screen_width - w) / 2;
        let y = (screen_height - h) / 2;
        match move_window(hwnd, x, y, w, h) {
            Ok(_) => info!("Moved window {:?} to center ({}, {})", hwnd, x, y),
            Err(e) => warn!("Failed to move window {:?}: {}", hwnd, e),
        }
    } else {
        warn!("Failed to get position for window {:?}", hwnd);
    }
}

#[cfg(not(target_os = "windows"))]
/// Stub for non-Windows platforms.
pub fn move_window_to_origin(_hwnd: HWND) {
    warn!("move_window_to_origin is only available on Windows");
}

#[cfg(not(target_os = "windows"))]
/// Stub implementation for non-Windows platforms. Calling this function on a
/// non-Windows build logs a warning and performs no action.
pub fn move_all_to_origin() {
    warn!("move_all_to_origin is only available on Windows");
}

#[cfg(not(target_os = "windows"))]
pub fn capture_all_desktops(_file: &str) {
    warn!("capture_all_desktops is only available on Windows");
}

#[cfg(not(target_os = "windows"))]
pub fn restore_all_desktops(_file: &str) {
    warn!("restore_all_desktops is only available on Windows");
}

/// Determines whether the specified `hwnd` is currently located at the given **(x, y)** coordinates
/// with the specified **width** and **height**.
///
/// # Behavior
/// - Retrieves the window’s current position and size using
///   [`get_window_position`](#fn.get_window_position).
/// - Compares the returned `(x, y, width, height)` tuple to the provided parameters.
/// - Returns `true` if they match exactly, otherwise `false`.
///
/// # Side Effects
/// - Calls `get_window_position`, which uses the Win32 API [`GetWindowRect`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
///   to retrieve the actual window rectangle on screen.
///
/// # Example
/// ```rust
/// if is_window_at_position(hwnd, 100, 100, 800, 600) {
///     println!("The window is exactly at (100, 100) with size (800x600).");
/// } else {
///     println!("The window is not at the specified position/size.");
/// }
/// ```
///
/// # Notes
/// - If `get_window_position` fails or returns an error, this function returns `false`.
/// - Primarily used internally (e.g., in `are_all_windows_at_home`).
pub fn is_window_at_position(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> bool {
    if let Ok((wx, wy, ww, wh)) = get_window_position(hwnd) {
        wx == x && wy == y && ww == w && wh == h
    } else {
        false
    }
}

/// Retrieves the current position and size of a window.
///
/// This function uses the Win32 API `GetWindowRect` to obtain the coordinates of the window's
/// bounding rectangle. It calculates the width and height from the `RECT` structure and returns
/// the position and size of the window as a tuple.
///
/// # Behavior
/// - If the window handle (`hwnd`) is valid, the function returns a tuple of `(x, y, width, height)`.
/// - If the window handle is invalid or the call to `GetWindowRect` fails, an error is returned.
///
/// # Example
/// ```rust
/// use windows::Win32::Foundation::HWND;
///
/// let hwnd = HWND(12345 as *mut _); // Example HWND
/// match get_window_position(hwnd) {
///     Ok((x, y, w, h)) => println!("Window position: ({}, {}, {}, {})", x, y, w, h),
///     Err(err) => println!("Failed to get window position: {}", err),
/// }
/// ```
///
/// # Dependencies
/// - Calls the `GetWindowRect` function from the Win32 API to retrieve the window's rectangle.
///
/// # Returns
/// - `Ok((x, y, width, height))`: If the window's position and size are successfully retrieved.
/// - `Err(windows::core::Error)`: If the `GetWindowRect` API call fails or the window handle is invalid.
///
/// # Parameters
/// - `hwnd: HWND`: The handle to the window whose position and size are being queried.
///
/// # Side Effects
/// - None.
///
/// # Error Conditions
/// - Returns an error if the `hwnd` is invalid or the `GetWindowRect` API call fails.
///
/// # Notes
/// - Ensure the `hwnd` passed to this function is valid before calling.
///
/// # Win32 API Reference
/// - [`GetWindowRect`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
pub fn get_window_position(hwnd: HWND) -> Result<(i32, i32, i32, i32)> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Ok((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ))
        } else {
            Err(windows::core::Error::from_win32())
        }
    }
}

/// Sets the restore rectangle for a minimized window so it will
/// reappear at the specified coordinates when restored.
pub fn set_restore_position(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    unsafe {
        let mut placement = WINDOWPLACEMENT::default();
        placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        GetWindowPlacement(hwnd, &mut placement)?;
        placement.rcNormalPosition = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        SetWindowPlacement(hwnd, &placement)?;
        Ok(())
    }
}

/// Converts a textual key identifier (e.g. `"A"`, `"F1"`, `"Ctrl"`) into its corresponding Windows **virtual key code**.
///
/// # Behavior
/// - Matches the input `key` (converted to uppercase) against a predefined list of known key names (e.g. `"F1"`, `"NUMPAD0"`, `"LEFTALT"`, etc.).
/// - Returns the matching `u32` virtual key code if recognized, or `None` if the `key` is unrecognized.
/// - Handles a wide variety of function, alphanumeric, numpad, arrow, and special keys.
/// - Case-insensitive for recognized tokens.
///
/// # Side Effects
/// - None directly, but used by functions such as `is_hotkey_pressed` or `Hotkey::register` to map textual keys into numeric codes.
///
/// # Example
/// ```rust
/// if let Some(vk_code) = virtual_key_from_string("F5") {
///     println!("Virtual key code for F5 is: 0x{:X}", vk_code);
/// } else {
///     println!("Unrecognized key.");
/// }
/// ```
///
/// # Notes
/// - Missing or incorrect keys (e.g. `"F27"` or `"SomeRandomKey"`) will yield `None`.
/// - This helps unify string-based user input with the numeric values required for Win32 API calls.
pub fn virtual_key_from_string(key: &str) -> Option<u32> {
    match key.to_uppercase().as_str() {
        // Function keys
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "F4" => Some(0x73),
        "F5" => Some(0x74),
        "F6" => Some(0x75),
        "F7" => Some(0x76),
        "F8" => Some(0x77),
        "F9" => Some(0x78),
        "F10" => Some(0x79),
        "F11" => Some(0x7A),
        "F12" => Some(0x7B),
        "F13" => Some(0x7C),
        "F14" => Some(0x7D),
        "F15" => Some(0x7E),
        "F16" => Some(0x7F),
        "F17" => Some(0x80),
        "F18" => Some(0x81),
        "F19" => Some(0x82),
        "F20" => Some(0x83),
        "F21" => Some(0x84),
        "F22" => Some(0x85),
        "F23" => Some(0x86),
        "F24" => Some(0x87),

        // Alphabet keys
        "A" => Some(0x41),
        "B" => Some(0x42),
        "C" => Some(0x43),
        "D" => Some(0x44),
        "E" => Some(0x45),
        "F" => Some(0x46),
        "G" => Some(0x47),
        "H" => Some(0x48),
        "I" => Some(0x49),
        "J" => Some(0x4A),
        "K" => Some(0x4B),
        "L" => Some(0x4C),
        "M" => Some(0x4D),
        "N" => Some(0x4E),
        "O" => Some(0x4F),
        "P" => Some(0x50),
        "Q" => Some(0x51),
        "R" => Some(0x52),
        "S" => Some(0x53),
        "T" => Some(0x54),
        "U" => Some(0x55),
        "V" => Some(0x56),
        "W" => Some(0x57),
        "X" => Some(0x58),
        "Y" => Some(0x59),
        "Z" => Some(0x5A),

        // Number keys
        "0" => Some(0x30),
        "1" => Some(0x31),
        "2" => Some(0x32),
        "3" => Some(0x33),
        "4" => Some(0x34),
        "5" => Some(0x35),
        "6" => Some(0x36),
        "7" => Some(0x37),
        "8" => Some(0x38),
        "9" => Some(0x39),

        // Numpad keys
        "NUMPAD0" => Some(0x60),
        "NUMPAD1" => Some(0x61),
        "NUMPAD2" => Some(0x62),
        "NUMPAD3" => Some(0x63),
        "NUMPAD4" => Some(0x64),
        "NUMPAD5" => Some(0x65),
        "NUMPAD6" => Some(0x66),
        "NUMPAD7" => Some(0x67),
        "NUMPAD8" => Some(0x68),
        "NUMPAD9" => Some(0x69),
        "NUMPADMULTIPLY" => Some(0x6A),
        "NUMPADADD" => Some(0x6B),
        "NUMPADSEPARATOR" => Some(0x6C),
        "NUMPADSUBTRACT" => Some(0x6D),
        "NUMPADDOT" => Some(0x6E),
        "NUMPADDIVIDE" => Some(0x6F),

        // Arrow keys
        "UP" => Some(0x26),
        "DOWN" => Some(0x28),
        "LEFT" => Some(0x25),
        "RIGHT" => Some(0x27),

        // Special keys
        "BACKSPACE" => Some(0x08),
        "TAB" => Some(0x09),
        "ENTER" => Some(0x0D),
        "SHIFT" => Some(0x10),
        "CTRL" => Some(0x11),
        "ALT" => Some(0x12),
        "PAUSE" => Some(0x13),
        "CAPSLOCK" => Some(0x14),
        "ESCAPE" => Some(0x1B),
        "SPACE" => Some(0x20),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "END" => Some(0x23),
        "HOME" => Some(0x24),
        "INSERT" => Some(0x2D),
        "DELETE" => Some(0x2E),

        // Symbols
        "OEM_PLUS" => Some(0xBB),   // '+' key
        "OEM_COMMA" => Some(0xBC),  // ',' key
        "OEM_MINUS" => Some(0xBD),  // '-' key
        "OEM_PERIOD" => Some(0xBE), // '.' key
        "OEM_1" => Some(0xBA),      // ';:' key
        "OEM_2" => Some(0xBF),      // '/?' key
        "OEM_3" => Some(0xC0),      // '`~' key
        "OEM_4" => Some(0xDB),      // '[{' key
        "OEM_5" => Some(0xDC),      // '\|' key
        "OEM_6" => Some(0xDD),      // ']}' key
        "OEM_7" => Some(0xDE),      // ''"' key

        // Additional keys
        "PRINTSCREEN" => Some(0x2C),
        "SCROLLLOCK" => Some(0x91),
        "NUMLOCK" => Some(0x90),
        "LEFTSHIFT" => Some(0xA0),
        "RIGHTSHIFT" => Some(0xA1),
        "LEFTCTRL" => Some(0xA2),
        "RIGHTCTRL" => Some(0xA3),
        "LEFTALT" => Some(0xA4),
        "RIGHTALT" => Some(0xA5),

        _ => None,
    }
}

/// Retrieves the **currently active window** (foreground window) along with its **title**.
///
/// # Behavior
/// - Calls the Win32 API function [`GetForegroundWindow`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow)
///   to get a handle (`HWND`) to the active window.
/// - If the call returns a valid window handle, it then retrieves the window’s title text via
///   [`GetWindowTextW`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextw).
/// - Converts the raw UTF-16 buffer into a Rust `String`.
/// - Returns `Some((HWND, String))` if successful, or `None` if no active window is found.
///
/// # Side Effects
/// - Accesses the Windows API for window management, which is only available on Windows.
/// - Logs a warning if no active window is detected (using the `log` crate).
///
/// # Example
/// ```no_run
/// if let Some((hwnd, title)) = get_active_window() {
///     println!("Active window is: '{}' (HWND: {:?})", title, hwnd);
/// } else {
///     println!("No active window detected.");
/// }
/// ```
///
/// # Error Conditions
/// - Returns `None` if `GetForegroundWindow` returns a eull pointer (no active window).
///
/// # Notes
/// - The window title length is capped at 256 characters in this function due to the fixed buffer size.
/// - Only intended for use in a Windows environment.
pub fn get_active_window() -> Option<(HWND, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            warn!("No active window detected.");
            None
        } else {
            let mut buffer = [0u16; 256];
            let length = GetWindowTextW(hwnd, &mut buffer);
            let title = String::from_utf16_lossy(&buffer[..length as usize]);
            info!("Active window detected: '{}'.", title);
            Some((hwnd, title))
        }
    }
}

/// Repositions and resizes a window identified by `hwnd` to the coordinates `(x, y)` with dimensions `(w, h)`.
///
/// # Behavior
/// - Invokes the Win32 API call [`SetWindowPos`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
///   to move the specified window to the new location and size.
/// - If the window is minimized, [`set_restore_position`](fn.set_restore_position.html)
///   updates its stored coordinates, then the window is restored and positioned
///   with `SetWindowPos`.
/// - Maintains the window’s Z-order by using `SWP_NOZORDER`.
/// - Returns `Ok(())` if the operation succeeds, or a Windows error wrapped in `Err` on failure.
///
/// # Side Effects
/// - Logs the action using the `log` crate (`info!`), including the new position and size.
/// - Potentially changes the visible arrangement of the desktop if the window is on screen.
///
/// # Example
/// ```no_run
/// if let Err(e) = move_window(hwnd, 100, 100, 800, 600) {
///     eprintln!("Could not move the window: {}", e);
/// }
/// ```
///
/// # Error Conditions
/// - Returns a `windows::core::Error` if the underlying Win32 call fails (e.g., invalid `HWND`).
///
/// # Notes
/// - Typically called within `toggle_workspace_windows` and during manual “Move to Home/Target” user actions.
/// - Only valid on Windows, where `SetWindowPos` is available.
pub fn move_window(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            set_restore_position(hwnd, x, y, w, h)?;
            ShowWindow(hwnd, SW_RESTORE);
            SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER)?;
        } else {
            SetWindowPos(hwnd, HWND_TOP, x, y, w, h, SWP_NOZORDER)?;
        }
        info!(
            "Moved window (HWND: {:?}) to position ({}, {}, {}, {}).",
            hwnd.0, x, y, w, h
        );
        Ok(())
    }
}

/// Displays a dialog message prompting the user to press **Enter** or **Esc**, and upon Enter,
/// also retrieves the **currently active window** (its handle and title).
///
/// # Behavior
/// - Calls `MessageBoxW` with an informational prompt:  
///   _“Press Enter to confirm or Escape to cancel.”_
/// - Loops, polling key states via `GetAsyncKeyState`.
/// - If the **Enter** key is detected:
///   - Fetches the active window with [`get_active_window`](#fn.get_active_window).
///   - Returns `Some(("Enter", HWND, String))` if an active window exists.
///   - If no active window is found, returns `None`.
/// - If the **Esc** key is detected, returns `None`.
///
/// # Side Effects
/// - Blocks execution until the user presses Enter or Escape.
/// - Shows a Windows dialog box and reads from the OS key state in real time.
/// - Logs a warning if no active window is found when Enter is pressed.
///
/// # Example
/// ```no_run
/// if let Some((action, hwnd, title)) = listen_for_keys_with_dialog_and_window() {
///     match action {
///         "Enter" => println!("Window '{}' (HWND: {:?}) captured!", title, hwnd),
///         _       => println!("Unexpected action!"),
///     }
/// } else {
///     println!("User canceled or no active window found.");
/// }
/// ```
///
/// # Notes
/// - Only available on Windows, as it relies on the native message box and key state APIs.
/// - Useful for quickly capturing which window is active at the moment the user presses Enter.
pub fn listen_for_keys_with_dialog() -> Option<&'static str> {
    unsafe {
        let message = "Press Enter to confirm or Escape to cancel.";
        MessageBoxW(
            None,
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Action Required"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );

        loop {
            if GetAsyncKeyState(VK_RETURN.0 as i32) < 0 {
                info!("Enter key detected.");
                return Some("Enter");
            }
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 {
                warn!("Escape key detected.");
                return Some("Esc");
            }
        }
    }
}

/// Periodically checks for **pressed hotkeys** across all workspaces and toggles the associated workspace windows if matched.
///
/// # Behavior
/// - Locks the `workspaces` from the `app` to iterate over each `Workspace`.
/// - Skips any workspace that is marked `disabled`.
/// - For each workspace with a valid `hotkey`, calls `is_hotkey_pressed(...)`.
///   - If true, **collects** that workspace’s index in a local list (`workspaces_to_toggle`).
/// - After releasing the lock, toggles windows for each collected workspace via `toggle_workspace_windows(...)`.
/// - Updates `last_hotkey_info` for any triggered hotkey, capturing the sequence and a timestamp.
///
/// # Side Effects
/// - May call Win32 API functions through `is_hotkey_pressed` (for checking key states) and `toggle_workspace_windows` (for re-positioning windows).
/// - Logs details about which hotkey was activated.
/// - Typically runs in a background thread loop (`Promise::spawn_thread`), sleeping a bit between checks.
///
/// # Example
/// ```no_run
/// // In a loop or thread, we might do:
/// loop {
///     check_hotkeys(&app);
///     std::thread::sleep(std::time::Duration::from_millis(100));
/// }
/// ```
///
/// # Notes
/// - This function is central to the application’s hotkey-based workspace toggling.
/// - Must be invoked repeatedly (e.g., via a timed loop) to capture newly pressed keys.
pub fn check_hotkeys(app: &App) {
    let mut workspaces_to_toggle = Vec::new();
    let workspaces = app.workspaces.lock().unwrap();

    for (i, workspace) in workspaces.iter().enumerate() {
        if workspace.disabled {
            continue;
        }

        if let Some(ref hotkey) = workspace.hotkey {
            if is_hotkey_pressed(&hotkey.key_sequence) {
                workspaces_to_toggle.push(i);
                let mut last_hotkey_info = app.last_hotkey_info.lock().unwrap();
                *last_hotkey_info = Some((hotkey.key_sequence.clone(), Instant::now()));
            }
        }
    }

    drop(workspaces); // Release lock before toggling

    let mut workspaces = app.workspaces.lock().unwrap();
    for index in workspaces_to_toggle {
        if let Some(workspace) = workspaces.get_mut(index) {
            toggle_workspace_windows(workspace);
        }
    }
}

pub fn listen_for_keys_with_dialog_and_window() -> Option<(&'static str, HWND, String)> {
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(
                "Press Enter to confirm or Escape to cancel."
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            PCWSTR(
                "Action Required"
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );

        loop {
            if GetAsyncKeyState(VK_RETURN.0 as i32) < 0 {
                if let Some((hwnd, title)) = get_active_window() {
                    return Some(("Enter", hwnd, title));
                }
                break;
            }
            if GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 {
                break;
            }
        }
    }
    None
}

/// Poll for Enter or Escape key presses globally without showing a dialog.
///
/// Returns `Some(true)` if Enter was pressed, `Some(false)` if Escape was
/// pressed, or `None` if neither key was pressed since the last call.
#[cfg(target_os = "windows")]
pub fn poll_recapture_keys() -> Option<bool> {
    unsafe {
        if GetAsyncKeyState(VK_RETURN.0 as i32) & 1 != 0 {
            return Some(true);
        }
        if GetAsyncKeyState(VK_ESCAPE.0 as i32) & 1 != 0 {
            return Some(false);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn poll_recapture_keys() -> Option<bool> {
    None
}
