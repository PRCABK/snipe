use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorRgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_hex_rgb(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn to_hex_rgba(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    pub fn to_rgb_str(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    pub fn to_rgba_str(&self) -> String {
        format!(
            "rgba({}, {}, {}, {:.2})",
            self.r,
            self.g,
            self.b,
            self.a as f32 / 255.0
        )
    }

    pub fn to_hsl(&self) -> ColorHsl {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) / 2.0;

        let (h, s) = if delta == 0.0 {
            (0.0, 0.0)
        } else {
            let s = if l > 0.5 {
                delta / (2.0 - max - min)
            } else {
                delta / (max + min)
            };

            let h = if max == r {
                ((g - b) / delta + (if g < b { 6.0 } else { 0.0 })) * 60.0
            } else if max == g {
                ((b - r) / delta + 2.0) * 60.0
            } else {
                ((r - g) / delta + 4.0) * 60.0
            };

            (h, s)
        };

        ColorHsl {
            h,
            s: s * 100.0,
            l: l * 100.0,
        }
    }

    pub fn to_hsv(&self) -> ColorHsv {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta + (if g < b { 6.0 } else { 0.0 })) * 60.0
        } else if max == g {
            ((b - r) / delta + 2.0) * 60.0
        } else {
            ((r - g) / delta + 4.0) * 60.0
        };

        ColorHsv {
            h,
            s: s * 100.0,
            v: v * 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ColorHsl {
    pub h: f32, // 0..360
    pub s: f32, // 0..100 %
    pub l: f32, // 0..100 %
}

impl ColorHsl {
    pub fn to_hsl_str(&self) -> String {
        format!("hsl({:.0}, {:.0}%, {:.0}%)", self.h, self.s, self.l)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ColorHsv {
    pub h: f32, // 0..360
    pub s: f32, // 0..100 %
    pub v: f32, // 0..100 %
}

impl ColorHsv {
    pub fn to_hsv_str(&self) -> String {
        format!("hsv({:.0}, {:.0}%, {:.0}%)", self.h, self.s, self.v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_formatting() {
        let c = ColorRgba::new(255, 0, 128, 255);
        assert_eq!(c.to_hex_rgb(), "#FF0080");
        assert_eq!(c.to_hex_rgba(), "#FF0080FF");
        assert_eq!(c.to_rgb_str(), "rgb(255, 0, 128)");
    }

    #[test]
    fn test_rgb_to_hsl() {
        let white = ColorRgba::rgb(255, 255, 255);
        let hsl = white.to_hsl();
        assert_eq!(hsl.l.round(), 100.0);

        let black = ColorRgba::rgb(0, 0, 0);
        let hsl_black = black.to_hsl();
        assert_eq!(hsl_black.l.round(), 0.0);

        let red = ColorRgba::rgb(255, 0, 0);
        let hsl_red = red.to_hsl();
        assert_eq!(hsl_red.h.round(), 0.0);
        assert_eq!(hsl_red.s.round(), 100.0);
        assert_eq!(hsl_red.l.round(), 50.0);
    }
}
