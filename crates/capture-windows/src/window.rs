use capture_core::{CaptureError, WindowInfo};
use domain::DesktopPxRect;

#[cfg(windows)]
pub fn enumerate_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    use std::mem::size_of;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        IsIconic, IsWindowVisible,
    };

    let mut windows_list = Vec::new();

    unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list = &mut *(lparam.0 as *mut Vec<WindowInfo>);

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let is_minimized = IsIconic(hwnd).as_bool();
        if is_minimized {
            return BOOL(1);
        }

        // Window Title
        let text_len = GetWindowTextLengthW(hwnd);
        let mut title_buf = vec![0u16; (text_len + 1) as usize];
        let actual_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..actual_len as usize]);

        // Class name
        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf);
        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

        // Extended frame bounds (excluding drop shadows)
        let mut rect = RECT::default();
        let dwm_res = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            size_of::<RECT>() as u32,
        );

        if dwm_res.is_err() {
            let _ = GetWindowRect(hwnd, &mut rect);
        }

        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;

        // Filter out empty / invisible utility windows
        if width > 10 && height > 10 {
            list.push(WindowInfo {
                id: hwnd.0 as isize,
                title,
                class_name,
                bounds: DesktopPxRect::new(rect.left, rect.top, width, height),
                is_minimized,
            });
        }

        BOOL(1)
    }

    unsafe {
        let list_ptr = &mut windows_list as *mut Vec<WindowInfo>;
        let _ = EnumWindows(Some(enum_window_proc), LPARAM(list_ptr as isize));
    }

    Ok(windows_list)
}

#[cfg(not(windows))]
pub fn enumerate_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    Ok(Vec::new())
}
