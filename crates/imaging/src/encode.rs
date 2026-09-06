use chrono::Local;
use domain::Frame;
use image::{ColorType, ImageEncoder};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum ImageEncodeError {
    #[error("Image encoding error: {0}")]
    Image(#[from] image::ImageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),
}

pub fn encode_frame(frame: &Frame, format: &str, quality: u8) -> Result<Vec<u8>, ImageEncodeError> {
    let rgba_frame = frame.to_rgba8();
    let mut buffer = Cursor::new(Vec::new());

    match format.to_lowercase().as_str() {
        "png" => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            encoder.write_image(
                &rgba_frame.pixels,
                rgba_frame.width,
                rgba_frame.height,
                ColorType::Rgba8.into(),
            )?;
        }
        "jpg" | "jpeg" => {
            // Drop alpha for standard JPEG
            let mut rgb_pixels =
                Vec::with_capacity((rgba_frame.width * rgba_frame.height * 3) as usize);
            for chunk in rgba_frame.pixels.chunks_exact(4) {
                rgb_pixels.push(chunk[0]);
                rgb_pixels.push(chunk[1]);
                rgb_pixels.push(chunk[2]);
            }

            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
            encoder.encode(
                &rgb_pixels,
                rgba_frame.width,
                rgba_frame.height,
                ColorType::Rgb8.into(),
            )?;
        }
        "webp" => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
            encoder.write_image(
                &rgba_frame.pixels,
                rgba_frame.width,
                rgba_frame.height,
                ColorType::Rgba8.into(),
            )?;
        }
        other => return Err(ImageEncodeError::UnsupportedFormat(other.to_string())),
    }

    Ok(buffer.into_inner())
}

/// Formats filename from template:
/// e.g. "Screenshot_{yyyy-MM-dd}_{HH-mm-ss}_{w}x{h}"
pub fn format_filename(template: &str, width: u32, height: u32, extension: &str) -> String {
    let now = Local::now();
    let s = template
        .replace("{yyyy-MM-dd}", &now.format("%Y-%m-%d").to_string())
        .replace("{HH-mm-ss}", &now.format("%H-%M-%S").to_string())
        .replace("{w}", &width.to_string())
        .replace("{h}", &height.to_string());

    format!("{}.{}", s, extension.trim_start_matches('.'))
}

/// Atomically saves the encoded bytes to a file (writes to .tmp then renames).
pub fn save_frame_atomic(
    frame: &Frame,
    dir: &Path,
    template: &str,
    format: &str,
    quality: u8,
) -> Result<PathBuf, ImageEncodeError> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    let encoded_bytes = encode_frame(frame, format, quality)?;
    let filename = format_filename(template, frame.width, frame.height, format);
    let target_path = dir.join(filename);
    let temp_path = dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));

    fs::write(&temp_path, encoded_bytes)?;
    fs::rename(&temp_path, &target_path)?;

    info!("Saved screenshot to {}", target_path.display());
    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::PixelFormat;

    #[test]
    fn test_format_filename() {
        let name = format_filename("Test_{w}x{h}", 1920, 1080, "png");
        assert_eq!(name, "Test_1920x1080.png");
    }

    #[test]
    fn test_encode_png() {
        let pixels = vec![255u8; 16]; // 2x2 RGBA
        let frame = Frame::new(2, 2, 8, PixelFormat::Rgba8, pixels).unwrap();
        let encoded = encode_frame(&frame, "png", 90).unwrap();
        assert!(!encoded.is_empty());
        assert_eq!(&encoded[1..4], b"PNG");
    }
}
