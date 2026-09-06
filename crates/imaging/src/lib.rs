pub mod clipboard;
pub mod encode;
pub mod filter;
pub mod magnifier;

pub use clipboard::{copy_frame_to_clipboard, copy_text_to_clipboard, ClipboardError};
pub use encode::{encode_frame, format_filename, save_frame_atomic, ImageEncodeError};
pub use filter::{apply_box_blur, apply_mosaic};
pub use magnifier::{generate_magnifier, MagnifierData};
