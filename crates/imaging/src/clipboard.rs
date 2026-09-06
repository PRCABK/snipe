use arboard::{Clipboard, ImageData};
use domain::Frame;
use std::borrow::Cow;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("Clipboard operation failed: {0}")]
    Arboard(#[from] arboard::Error),
}

pub fn copy_frame_to_clipboard(frame: &Frame) -> Result<(), ClipboardError> {
    let rgba = frame.to_rgba8();
    let mut clipboard = Clipboard::new()?;

    let image_data = ImageData {
        width: rgba.width as usize,
        height: rgba.height as usize,
        bytes: Cow::Borrowed(&rgba.pixels),
    };

    clipboard.set_image(image_data)?;
    info!(
        "Successfully copied frame ({}x{}) to clipboard",
        frame.width, frame.height
    );
    Ok(())
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    info!(
        "Successfully copied text (length: {}) to clipboard",
        text.len()
    );
    Ok(())
}
