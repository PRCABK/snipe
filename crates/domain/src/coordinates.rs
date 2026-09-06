use crate::error::DomainError;
use serde::{Deserialize, Serialize};

/// Windows virtual desktop coordinate in physical pixels (can be negative on multi-monitor setups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DesktopPxPoint {
    pub x: i32,
    pub y: i32,
}

impl DesktopPxPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Rectangle on the Windows virtual desktop in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DesktopPxRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DesktopPxRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    pub fn contains(&self, pt: DesktopPxPoint) -> bool {
        pt.x >= self.x && pt.x < self.right() && pt.y >= self.y && pt.y < self.bottom()
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn normalize(p1: DesktopPxPoint, p2: DesktopPxPoint) -> Self {
        let min_x = p1.x.min(p2.x);
        let min_y = p1.y.min(p2.y);
        let max_x = p1.x.max(p2.x);
        let max_y = p1.y.max(p2.y);
        Self {
            x: min_x,
            y: min_y,
            width: (max_x - min_x) as u32,
            height: (max_y - min_y) as u32,
        }
    }
}

/// Coordinate relative to a specific monitor's origin in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct MonitorPxPoint {
    pub x: i32,
    pub y: i32,
}

impl MonitorPxPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Rectangle relative to a specific monitor in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MonitorPxRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorPxRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Coordinate relative to a captured image/frame's top-left (0, 0) in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ImagePxPoint {
    pub x: u32,
    pub y: u32,
}

impl ImagePxPoint {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// Rectangle inside a captured image/frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImagePxRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ImagePxRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn clamp_to(&self, max_width: u32, max_height: u32) -> Self {
        let x = self.x.min(max_width);
        let y = self.y.min(max_height);
        let width = self.width.min(max_width.saturating_sub(x));
        let height = self.height.min(max_height.saturating_sub(y));
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Logical point in Slint DIP / device independent pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct LogicalPoint {
    pub x: f32,
    pub y: f32,
}

impl LogicalPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Logical rectangle in Slint DIP / device independent pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct LogicalRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LogicalRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Conversion functions between desktop physical pixels and image local coordinates.
pub fn desktop_rect_to_image_rect(
    desktop_rect: DesktopPxRect,
    frame_origin: DesktopPxPoint,
    frame_width: u32,
    frame_height: u32,
) -> Result<ImagePxRect, DomainError> {
    let local_x = desktop_rect.x - frame_origin.x;
    let local_y = desktop_rect.y - frame_origin.y;

    if local_x < 0 || local_y < 0 {
        return Err(DomainError::OutOfBounds(format!(
            "Desktop rect ({}, {}) is outside frame origin ({}, {})",
            desktop_rect.x, desktop_rect.y, frame_origin.x, frame_origin.y
        )));
    }

    let img_x = local_x as u32;
    let img_y = local_y as u32;

    let clamped = ImagePxRect::new(img_x, img_y, desktop_rect.width, desktop_rect.height)
        .clamp_to(frame_width, frame_height);

    if clamped.width == 0 || clamped.height == 0 {
        return Err(DomainError::InvalidDimensions(
            clamped.width,
            clamped.height,
        ));
    }

    Ok(clamped)
}

/// Converts logical points to physical desktop points given DPI scale factor.
pub fn logical_to_desktop(pt: LogicalPoint, scale: f32, offset: DesktopPxPoint) -> DesktopPxPoint {
    DesktopPxPoint {
        x: ((pt.x * scale).round() as i32) + offset.x,
        y: ((pt.y * scale).round() as i32) + offset.y,
    }
}

/// Converts physical desktop points to logical points given DPI scale factor.
pub fn desktop_to_logical(pt: DesktopPxPoint, scale: f32, offset: DesktopPxPoint) -> LogicalPoint {
    let safe_scale = if scale <= 0.0 { 1.0 } else { scale };
    LogicalPoint {
        x: ((pt.x - offset.x) as f32) / safe_scale,
        y: ((pt.y - offset.y) as f32) / safe_scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_rect_normalize() {
        let p1 = DesktopPxPoint::new(100, 200);
        let p2 = DesktopPxPoint::new(10, 50);
        let rect = DesktopPxRect::normalize(p1, p2);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 50);
        assert_eq!(rect.width, 90);
        assert_eq!(rect.height, 150);
    }

    #[test]
    fn test_desktop_rect_contains() {
        let rect = DesktopPxRect::new(100, 100, 200, 150);
        assert!(rect.contains(DesktopPxPoint::new(100, 100)));
        assert!(rect.contains(DesktopPxPoint::new(299, 249)));
        assert!(!rect.contains(DesktopPxPoint::new(300, 250)));
        assert!(!rect.contains(DesktopPxPoint::new(99, 100)));
    }

    #[test]
    fn test_desktop_to_image_conversion() {
        let origin = DesktopPxPoint::new(-1920, 0); // multi-monitor left screen
        let selection = DesktopPxRect::new(-1500, 100, 400, 300);
        let converted = desktop_rect_to_image_rect(selection, origin, 1920, 1080).unwrap();
        assert_eq!(converted.x, 420);
        assert_eq!(converted.y, 100);
        assert_eq!(converted.width, 400);
        assert_eq!(converted.height, 300);
    }

    #[test]
    fn test_logical_desktop_roundtrip() {
        let scale = 1.5;
        let offset = DesktopPxPoint::new(100, 200);
        let logical = LogicalPoint::new(150.0, 100.0);
        let desktop = logical_to_desktop(logical, scale, offset);
        let roundtrip = desktop_to_logical(desktop, scale, offset);
        assert!((roundtrip.x - logical.x).abs() < 1.0);
        assert!((roundtrip.y - logical.y).abs() < 1.0);
    }
}
