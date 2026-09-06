use domain::Frame;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum LongshotError {
    #[error("No frames collected to stitch")]
    NoFrames,

    #[error(
        "Frames dimension mismatch: expected {expected_w}x{expected_h}, got {actual_w}x{actual_h}"
    )]
    DimensionMismatch {
        expected_w: u32,
        expected_h: u32,
        actual_w: u32,
        actual_h: u32,
    },

    #[error("Stitching exceeded maximum height limit of {0} pixels")]
    MaxHeightExceeded(u32),

    #[error("Frame pixel format mismatch")]
    PixelFormatMismatch,

    #[error("Could not reliably align frame {0} with the previous frame")]
    AlignmentFailed(usize),

    #[error("Invalid frame buffer layout")]
    InvalidFrameLayout,

    #[error("Frame buffer allocation failed: {0}")]
    AllocationError(String),
}

pub struct StitchOptions {
    pub max_height: u32,
    pub min_overlap_ratio: f32, // e.g. 0.2
    pub max_overlap_ratio: f32, // e.g. 0.95
    pub search_step: usize,
}

impl Default for StitchOptions {
    fn default() -> Self {
        Self {
            max_height: 30000,
            min_overlap_ratio: 0.15,
            max_overlap_ratio: 0.95,
            search_step: 2,
        }
    }
}

/// Estimates the vertical displacement (downward scroll) from prev_frame to next_frame.
/// Returns the number of new pixels at the bottom of next_frame (displacement).
pub fn estimate_vertical_scroll(
    prev: &Frame,
    next: &Frame,
    options: &StitchOptions,
) -> Option<u32> {
    if prev.width != next.width || prev.height != next.height {
        return None;
    }

    let width = prev.width;
    let height = prev.height;
    let min_overlap = (height as f32 * options.min_overlap_ratio) as u32;
    let max_overlap = (height as f32 * options.max_overlap_ratio) as u32;

    let min_shift = 1;
    let max_shift = height.saturating_sub(min_overlap);

    let mut best_shift = 0;
    let mut min_diff = u64::MAX;

    // Use sample columns across width to speed up matching
    let sample_cols: Vec<u32> = (width / 8..width * 7 / 8)
        .step_by((width / 10).max(1) as usize)
        .collect();

    for shift in (min_shift..=max_shift).step_by(options.search_step) {
        let overlap_len = height - shift;
        if overlap_len > max_overlap || overlap_len < min_overlap {
            continue;
        }

        let mut diff_sum = 0u64;
        let mut sample_count = 0u64;

        // Sample rows in the overlapping region
        for y_prev in (shift..height).step_by(4) {
            let y_next = y_prev - shift;
            for &x in &sample_cols {
                let p1 = prev.pixel_at(x, y_prev);
                let p2 = next.pixel_at(x, y_next);
                if let (Some(c1), Some(c2)) = (p1, p2) {
                    let d = (c1.r as i32 - c2.r as i32).abs()
                        + (c1.g as i32 - c2.g as i32).abs()
                        + (c1.b as i32 - c2.b as i32).abs();
                    diff_sum += d as u64;
                    sample_count += 1;
                }
            }
        }

        if sample_count > 0 {
            let avg_diff = diff_sum / sample_count;
            if avg_diff < min_diff {
                min_diff = avg_diff;
                best_shift = shift;
            }
        }
    }

    // A low difference threshold confirms reliable overlap
    if min_diff < 40 && best_shift > 0 {
        debug!(
            "Detected vertical scroll displacement: {} px (diff: {})",
            best_shift, min_diff
        );
        Some(best_shift)
    } else {
        None
    }
}

