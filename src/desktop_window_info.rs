use serde::{Deserialize, Serialize};

/// Serializable information about a window on a specific virtual desktop.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DesktopWindowInfo {
    pub desktop_index: u32,
    pub hwnd: isize,
    pub title: String,
    pub rect: (i32, i32, i32, i32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let info = DesktopWindowInfo {
            desktop_index: 1,
            hwnd: 42,
            title: "test".into(),
            rect: (1, 2, 3, 4),
        };
        let j = serde_json::to_string(&info).unwrap();
        let back: DesktopWindowInfo = serde_json::from_str(&j).unwrap();
        assert_eq!(info, back);
    }
}
