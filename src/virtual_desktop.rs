#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::core::Result;
/// Represents a virtual desktop. Only a minimal stub implementation is
/// provided as full virtual desktop manipulation is outside the scope of this
/// project.
#[cfg(target_os = "windows")]
#[derive(Clone)]
pub struct Desktop {
    index: u32,
}

#[cfg(target_os = "windows")]
impl Desktop {
    pub fn get_index(&self) -> Result<u32> {
        Ok(self.index)
    }
}

/// Retrieve the current active virtual desktop.
#[cfg(target_os = "windows")]
pub fn get_current_desktop() -> Result<Desktop> {
    Ok(Desktop { index: 0 })
}

/// Enumerate available virtual desktops.
#[cfg(target_os = "windows")]
pub fn get_desktops() -> Result<Vec<Desktop>> {
    Ok(vec![Desktop { index: 0 }])
}

/// Switch to the provided desktop.
#[cfg(target_os = "windows")]
pub fn switch_desktop(_desktop: &Desktop) -> Result<()> {
    Ok(())
}

/// Obtain the desktop that owns the specified window handle.
#[cfg(target_os = "windows")]
pub fn get_desktop_by_window(_hwnd: HWND) -> Result<Desktop> {
    Ok(Desktop { index: 0 })
}

#[cfg(not(target_os = "windows"))]
use windows::Win32::Foundation::HWND;
#[cfg(not(target_os = "windows"))]
pub type Result<T> = std::result::Result<T, String>;
#[cfg(not(target_os = "windows"))]
#[derive(Clone)]
/// Minimal stand-in struct for non-Windows builds.
pub struct Desktop {
    index: u32,
}

#[cfg(not(target_os = "windows"))]
impl Desktop {
    /// Return the index of this desktop.
    pub fn get_index(&self) -> Result<u32> {
        Ok(self.index)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_current_desktop() -> Result<Desktop> { Ok(Desktop { index: 0 }) }
#[cfg(not(target_os = "windows"))]
pub fn get_desktops() -> Result<Vec<Desktop>> { Ok(vec![Desktop { index: 0 }]) }
#[cfg(not(target_os = "windows"))]
pub fn switch_desktop(_: &Desktop) -> Result<()> { Ok(()) }
#[cfg(not(target_os = "windows"))]
pub fn get_desktop_by_window(_: HWND) -> Result<Desktop> { Ok(Desktop { index: 0 }) }