/// Stitches a series of consecutive scrolled frames into a single long vertical Frame.
pub fn stitch_frames(frames: &[Frame], options: &StitchOptions) -> Result<Frame, LongshotError> {
    if frames.is_empty() {
        return Err(LongshotError::NoFrames);
    }

    let first = &frames[0];
    let width = first.width;
    let frame_height = first.height;
    let format = first.format;
    let bytes_per_pixel = 4u32;
    let stride = width
        .checked_mul(bytes_per_pixel)
        .ok_or(LongshotError::InvalidFrameLayout)?;

    let validate_frame = |frame: &Frame| -> Result<(), LongshotError> {
        if frame.width != width || frame.height != frame_height {
            return Err(LongshotError::DimensionMismatch {
                expected_w: width,
                expected_h: frame_height,
                actual_w: frame.width,
                actual_h: frame.height,
            });
        }
        if frame.format != format {
            return Err(LongshotError::PixelFormatMismatch);
        }
        let minimum_stride = frame
            .width
            .checked_mul(bytes_per_pixel)
            .ok_or(LongshotError::InvalidFrameLayout)?;
        let required_len = (frame.stride as usize)
            .checked_mul(frame.height as usize)
            .ok_or(LongshotError::InvalidFrameLayout)?;
        if frame.stride < minimum_stride || frame.pixels.len() < required_len {
            return Err(LongshotError::InvalidFrameLayout);
        }
        Ok(())
    };

    validate_frame(first)?;

    let mut total_height = frame_height;
    let initial_capacity = (stride as usize)
        .checked_mul(frame_height as usize)
        .ok_or(LongshotError::InvalidFrameLayout)?;
    let mut stitched_pixels = Vec::new();
    stitched_pixels
        .try_reserve_exact(initial_capacity)
        .map_err(|err| LongshotError::AllocationError(err.to_string()))?;
    append_rows(&mut stitched_pixels, first, 0, frame_height, stride)?;

    for i in 1..frames.len() {
        let prev = &frames[i - 1];
        let curr = &frames[i];

        validate_frame(curr)?;

        let shift = estimate_vertical_scroll(prev, curr, options)
            .ok_or(LongshotError::AlignmentFailed(i))?;

        let next_height = total_height
            .checked_add(shift)
            .ok_or(LongshotError::MaxHeightExceeded(options.max_height))?;
        if next_height > options.max_height {
            return Err(LongshotError::MaxHeightExceeded(options.max_height));
        }

        let start_row = curr.height.saturating_sub(shift);
        append_rows(&mut stitched_pixels, curr, start_row, shift, stride)?;
        total_height = next_height;
    }

    info!(
        "Stitched {} frames into long image ({}x{})",
        frames.len(),
        width,
        total_height
    );

    Ok(Frame {
        width,
        height: total_height,
        stride,
        format,
        pixels: stitched_pixels,
        monitor_id: frames[0].monitor_id.clone(),
        timestamp_ms: 0,
    })
}

fn append_rows(
    destination: &mut Vec<u8>,
    frame: &Frame,
    start_row: u32,
    row_count: u32,
    output_stride: u32,
) -> Result<(), LongshotError> {
    let end_row = start_row
        .checked_add(row_count)
        .ok_or(LongshotError::InvalidFrameLayout)?;
    if end_row > frame.height || output_stride > frame.stride {
        return Err(LongshotError::InvalidFrameLayout);
    }

    let additional = (output_stride as usize)
        .checked_mul(row_count as usize)
        .ok_or(LongshotError::InvalidFrameLayout)?;
    destination
        .try_reserve(additional)
        .map_err(|err| LongshotError::AllocationError(err.to_string()))?;

    for row in start_row..end_row {
        let row_start = (row as usize)
            .checked_mul(frame.stride as usize)
            .ok_or(LongshotError::InvalidFrameLayout)?;
        let row_end = row_start
            .checked_add(output_stride as usize)
            .ok_or(LongshotError::InvalidFrameLayout)?;
        let source = frame
            .pixels
            .get(row_start..row_end)
            .ok_or(LongshotError::InvalidFrameLayout)?;
        destination.extend_from_slice(source);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::PixelFormat;

    #[test]
    fn test_single_frame_stitch() {
        let frame = Frame::new(10, 10, 40, PixelFormat::Rgba8, vec![255; 400]).unwrap();
        let stitched = stitch_frames(&[frame], &StitchOptions::default()).unwrap();
        assert_eq!(stitched.width, 10);
        assert_eq!(stitched.height, 10);
    }

    #[test]
    fn preserves_bgra_format_and_removes_stride_padding() {
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&[1, 2, 3, 255, 9, 9, 9, 9]);
        pixels.extend_from_slice(&[4, 5, 6, 255, 8, 8, 8, 8]);
        let frame = Frame::new(1, 2, 8, PixelFormat::Bgra8, pixels).unwrap();

        let stitched = stitch_frames(&[frame], &StitchOptions::default()).unwrap();

        assert_eq!(stitched.format, PixelFormat::Bgra8);
        assert_eq!(stitched.stride, 4);
        assert_eq!(stitched.pixels, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }
}
