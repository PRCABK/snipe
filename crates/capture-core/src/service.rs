use async_trait::async_trait;
use domain::Frame;
use crate::error::CaptureError;
use crate::types::{CaptureTarget, DisplayInfo, WindowInfo};

#[async_trait]
pub trait CaptureService: Send + Sync {
    async fn displays(&self) -> Result<Vec<DisplayInfo>, CaptureError>;
    async fn windows(&self) -> Result<Vec<WindowInfo>, CaptureError>;
    async fn capture(&self, target: CaptureTarget) -> Result<Frame, CaptureError>;
}
