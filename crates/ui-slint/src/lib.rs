slint::include_modules!();

use domain::Frame;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub fn frame_to_slint_image(frame: &Frame) -> Image {
    let rgba = frame.to_rgba8();
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(rgba.width, rgba.height);
    let slice = buffer.make_mut_slice();

    for (i, chunk) in rgba.pixels.chunks_exact(4).enumerate() {
        if i < slice.len() {
            slice[i] = Rgba8Pixel {
                r: chunk[0],
                g: chunk[1],
                b: chunk[2],
                a: chunk[3],
            };
        }
    }

    Image::from_rgba8(buffer)
}
