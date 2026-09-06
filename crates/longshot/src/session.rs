use crate::stitcher::{stitch_frames, LongshotError, StitchOptions};
use domain::{DesktopPxRect, Frame};
use std::time::Instant;

pub enum SessionState {
    Recording,
    Paused,
    Completed,
    Failed(String),
}

pub struct LongshotSession {
    pub target_rect: DesktopPxRect,
    frames: Vec<Frame>,
    options: StitchOptions,
    start_time: Instant,
    state: SessionState,
}

impl LongshotSession {
    pub fn new(target_rect: DesktopPxRect, options: StitchOptions) -> Self {
        Self {
            target_rect,
            frames: Vec::new(),
            options,
            start_time: Instant::now(),
            state: SessionState::Recording,
        }
    }

    pub fn add_frame(&mut self, frame: Frame) -> bool {
        if matches!(self.state, SessionState::Completed | SessionState::Failed(_)) {
            return false;
        }

        // Limit maximum frames to prevent memory explosion
        if self.frames.len() >= 300 {
            self.state = SessionState::Completed;
            return false;
        }

        self.frames.push(frame);
        true
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn finish(&mut self) -> Result<Frame, LongshotError> {
        let result = stitch_frames(&self.frames, &self.options);
        match &result {
            Ok(_) => self.state = SessionState::Completed,
            Err(e) => self.state = SessionState::Failed(e.to_string()),
        }
        result
    }
}
