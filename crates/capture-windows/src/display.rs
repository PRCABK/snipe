use capture_core::{CaptureError, DisplayInfo};
use domain::{DesktopPxPoint, DesktopPxRect};

#[cfg(windows)]
pub fn enumerate_displays() -> Result<Vec<DisplayInfo>, CaptureError> {
    use std::mem::size_of;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    let mut displays = Vec::new();

    unsafe extern "system" fn monitor_enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let displays_ptr = lparam.0 as *mut Vec<DisplayInfo>;
        let mut mi: MONITORINFOEXW = std::mem::zeroed();
        mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmon, &mut mi.monitorInfo as *mut _ as *mut _).as_bool() {
            let rc = mi.monitorInfo.rcMonitor;
            let width = (rc.right - rc.left).max(0) as u32;
            let height = (rc.bottom - rc.top).max(0) as u32;
            let bounds = DesktopPxRect::new(rc.left, rc.top, width, height);

            let is_primary = (mi.monitorInfo.dwFlags & 1) != 0;

            let name_len = mi.szDevice.iter().position(|&c| c == 0).unwrap_or(mi.szDevice.len());
            let name = String::from_utf16_lossy(&mi.szDevice[..name_len]);

            let mut dpi_x = 96u32;
            let mut dpi_y = 96u32;
            let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            let scale_factor = (dpi_x as f32) / 96.0;

            (*displays_ptr).push(DisplayInfo {
                id: name.clone(),
                name,
                bounds,
                scale_factor,
                is_primary,
            });
        }

        BOOL(1)
    }

    unsafe {
        let displays_ptr = &mut displays as *mut Vec<DisplayInfo>;
        EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(monitor_enum_proc),
            LPARAM(displays_ptr as isize),
        );
    }

    if displays.is_empty() {
        return Err(CaptureError::PlatformError("No active displays found".to_string()));
    }

    Ok(displays)
}

#[cfg(not(windows))]
pub fn enumerate_displays() -> Result<Vec<DisplayInfo>, CaptureError> {
    Ok(vec![DisplayInfo {
        id: "display-default".to_string(),
        name: "Mock Display".to_string(),
        bounds: DesktopPxRect::new(0, 0, 1920, 1080),
        scale_factor: 1.0,
        is_primary: true,
    }])
}

pub fn get_virtual_desktop_bounds(displays: &[DisplayInfo]) -> DesktopPxRect {
    if displays.is_empty() {
        return DesktopPxRect::new(0, 0, 1920, 1080);
    }

    let min_x = displays.iter().map(|d| d.bounds.x).min().unwrap_or(0);
    let min_y = displays.iter().map(|d| d.bounds.y).min().unwrap_or(0);
    let max_x = displays.iter().map(|d| d.bounds.right()).max().unwrap_or(1920);
    let max_y = displays.iter().map(|d| d.bounds.bottom()).max().unwrap_or(1080);

    DesktopPxRect::new(
        min_x,
        min_y,
        (max_x - min_x).max(1) as u32,
        (max_y - min_y).max(1) as u32,
    )
}
