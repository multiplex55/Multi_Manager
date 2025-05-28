use crate::workspace::is_valid_key_combo;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fmt;
use windows::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey;
use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;
use windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS;

#[derive(Clone, Serialize, Deserialize)]
pub struct Hotkey {
    pub key_sequence: String,
    #[serde(skip)]
    pub id: Option<i32>, // Optional ID used for registering the hotkey
}
impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key_sequence)
    }
}

impl Hotkey {
    /// Constructs a new `Hotkey` from the provided `key_sequence`, validating it to ensure
    /// it represents a **valid** key combination.
    ///
    /// # Behavior
    /// - Checks if the provided `key_sequence` (e.g. `"Ctrl+Alt+H"`) is valid by calling
    ///   [`is_valid_key_combo`](../workspace/fn.is_valid_key_combo.html).
    /// - If valid, returns `Ok(Hotkey { key_sequence, id: None })`.
    /// - If invalid, returns `Err(...)` with a descriptive error message.
    ///
    /// # Side Effects
    /// - None directly; this function only creates an in-memory `Hotkey` object.
    /// - To actually register the hotkey with the operating system, call [`Hotkey::register`](#method.register).
    ///
    /// # Example
    /// ```rust
    /// match Hotkey::new("Ctrl+Shift+P") {
    ///     Ok(hotkey) => println!("Valid hotkey created: {}", hotkey.key_sequence),
    ///     Err(e)     => eprintln!("Failed to create hotkey: {}", e),
    /// }
    /// ```
    ///
    /// # Error Conditions
    /// - Returns an error if `key_sequence` fails the `is_valid_key_combo` check (e.g., unknown key part).
    ///
    /// # Notes
    /// - This constructor does not attempt to register the hotkey; it only initializes the structure.
    /// - The `id` field defaults to `None` until `register(...)` is successfully called.
    pub fn new(key_sequence: &str) -> Result<Self, String> {
        if is_valid_key_combo(key_sequence) {
            Ok(Self {
                key_sequence: key_sequence.to_string(),
                id: None,
            })
        } else {
            Err(format!("Invalid hotkey: '{}'", key_sequence))
        }
    }

    /// Registers this `Hotkey` with the **global** Windows hotkey system, binding it to the given `id`.
    ///
    /// # Behavior
    /// - Parses the `key_sequence` into modifier flags (`Ctrl`, `Alt`, `Shift`, `Win`) and a main virtual key using [`virtual_key_from_string`](../window_manager/fn.virtual_key_from_string.html).
    /// - Calls [`RegisterHotKey`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey)
    ///   to register the combination.  
    /// - If registration succeeds:
    ///   - Updates `self.id` to `Some(id)`.
    ///   - Inserts the hotkey into `app.registered_hotkeys`.
    ///   - Logs an info-level message indicating success.
    /// - If registration fails, logs an error and returns `false`.
    ///
    /// # Side Effects
    /// - A system-wide hotkey is created, affecting all applications in Windows.
    /// - Modifies `self.id` and `app.registered_hotkeys` on success.
    /// - Uses Win32 APIs, which are only valid on Windows.
    ///
    /// # Example
    /// ```rust
    /// let mut hotkey = Hotkey::new("Ctrl+Shift+X").unwrap();
    /// if hotkey.register(&app, 100) {
    ///     println!("Hotkey registered with ID 100");
    /// } else {
    ///     eprintln!("Failed to register hotkey");
    /// }
    /// ```
    ///
    /// # Error Conditions
    /// - Returns `false` if any of:
    ///   - `virtual_key_from_string` yields no recognized key.
    ///   - The Win32 `RegisterHotKey(...)` function call fails.
    /// - Logs an error or warning in these cases.
    ///
    /// # Notes
    /// - Global hotkeys can be a scarce resource on Windows; collisions with other apps can fail the registration.
    /// - To unregister the hotkey, call [`Hotkey::unregister`](#method.unregister).
    pub fn register(&mut self, app: &crate::gui::App, id: i32) -> bool {
        let mut modifiers: u32 = 0;
        let mut vk_code: Option<u32> = None;

        for part in self.key_sequence.split('+') {
            match part.to_lowercase().as_str() {
                "ctrl" => modifiers |= windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL.0,
                "alt" => modifiers |= windows::Win32::UI::Input::KeyboardAndMouse::MOD_ALT.0,
                "shift" => modifiers |= windows::Win32::UI::Input::KeyboardAndMouse::MOD_SHIFT.0,
                "win" => modifiers |= windows::Win32::UI::Input::KeyboardAndMouse::MOD_WIN.0,
                _ => vk_code = crate::window_manager::virtual_key_from_string(part),
            }
        }

        if let Some(vk) = vk_code {
            unsafe {
                if RegisterHotKey(None, id, HOT_KEY_MODIFIERS(modifiers), vk).is_ok() {
                    self.id = Some(id);
                    let mut registered_hotkeys = app.registered_hotkeys.lock().unwrap();
                    registered_hotkeys.insert(self.key_sequence.clone(), id as usize);
                    info!("Registered hotkey '{}' with ID {}.", self.key_sequence, id);
                    return true;
                } else {
                    error!("Failed to register hotkey: '{}'.", self.key_sequence);
                }
            }
        } else {
            warn!("Invalid key sequence for hotkey '{}'.", self.key_sequence);
        }

        false
    }

    /// Unregisters this `Hotkey` from the **global** Windows hotkey system, if it was previously registered.
    ///
    /// # Behavior
    /// - If `self.id` contains a valid integer, calls the Win32 API
    ///   [`UnregisterHotKey`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unregisterhotkey)
    ///   to remove the global hotkey binding.
    /// - On success, removes the corresponding entry from `app.registered_hotkeys`.
    /// - Logs an info-level message if the unregistration succeeds or a warning if it fails.
    /// - Returns `true` if the unregistration call succeeds, otherwise `false`.
    ///
    /// # Side Effects
    /// - A system-wide hotkey is freed, meaning other applications (or this one) could potentially re-register it.
    /// - Logs results using the `log` crate.
    /// - Modifies the `registered_hotkeys` map in the provided `app`.
    ///
    /// # Example
    /// ```rust
    /// let hotkey = Hotkey::new("Ctrl+Q").unwrap();
    /// // ... assume hotkey was registered successfully...
    /// if hotkey.unregister(&app) {
    ///     println!("Hotkey unregistered successfully!");
    /// } else {
    ///     eprintln!("Failed to unregister hotkey!");
    /// }
    /// ```
    ///
    /// # Notes
    /// - If `self.id` is `None`, this function simply returns `false` without calling the Win32 API.
    /// - Only valid on Windows, as it relies on the native global hotkey mechanism.
    pub fn unregister(&self, app: &crate::gui::App) -> bool {
        if let Some(id) = self.id {
            unsafe {
                if UnregisterHotKey(None, id).is_ok() {
                    let mut registered_hotkeys = app.registered_hotkeys.lock().unwrap();
                    registered_hotkeys.remove(&self.key_sequence);
                    info!("Unregistered hotkey '{}'.", self.key_sequence);
                    return true;
                } else {
                    warn!("Failed to unregister hotkey '{}'.", self.key_sequence);
                }
            }
        }
        false
    }
}
