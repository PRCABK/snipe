use domain::{ColorRgba, Frame, PixelFormat};

#[derive(Debug, Clone)]
pub struct MagnifierData {
    pub center_color: ColorRgba,
    pub grid_size: u32,
    pub pixel_size: u32,
    pub preview_frame: Frame,
}

/// Generates a zoomed preview around center (cx, cy)
pub fn generate_magnifier(
    frame: &Frame,
    cx: i32,
    cy: i32,
    grid_count: u32,
    cell_pixels: u32,
) -> MagnifierData {
    let half_grid = (grid_count / 2) as i32;
    let center_color = if cx >= 0 && cy >= 0 {
        frame.pixel_at(cx as u32, cy as u32).unwrap_or_default()
    } else {
        ColorRgba::default()
    };

    let preview_dim = grid_count * cell_pixels;
    let mut pixels = vec![0u8; (preview_dim * preview_dim * 4) as usize];
    let stride = preview_dim * 4;

    for gy in 0..grid_count {
        for gx in 0..grid_count {
            let sample_x = cx - half_grid + gx as i32;
            let sample_y = cy - half_grid + gy as i32;

            let color = if sample_x >= 0
                && sample_y >= 0
                && (sample_x as u32) < frame.width
                && (sample_y as u32) < frame.height
            {
                frame
                    .pixel_at(sample_x as u32, sample_y as u32)
                    .unwrap_or_default()
            } else {
                ColorRgba::new(0, 0, 0, 255)
            };

            // Paint cell
            let px_start_x = gx * cell_pixels;
            let px_start_y = gy * cell_pixels;

            let is_center = gx == grid_count / 2 && gy == grid_count / 2;

            for py in 0..cell_pixels {
                for px in 0..cell_pixels {
                    let out_x = px_start_x + px;
                    let out_y = px_start_y + py;
                    let offset = (out_y * stride + out_x * 4) as usize;

                    // Draw grid border
                    let is_border =
                        px == 0 || py == 0 || px == cell_pixels - 1 || py == cell_pixels - 1;
                    if is_center && is_border {
                        // High contrast red crosshair border for center
                        pixels[offset] = 0;
                        pixels[offset + 1] = 0;
                        pixels[offset + 2] = 255;
                        pixels[offset + 3] = 255;
                    } else if is_border {
                        // Subtle grid line
                        pixels[offset] = 60;
                        pixels[offset + 1] = 60;
                        pixels[offset + 2] = 60;
                        pixels[offset + 3] = 255;
                    } else {
                        // Cell color
                        pixels[offset] = color.r;
                        pixels[offset + 1] = color.g;
                        pixels[offset + 2] = color.b;
                        pixels[offset + 3] = 255;
                    }
                }
            }
        }
    }

    MagnifierData {
        center_color,
        grid_size: grid_count,
        pixel_size: cell_pixels,
        preview_frame: Frame {
            width: preview_dim,
            height: preview_dim,
            stride,
            format: PixelFormat::Rgba8,
            pixels,
            monitor_id: None,
            timestamp_ms: 0,
        },
    }
}
