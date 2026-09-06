use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Display not found: {0}")]
    DisplayNotFound(String),

    #[error("Window not found or invalid: {0}")]
    WindowNotFound(isize),

    #[error("Platform capture failed: {0}")]
    PlatformError(String),

    #[error("Capture access denied or protected content: {0}")]
    AccessDenied(String),

    #[error("Device lost or Direct3D reset")]
    DeviceLost,
}
