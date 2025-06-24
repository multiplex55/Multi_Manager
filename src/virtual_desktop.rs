#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::core::Result;
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

#[cfg(target_os = "windows")]
pub fn get_current_desktop() -> Result<Desktop> {
    Ok(Desktop { index: 0 })
}

#[cfg(target_os = "windows")]
pub fn get_desktops() -> Result<Vec<Desktop>> {
    Ok(vec![Desktop { index: 0 }])
}

#[cfg(target_os = "windows")]
pub fn switch_desktop(_desktop: &Desktop) -> Result<()> {
    Ok(())
}

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
pub struct Desktop {
    index: u32,
}

#[cfg(not(target_os = "windows"))]
impl Desktop {
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
