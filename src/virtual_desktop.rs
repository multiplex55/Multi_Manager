#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::core::Result;
#[cfg(target_os = "windows")]
pub struct Desktop;

#[cfg(target_os = "windows")]
pub fn get_current_desktop() -> Result<Desktop> {
    Err(windows::core::Error::from_win32())
}

#[cfg(target_os = "windows")]
pub fn get_desktops() -> Result<Vec<Desktop>> {
    Err(windows::core::Error::from_win32())
}

#[cfg(target_os = "windows")]
pub fn switch_desktop(_desktop: &Desktop) -> Result<()> {
    Err(windows::core::Error::from_win32())
}

#[cfg(target_os = "windows")]
pub fn get_desktop_by_window(_hwnd: HWND) -> Result<Desktop> {
    Err(windows::core::Error::from_win32())
}

#[cfg(not(target_os = "windows"))]
use windows::Win32::Foundation::HWND;
#[cfg(not(target_os = "windows"))]
pub type Desktop = ();
#[cfg(not(target_os = "windows"))]
pub type Result<T> = std::result::Result<T, String>;
#[cfg(not(target_os = "windows"))]
pub fn get_current_desktop() -> Result<Desktop> { Err("unsupported".into()) }
#[cfg(not(target_os = "windows"))]
pub fn get_desktops() -> Result<Vec<Desktop>> { Err("unsupported".into()) }
#[cfg(not(target_os = "windows"))]
pub fn switch_desktop(_: &Desktop) -> Result<()> { Err("unsupported".into()) }
#[cfg(not(target_os = "windows"))]
pub fn get_desktop_by_window(_: HWND) -> Result<Desktop> { Err("unsupported".into()) }
