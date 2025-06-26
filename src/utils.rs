use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Display a simple informational message box with an "OK" button.
///
/// This is a thin wrapper around the Windows API `MessageBoxW` function.
/// It is primarily used to provide quick feedback to the user (e.g., when
/// workspaces or desktop layouts are successfully saved).
pub fn show_message_box(message: &str, title: &str) {
    unsafe {
        MessageBoxW(
            HWND(ptr::null_mut()), // Null pointer for no parent window
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            PCWSTR(
                title
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// Displays a **modal confirmation dialog** with “Yes” and “No” buttons, returning `true` if the user clicks “Yes,”
/// or `false` if they click “No” (or close the dialog).
///
/// # Behavior
/// - Uses the Win32 API [`MessageBoxW`](https://learn.microsoft.com/en-us/windows/winuser/nf-winuser-messageboxw)
///   with the flags `MB_YESNO | MB_ICONQUESTION`.
/// - Presents a question-mark icon and waits for user interaction.
/// - Returns a boolean:
///   - `true` if the user chooses “Yes”.
///   - `false` if the user chooses “No” or if the call fails for any reason.
///
/// # Side Effects
/// - Blocks until the user dismisses the dialog.
/// - Shows a native Windows message box on the screen, capturing the user’s response.
///
/// # Example
/// ```no_run
/// if show_confirmation_box("Are you sure you want to continue?", "Confirm Action") {
///     println!("User clicked Yes.");
/// } else {
///     println!("User clicked No or closed the dialog.");
/// }
/// ```
///
/// # Notes
/// - This function is **Windows-specific** due to its use of the native message box API.
/// - For an informational or one-button dialog, use
///   [`show_message_box`](#fn.show_message_box) instead.
pub fn show_confirmation_box(message: &str, title: &str) -> bool {
    unsafe {
        let result = MessageBoxW(
            HWND(ptr::null_mut()), // Null pointer for no parent window
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            PCWSTR(
                title
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            MB_YESNO | MB_ICONQUESTION,
        );

        result == windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_RESULT(6) // IDYES is defined as 6
    }
}

/// Display an error message box with an "OK" button.
///
/// This is similar to [`show_message_box`] but uses a red error icon.
pub fn show_error_box(message: &str, title: &str) {
    unsafe {
        MessageBoxW(
            HWND(ptr::null_mut()),
            PCWSTR(
                message
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            PCWSTR(
                title
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
            ),
            MB_OK | MB_ICONERROR,
        );
    }
}
