use crate::color::ColorRgba;
use crate::coordinates::ImagePxRect;
use crate::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Bgra8,
    Rgba8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub pixels: Vec<u8>,
    pub monitor_id: Option<String>,
    pub timestamp_ms: u64,
}

impl Frame {
    pub fn new(
        width: u32,
        height: u32,
        stride: u32,
        format: PixelFormat,
        pixels: Vec<u8>,
    ) -> Result<Self, DomainError> {
        let minimum_stride = width
            .checked_mul(4)
            .ok_or(DomainError::InvalidDimensions(width, height))?;
        if width == 0 || height == 0 || stride < minimum_stride {
            return Err(DomainError::InvalidDimensions(width, height));
        }
        let expected_len = (stride as usize)
            .checked_mul(height as usize)
            .ok_or(DomainError::InvalidDimensions(width, height))?;
        if pixels.len() < expected_len {
            return Err(DomainError::BufferMismatch {
                expected: expected_len,
                actual: pixels.len(),
            });
        }

        Ok(Self {
            width,
            height,
            stride,
            format,
            pixels,
            monitor_id: None,
            timestamp_ms: 0,
        })
    }

    pub fn pixel_at(&self, x: u32, y: u32) -> Option<ColorRgba> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let bytes_per_pixel = 4;
        let offset = (y * self.stride + x * bytes_per_pixel) as usize;
        if offset + 3 >= self.pixels.len() {
            return None;
        }

        match self.format {
            PixelFormat::Bgra8 => {
                let b = self.pixels[offset];
                let g = self.pixels[offset + 1];
                let r = self.pixels[offset + 2];
                let a = self.pixels[offset + 3];
                Some(ColorRgba::new(r, g, b, a))
            }
            PixelFormat::Rgba8 => {
                let r = self.pixels[offset];
                let g = self.pixels[offset + 1];
                let b = self.pixels[offset + 2];
                let a = self.pixels[offset + 3];
                Some(ColorRgba::new(r, g, b, a))
            }
        }
    }

    pub fn crop(&self, rect: ImagePxRect) -> Result<Frame, DomainError> {
        let clamped = rect.clamp_to(self.width, self.height);
        if clamped.width == 0 || clamped.height == 0 {
            return Err(DomainError::InvalidDimensions(
                clamped.width,
                clamped.height,
            ));
        }

        let bytes_per_pixel = 4u32;
        let new_stride = clamped.width * bytes_per_pixel;
        let mut new_pixels = Vec::with_capacity((new_stride * clamped.height) as usize);

        for row in 0..clamped.height {
            let src_y = clamped.y + row;
            let src_start = (src_y * self.stride + clamped.x * bytes_per_pixel) as usize;
            let src_end = src_start + (clamped.width * bytes_per_pixel) as usize;
            new_pixels.extend_from_slice(&self.pixels[src_start..src_end]);
        }

        Ok(Frame {
            width: clamped.width,
            height: clamped.height,
            stride: new_stride,
            format: self.format,
            pixels: new_pixels,
            monitor_id: self.monitor_id.clone(),
            timestamp_ms: self.timestamp_ms,
        })
    }

    /// Return a tightly packed RGBA8 copy, removing any per-row padding.
    pub fn to_rgba8(&self) -> Frame {
        let output_stride = self.width * 4;
        let mut rgba = Vec::with_capacity((output_stride * self.height) as usize);

        for y in 0..self.height {
            let row_start = (y * self.stride) as usize;
            for x in 0..self.width {
                let offset = row_start + (x * 4) as usize;
                match self.format {
                    PixelFormat::Bgra8 => {
                        rgba.extend_from_slice(&[
                            self.pixels[offset + 2],
                            self.pixels[offset + 1],
                            self.pixels[offset],
                            self.pixels[offset + 3],
                        ]);
                    }
                    PixelFormat::Rgba8 => {
                        rgba.extend_from_slice(&self.pixels[offset..offset + 4]);
                    }
                }
            }
        }

        Frame {
            width: self.width,
            height: self.height,
            stride: output_stride,
            format: PixelFormat::Rgba8,
            pixels: rgba,
            monitor_id: self.monitor_id.clone(),
            timestamp_ms: self.timestamp_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_crop_and_pixel() {
        let width = 10;
        let height = 10;
        let stride = 40;
        let mut pixels = vec![0u8; 400];

        // Fill (2, 2) with BGRA red
        let offset = (2 * 40 + 2 * 4) as usize;
        pixels[offset] = 0; // B
        pixels[offset + 1] = 0; // G
        pixels[offset + 2] = 255; // R
        pixels[offset + 3] = 255; // A

        let frame = Frame::new(width, height, stride, PixelFormat::Bgra8, pixels).unwrap();
        let pixel = frame.pixel_at(2, 2).unwrap();
        assert_eq!(pixel, ColorRgba::new(255, 0, 0, 255));

        // Crop rect containing (2, 2)
        let cropped = frame.crop(ImagePxRect::new(1, 1, 4, 4)).unwrap();
        assert_eq!(cropped.width, 4);
        assert_eq!(cropped.height, 4);
        let cropped_pixel = cropped.pixel_at(1, 1).unwrap();
        assert_eq!(cropped_pixel, ColorRgba::new(255, 0, 0, 255));
    }

    #[test]
    fn to_rgba8_removes_row_padding() {
        let pixels = vec![1, 2, 3, 255, 9, 9, 9, 9, 4, 5, 6, 255, 8, 8, 8, 8];
        let frame = Frame::new(1, 2, 8, PixelFormat::Bgra8, pixels).unwrap();

        let rgba = frame.to_rgba8();

        assert_eq!(rgba.stride, 4);
        assert_eq!(rgba.pixels, vec![3, 2, 1, 255, 6, 5, 4, 255]);
    }
}
