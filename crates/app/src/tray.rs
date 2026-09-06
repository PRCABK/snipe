use domain::HotkeyAction;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub const WM_TRAYICON: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 100;

pub const TRAY_MENU_SCREENSHOT: usize = 2001;
pub const TRAY_MENU_LONGSHOT: usize = 2002;
pub const TRAY_MENU_COLOR: usize = 2003;
pub const TRAY_MENU_SETTINGS: usize = 2004;
pub const TRAY_MENU_EXIT: usize = 2005;

pub enum SystemMessage {
    Hotkey(HotkeyAction),
    TrayAction(usize),
    ShutdownRequested,
    ExplorerRestarted,
}

#[cfg(windows)]
pub struct Win32MessageWindow {
    pub hwnd: windows::Win32::Foundation::HWND,
}

#[cfg(windows)]
impl Win32MessageWindow {
    pub fn new(sender: UnboundedSender<SystemMessage>) -> Result<Self, String> {
        use std::mem::size_of;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, RegisterClassW, RegisterWindowMessageW,
            SetWindowLongPtrW, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, GWLP_USERDATA,
        };

        let class_name: Vec<u16> = "Snipe_MessageWindowClass\0".encode_utf16().collect();
        let taskbar_msg: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();

        unsafe {
            let taskbar_msg_id = RegisterWindowMessageW(PCWSTR(taskbar_msg.as_ptr()));
            let hinstance = GetModuleHandleW(None).map_err(|e| e.to_string())?;

            unsafe extern "system" fn wnd_proc(
                hwnd: HWND,
                msg: u32,
                wparam: WPARAM,
                lparam: LPARAM,
            ) -> LRESULT {
                use windows::Win32::UI::WindowsAndMessaging::{
                    GetWindowLongPtrW, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_RBUTTONUP,
                };

                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let sender = &*(ptr as *const UnboundedSender<SystemMessage>);

                    if msg == WM_HOTKEY {
                        let id = wparam.0 as i32;
                        if let Some(action) = crate::hotkey::map_hotkey_id_to_action(id) {
                            let _ = sender.send(SystemMessage::Hotkey(action));
                        }
                        return LRESULT(0);
                    } else if msg == WM_TRAYICON {
                        let event = (lparam.0 & 0xffff) as u32;
                        if event == WM_RBUTTONUP {
                            show_tray_menu(hwnd);
                        }
                        return LRESULT(0);
                    } else if msg == WM_COMMAND {
                        let menu_id = (wparam.0 & 0xffff) as usize;
                        let _ = sender.send(SystemMessage::TrayAction(menu_id));
                        return LRESULT(0);
                    } else if msg == WM_DESTROY {
                        let _ = sender.send(SystemMessage::ShutdownRequested);
                        return LRESULT(0);
                    }
                }

                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            let wnd_class = WNDCLASSW {
                style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance.into(),
                hIcon: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
                hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR::default(),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
            };

            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                WS_POPUP,
                0,
                0,
                0,
                0,
                HWND::default(),
                windows::Win32::UI::WindowsAndMessaging::HMENU::default(),
                hinstance,
                None,
            ).map_err(|e| e.to_string())?;

            let boxed_sender = Box::new(sender);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed_sender) as isize);

            // Add Tray Icon
            add_tray_icon(hwnd);

            Ok(Self { hwnd })
        }
    }
}

#[cfg(windows)]
pub fn add_tray_icon(hwnd: windows::Win32::Foundation::HWND) {
    use std::mem::size_of;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIM_ADD, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};

    unsafe {
        let hicon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;

        let tip = "Snipe 截图工具\0".encode_utf16().collect::<Vec<u16>>();
        let copy_len = tip.len().min(nid.szTip.len());
        nid.szTip[..copy_len].copy_from_slice(&tip[..copy_len]);

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        info!("Added system tray icon");
    }
}

#[cfg(windows)]
pub fn remove_tray_icon(hwnd: windows::Win32::Foundation::HWND) {
    use std::mem::size_of;
    use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIM_DELETE, NOTIFYICONDATAW};

    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        info!("Removed system tray icon");
    }
}

#[cfg(windows)]
fn show_tray_menu(hwnd: windows::Win32::Foundation::HWND) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
        TrackPopupMenu, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    };

    unsafe {
        let menu = CreatePopupMenu().unwrap();
        let s1: Vec<u16> = "截取屏幕 (F1)\0".encode_utf16().collect();
        let s2: Vec<u16> = "滚动长截图 (Ctrl+Alt+S)\0".encode_utf16().collect();
        let s3: Vec<u16> = "屏幕取色 (F3)\0".encode_utf16().collect();
        let s4: Vec<u16> = "偏好设置\0".encode_utf16().collect();
        let s5: Vec<u16> = "退出 Snipe\0".encode_utf16().collect();

        let _ = AppendMenuW(menu, MF_STRING, TRAY_MENU_SCREENSHOT, PCWSTR(s1.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, TRAY_MENU_LONGSHOT, PCWSTR(s2.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, TRAY_MENU_COLOR, PCWSTR(s3.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, TRAY_MENU_SETTINGS, PCWSTR(s4.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, TRAY_MENU_EXIT, PCWSTR(s5.as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        SetForegroundWindow(hwnd);
        TrackPopupMenu(menu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, None);
        let _ = DestroyMenu(menu);
    }
}
