use domain::{Frame, ImagePxRect};

/// Applies pixelate / mosaic to a specific sub-rectangle of the frame
pub fn apply_mosaic(frame: &mut Frame, rect: ImagePxRect, block_size: u32) {
    let clamped = rect.clamp_to(frame.width, frame.height);
    if clamped.width == 0 || clamped.height == 0 || block_size == 0 {
        return;
    }

    let b_size = block_size.max(1);
    let bytes_per_pixel = 4u32;

    for by in (clamped.y..clamped.bottom()).step_by(b_size as usize) {
        for bx in (clamped.x..clamped.right()).step_by(b_size as usize) {
            let bw = (b_size).min(clamped.right().saturating_sub(bx));
            let bh = (b_size).min(clamped.bottom().saturating_sub(by));

            // Compute average color in block
            let mut sum_0 = 0u64;
            let mut sum_1 = 0u64;
            let mut sum_2 = 0u64;
            let mut sum_3 = 0u64;
            let mut count = 0u64;

            for y in by..(by + bh) {
                for x in bx..(bx + bw) {
                    let offset = (y * frame.stride + x * bytes_per_pixel) as usize;
                    if offset + 3 < frame.pixels.len() {
                        sum_0 += frame.pixels[offset] as u64;
                        sum_1 += frame.pixels[offset + 1] as u64;
                        sum_2 += frame.pixels[offset + 2] as u64;
                        sum_3 += frame.pixels[offset + 3] as u64;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let avg_0 = (sum_0 / count) as u8;
                let avg_1 = (sum_1 / count) as u8;
                let avg_2 = (sum_2 / count) as u8;
                let avg_3 = (sum_3 / count) as u8;

                // Fill block with average
                for y in by..(by + bh) {
                    for x in bx..(bx + bw) {
                        let offset = (y * frame.stride + x * bytes_per_pixel) as usize;
                        if offset + 3 < frame.pixels.len() {
                            frame.pixels[offset] = avg_0;
                            frame.pixels[offset + 1] = avg_1;
                            frame.pixels[offset + 2] = avg_2;
                            frame.pixels[offset + 3] = avg_3;
                        }
                    }
                }
            }
        }
    }
}

/// Fast box blur for mosaic/blur redactions
pub fn apply_box_blur(frame: &mut Frame, rect: ImagePxRect, radius: u32) {
    let clamped = rect.clamp_to(frame.width, frame.height);
    if clamped.width <= 1 || clamped.height <= 1 || radius == 0 {
        return;
    }

    let r = radius as i32;
    let bytes_per_pixel = 4u32;
    let mut temp = frame.pixels.clone();

    // Horizontal pass
    for y in clamped.y..clamped.bottom() {
        for x in clamped.x..clamped.right() {
            let mut sum_0 = 0u32;
            let mut sum_1 = 0u32;
            let mut sum_2 = 0u32;
            let mut sum_3 = 0u32;
            let mut count = 0u32;

            for dx in -r..=r {
                let sample_x =
                    (x as i32 + dx).clamp(clamped.x as i32, clamped.right() as i32 - 1) as u32;
                let offset = (y * frame.stride + sample_x * bytes_per_pixel) as usize;
                if offset + 3 < frame.pixels.len() {
                    sum_0 += frame.pixels[offset] as u32;
                    sum_1 += frame.pixels[offset + 1] as u32;
                    sum_2 += frame.pixels[offset + 2] as u32;
                    sum_3 += frame.pixels[offset + 3] as u32;
                    count += 1;
                }
            }

            if count > 0 {
                let offset = (y * frame.stride + x * bytes_per_pixel) as usize;
                temp[offset] = (sum_0 / count) as u8;
                temp[offset + 1] = (sum_1 / count) as u8;
                temp[offset + 2] = (sum_2 / count) as u8;
                temp[offset + 3] = (sum_3 / count) as u8;
            }
        }
    }

    // Vertical pass
    for x in clamped.x..clamped.right() {
        for y in clamped.y..clamped.bottom() {
            let mut sum_0 = 0u32;
            let mut sum_1 = 0u32;
            let mut sum_2 = 0u32;
            let mut sum_3 = 0u32;
            let mut count = 0u32;

            for dy in -r..=r {
                let sample_y =
                    (y as i32 + dy).clamp(clamped.y as i32, clamped.bottom() as i32 - 1) as u32;
                let offset = (sample_y * frame.stride + x * bytes_per_pixel) as usize;
                if offset + 3 < temp.len() {
                    sum_0 += temp[offset] as u32;
                    sum_1 += temp[offset + 1] as u32;
                    sum_2 += temp[offset + 2] as u32;
                    sum_3 += temp[offset + 3] as u32;
                    count += 1;
                }
            }

            if count > 0 {
                let offset = (y * frame.stride + x * bytes_per_pixel) as usize;
                frame.pixels[offset] = (sum_0 / count) as u8;
                frame.pixels[offset + 1] = (sum_1 / count) as u8;
                frame.pixels[offset + 2] = (sum_2 / count) as u8;
                frame.pixels[offset + 3] = (sum_3 / count) as u8;
            }
        }
    }
}
