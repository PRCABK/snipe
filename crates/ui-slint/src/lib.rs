slint::include_modules!();

use domain::Frame;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub fn frame_to_slint_image(frame: &Frame) -> Image {
    let rgba = frame.to_rgba8();
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(rgba.width, rgba.height);
    let slice = buffer.make_mut_slice();

    let (pixels, remainder) = rgba.pixels.as_chunks::<4>();
    debug_assert!(remainder.is_empty());
    for (target, source) in slice.iter_mut().zip(pixels) {
        *target = Rgba8Pixel {
            r: source[0],
            g: source[1],
            b: source[2],
            a: source[3],
        };
    }

    Image::from_rgba8(buffer)
}
