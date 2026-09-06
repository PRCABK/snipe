use crate::shape::{AnnotationItem, AnnotationKind};
use domain::{ColorRgba, Frame};
use imaging::{apply_box_blur, apply_mosaic};

/// Renders all annotations on a clone of the original frame at 1:1 physical resolution
pub fn composite_annotations(base_frame: &Frame, items: &[AnnotationItem]) -> Frame {
    let mut result = base_frame.clone();

    // 1. Process pixel filters first (mosaic and blur)
    for item in items {
        if !item.visible {
            continue;
        }
        match &item.kind {
            AnnotationKind::Mosaic(m) => {
                apply_mosaic(&mut result, m.rect, m.block_size);
            }
            AnnotationKind::Blur(b) => {
                apply_box_blur(&mut result, b.rect, b.radius);
            }
            _ => {}
        }
    }

    // 2. Process vector graphics and text
    for item in items {
        if !item.visible {
            continue;
        }
        match &item.kind {
            AnnotationKind::Rect(r) => {
                draw_rect(
                    &mut result,
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    r.stroke_color,
                    r.stroke_width,
                    r.fill_color,
                );
            }
            AnnotationKind::Ellipse(e) => {
                draw_ellipse(
                    &mut result,
                    e.cx,
                    e.cy,
                    e.rx,
                    e.ry,
                    e.stroke_color,
                    e.stroke_width,
                    e.fill_color,
                );
            }
            AnnotationKind::Line(l) => {
                draw_line(
                    &mut result,
                    l.x1,
                    l.y1,
                    l.x2,
                    l.y2,
                    l.stroke_color,
                    l.stroke_width,
                );
            }
            AnnotationKind::Arrow(a) => {
                draw_arrow(
                    &mut result,
                    a.x1,
                    a.y1,
                    a.x2,
                    a.y2,
                    a.stroke_color,
                    a.stroke_width,
                );
            }
            AnnotationKind::Freehand(p) => {
                draw_path(
                    &mut result,
                    &p.points,
                    p.stroke_color,
                    p.stroke_width,
                    p.is_highlighter,
                );
            }
            AnnotationKind::Step(s) => {
                draw_step_badge(&mut result, s.cx, s.cy, s.radius, s.bg_color, s.number);
            }
            AnnotationKind::Text(t) => {
                draw_simple_text(&mut result, t.x, t.y, &t.text, t.color);
            }
            AnnotationKind::Mosaic(_) | AnnotationKind::Blur(_) => {}
        }
    }

    result
}

fn set_pixel_blend(frame: &mut Frame, x: i32, y: i32, color: ColorRgba) {
    if x < 0 || y < 0 || (x as u32) >= frame.width || (y as u32) >= frame.height {
        return;
    }

    let offset = (y as u32 * frame.stride + x as u32 * 4) as usize;
    if offset + 3 >= frame.pixels.len() {
        return;
    }

    let alpha = color.a as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    let b_idx = if frame.format == domain::PixelFormat::Bgra8 {
        0
    } else {
        2
    };
    let r_idx = if frame.format == domain::PixelFormat::Bgra8 {
        2
    } else {
        0
    };

    let orig_r = frame.pixels[offset + r_idx] as f32;
    let orig_g = frame.pixels[offset + 1] as f32;
    let orig_b = frame.pixels[offset + b_idx] as f32;

    frame.pixels[offset + r_idx] = (color.r as f32 * alpha + orig_r * inv_alpha).round() as u8;
    frame.pixels[offset + 1] = (color.g as f32 * alpha + orig_g * inv_alpha).round() as u8;
    frame.pixels[offset + b_idx] = (color.b as f32 * alpha + orig_b * inv_alpha).round() as u8;
    frame.pixels[offset + 3] = 255;
}

fn draw_rect(
    frame: &mut Frame,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    stroke: ColorRgba,
    stroke_width: f32,
    fill: Option<ColorRgba>,
) {
    let ix = x.round() as i32;
    let iy = y.round() as i32;
    let iw = w.round() as i32;
    let ih = h.round() as i32;
    let sw = stroke_width.max(1.0).round() as i32;

    if let Some(f_color) = fill {
        for py in (iy + sw)..(iy + ih - sw) {
            for px in (ix + sw)..(ix + iw - sw) {
                set_pixel_blend(frame, px, py, f_color);
            }
        }
    }

    // Top & Bottom
    for py in iy..(iy + sw) {
        for px in ix..(ix + iw) {
            set_pixel_blend(frame, px, py, stroke);
        }
    }
    for py in (iy + ih - sw)..(iy + ih) {
        for px in ix..(ix + iw) {
            set_pixel_blend(frame, px, py, stroke);
        }
    }
    // Left & Right
    for px in ix..(ix + sw) {
        for py in iy..(iy + ih) {
            set_pixel_blend(frame, px, py, stroke);
        }
    }
    for px in (ix + iw - sw)..(ix + iw) {
        for py in iy..(iy + ih) {
            set_pixel_blend(frame, px, py, stroke);
        }
    }
}

