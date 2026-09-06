use domain::DesktopPxRect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub bounds: DesktopPxRect,
    pub scale_factor: f32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: isize, // HWND handle as integer
    pub title: String,
    pub class_name: String,
    pub bounds: DesktopPxRect,
    pub is_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureTarget {
    VirtualDesktop,
    Display(String),
    Window(isize),
    Region(DesktopPxRect),
}
