pub mod display;
pub mod gdi;
pub mod window;

use async_trait::async_trait;
use capture_core::{CaptureError, CaptureService, CaptureTarget, DisplayInfo, WindowInfo};
use domain::Frame;
use display::{enumerate_displays, get_virtual_desktop_bounds};
use gdi::capture_desktop_rect;
use window::enumerate_windows;

pub struct WindowsCaptureService;

impl WindowsCaptureService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsCaptureService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CaptureService for WindowsCaptureService {
    async fn displays(&self) -> Result<Vec<DisplayInfo>, CaptureError> {
        tokio::task::spawn_blocking(enumerate_displays)
            .await
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?
    }

    async fn windows(&self) -> Result<Vec<WindowInfo>, CaptureError> {
        tokio::task::spawn_blocking(enumerate_windows)
            .await
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?
    }

    async fn capture(&self, target: CaptureTarget) -> Result<Frame, CaptureError> {
        let displays = self.displays().await?;

        let rect = match target {
            CaptureTarget::VirtualDesktop => get_virtual_desktop_bounds(&displays),
            CaptureTarget::Display(id) => {
                let disp = displays
                    .into_iter()
                    .find(|d| d.id == id)
                    .ok_or_else(|| CaptureError::DisplayNotFound(id))?;
                disp.bounds
            }
            CaptureTarget::Window(hwnd_id) => {
                let windows = self.windows().await?;
                let win = windows
                    .into_iter()
                    .find(|w| w.id == hwnd_id)
                    .ok_or(CaptureError::WindowNotFound(hwnd_id))?;
                win.bounds
            }
            CaptureTarget::Region(r) => r,
        };

        tokio::task::spawn_blocking(move || capture_desktop_rect(rect))
            .await
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?
    }
}