fn draw_line(frame: &mut Frame, x1: f32, y1: f32, x2: f32, y2: f32, color: ColorRgba, width: f32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
    let steps = (dist * 2.0).ceil() as usize;

    let half_w = (width / 2.0).max(0.5);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = x1 + dx * t;
        let cy = y1 + dy * t;

        let ix = cx.round() as i32;
        let iy = cy.round() as i32;

        let r = half_w.ceil() as i32;
        for dy_box in -r..=r {
            for dx_box in -r..=r {
                if (dx_box * dx_box + dy_box * dy_box) as f32 <= half_w * half_w {
                    set_pixel_blend(frame, ix + dx_box, iy + dy_box, color);
                }
            }
        }
    }
}

fn draw_arrow(frame: &mut Frame, x1: f32, y1: f32, x2: f32, y2: f32, color: ColorRgba, width: f32) {
    draw_line(frame, x1, y1, x2, y2, color, width);

    let angle = (y2 - y1).atan2(x2 - x1);
    let arrow_len = (width * 4.0).max(16.0);
    let arrow_angle: f32 = 0.45; // ~26 degrees

    let left_x = x2 - arrow_len * (angle - arrow_angle).cos();
    let left_y = y2 - arrow_len * (angle - arrow_angle).sin();

    let right_x = x2 - arrow_len * (angle + arrow_angle).cos();
    let right_y = y2 - arrow_len * (angle + arrow_angle).sin();

    draw_line(frame, x2, y2, left_x, left_y, color, width);
    draw_line(frame, x2, y2, right_x, right_y, color, width);
}

fn draw_ellipse(
    frame: &mut Frame,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    stroke: ColorRgba,
    stroke_width: f32,
    fill: Option<ColorRgba>,
) {
    let min_x = (cx - rx - stroke_width).floor() as i32;
    let max_x = (cx + rx + stroke_width).ceil() as i32;
    let min_y = (cy - ry - stroke_width).floor() as i32;
    let max_y = (cy + ry + stroke_width).ceil() as i32;

    let rx_outer = rx + stroke_width / 2.0;
    let ry_outer = ry + stroke_width / 2.0;
    let rx_inner = (rx - stroke_width / 2.0).max(0.1);
    let ry_inner = (ry - stroke_width / 2.0).max(0.1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;

            let outer_dist = (dx * dx) / (rx_outer * rx_outer) + (dy * dy) / (ry_outer * ry_outer);
            let inner_dist = (dx * dx) / (rx_inner * rx_inner) + (dy * dy) / (ry_inner * ry_inner);

            if outer_dist <= 1.0 && inner_dist >= 1.0 {
                set_pixel_blend(frame, x, y, stroke);
            } else if inner_dist < 1.0 {
                if let Some(f) = fill {
                    set_pixel_blend(frame, x, y, f);
                }
            }
        }
    }
}

fn draw_path(
    frame: &mut Frame,
    points: &[(f32, f32)],
    color: ColorRgba,
    width: f32,
    is_highlighter: bool,
) {
    if points.len() < 2 {
        return;
    }

    let draw_color = if is_highlighter {
        ColorRgba::new(color.r, color.g, color.b, 100)
    } else {
        color
    };

    let draw_width = if is_highlighter { width * 2.5 } else { width };

    for w in points.windows(2) {
        draw_line(
            frame, w[0].0, w[0].1, w[1].0, w[1].1, draw_color, draw_width,
        );
    }
}

fn draw_step_badge(
    frame: &mut Frame,
    cx: f32,
    cy: f32,
    radius: f32,
    bg_color: ColorRgba,
    number: u32,
) {
    draw_ellipse(
        frame,
        cx,
        cy,
        radius,
        radius,
        ColorRgba::rgb(255, 255, 255),
        2.0,
        Some(bg_color),
    );
    // Simple pixel number dot pattern or text
    let num_str = number.to_string();
    draw_simple_text(
        frame,
        cx - 4.0,
        cy - 6.0,
        &num_str,
        ColorRgba::rgb(255, 255, 255),
    );
}

fn draw_simple_text(frame: &mut Frame, x: f32, y: f32, text: &str, color: ColorRgba) {
    // 5x7 bitmap font glyphs for basic numbers and chars
    let ix = x.round() as i32;
    let iy = y.round() as i32;

    for (idx, ch) in text.chars().enumerate() {
        let char_offset_x = ix + (idx as i32 * 8);
        draw_char_5x7(frame, char_offset_x, iy, ch, color);
    }
}

fn draw_char_5x7(frame: &mut Frame, x: i32, y: i32, ch: char, color: ColorRgba) {
    let glyph: [u8; 7] = match ch {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F],
    };

    for row in 0..7 {
        let mask = glyph[row];
        for col in 0..5 {
            if (mask & (0x10 >> col)) != 0 {
                set_pixel_blend(frame, x + col, y + row as i32, color);
            }
        }
    }
}
