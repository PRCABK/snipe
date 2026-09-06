use capture_core::CaptureError;
use domain::{DesktopPxRect, Frame, PixelFormat};

#[cfg(windows)]
pub fn capture_desktop_rect(rect: DesktopPxRect) -> Result<Frame, CaptureError> {
    use std::mem::size_of;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, ROP_CODE,
    };

    if rect.width == 0 || rect.height == 0 {
        return Err(CaptureError::PlatformError(format!(
            "Invalid capture dimensions: {}x{}",
            rect.width, rect.height
        )));
    }
    let capture_width = i32::try_from(rect.width).map_err(|_| {
        CaptureError::PlatformError(format!("Capture width is too large: {}", rect.width))
    })?;
    let capture_height = i32::try_from(rect.height).map_err(|_| {
        CaptureError::PlatformError(format!("Capture height is too large: {}", rect.height))
    })?;
    let byte_len = (rect.width as usize)
        .checked_mul(rect.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| CaptureError::PlatformError("Capture dimensions overflow".to_string()))?;

    // CAPTUREBLT flag (0x40000000) captures layered/transparent windows
    const CAPTUREBLT: u32 = 0x40000000;
    const SRCCOPY: u32 = 0x00CC0020;
    let rop = ROP_CODE(SRCCOPY | CAPTUREBLT);

    unsafe {
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() {
            return Err(CaptureError::PlatformError(
                "Failed to get screen DC".to_string(),
            ));
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() {
            let _ = ReleaseDC(HWND::default(), hdc_screen);
            return Err(CaptureError::PlatformError(
                "Failed to create compatible DC".to_string(),
            ));
        }

        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: capture_width,
                biHeight: -capture_height, // Top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default()],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(
            hdc_mem,
            &bi,
            DIB_RGB_COLORS,
            &mut bits,
            windows::Win32::Foundation::HANDLE::default(),
            0,
        );

        if hbm.is_err() || bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(HWND::default(), hdc_screen);
            return Err(CaptureError::PlatformError(
                "Failed to create DIB section".to_string(),
            ));
        }

        let hbm_obj = hbm.unwrap();
        let old_obj = SelectObject(hdc_mem, HGDIOBJ(hbm_obj.0));

        let blt_result = BitBlt(
            hdc_mem,
            0,
            0,
            capture_width,
            capture_height,
            hdc_screen,
            rect.x,
            rect.y,
            rop,
        );

        let mut pixels = Vec::with_capacity(byte_len);

        if blt_result.is_ok() {
            let slice = std::slice::from_raw_parts(bits as *const u8, byte_len);
            pixels.extend_from_slice(slice);
        }

        // Cleanup
        SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(HGDIOBJ(hbm_obj.0));
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(HWND::default(), hdc_screen);

        if let Err(err) = blt_result {
            return Err(CaptureError::PlatformError(format!(
                "BitBlt screen capture failed: {err}"
            )));
        }

        // Ensure alpha is 255 for desktop captures
        let (pixel_chunks, remainder) = pixels.as_chunks_mut::<4>();
        debug_assert!(remainder.is_empty());
        for pixel in pixel_chunks {
            pixel[3] = 255;
        }

        Ok(Frame {
            width: rect.width,
            height: rect.height,
            stride: rect.width * 4,
            format: PixelFormat::Bgra8,
            pixels,
            monitor_id: None,
            timestamp_ms: 0,
        })
    }
}

#[cfg(not(windows))]
pub fn capture_desktop_rect(rect: DesktopPxRect) -> Result<Frame, CaptureError> {
    let width = rect.width.max(100);
    let height = rect.height.max(100);
    let pixels = vec![200u8; (width * height * 4) as usize];
    Ok(Frame {
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8,
        pixels,
        monitor_id: None,
        timestamp_ms: 0,
    })
}
