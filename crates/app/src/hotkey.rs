use domain::HotkeyAction;
use tracing::{info, warn};

pub const HOTKEY_ID_SCREENSHOT: i32 = 1001;
pub const HOTKEY_ID_LONGSHOT: i32 = 1002;
pub const HOTKEY_ID_COLOR: i32 = 1003;
pub const HOTKEY_ID_PIN: i32 = 1004;
pub const HOTKEY_ID_SETTINGS: i32 = 1005;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub modifiers: u32,
    pub vk: u32,
}

pub fn parse_hotkey_str(s: &str) -> Option<ParsedHotkey> {
    let mut modifiers = 0u32;
    let mut vk = 0u32;

    const MOD_ALT: u32 = 0x0001;
    const MOD_CONTROL: u32 = 0x0002;
    const MOD_SHIFT: u32 = 0x0004;
    const MOD_WIN: u32 = 0x0008;

    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "meta" => modifiers |= MOD_WIN,
            // Function keys
            "f1" => vk = 0x70,
            "f2" => vk = 0x71,
            "f3" => vk = 0x72,
            "f4" => vk = 0x73,
            "f5" => vk = 0x74,
            "f6" => vk = 0x75,
            "f7" => vk = 0x76,
            "f8" => vk = 0x77,
            "f9" => vk = 0x78,
            "f10" => vk = 0x79,
            "f11" => vk = 0x7A,
            "f12" => vk = 0x7B,
            // Alphabet keys
            a if a.len() == 1 => {
                let ch = a.chars().next().unwrap().to_ascii_uppercase();
                if ch.is_ascii_alphanumeric() {
                    vk = ch as u32;
                }
            }
            "esc" | "escape" => vk = 0x1B,
            "printscreen" | "prtscn" => vk = 0x2C,
            _ => {}
        }
    }

    if vk != 0 {
        Some(ParsedHotkey { modifiers, vk })
    } else {
        None
    }
}

#[cfg(windows)]
pub fn register_global_hotkey(
    hwnd: windows::Win32::Foundation::HWND,
    id: i32,
    hotkey_str: &str,
) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, HOT_KEY_MODIFIERS};

    if let Some(parsed) = parse_hotkey_str(hotkey_str) {
        unsafe {
            let res = RegisterHotKey(
                hwnd,
                id,
                HOT_KEY_MODIFIERS(parsed.modifiers),
                parsed.vk,
            );
            if res.is_ok() {
                info!("Registered hotkey ID {} for '{}'", id, hotkey_str);
                return true;
            } else {
                warn!("Failed to register hotkey ID {} for '{}': shortcut conflict", id, hotkey_str);
                return false;
            }
        }
    }
    warn!("Invalid hotkey string: {}", hotkey_str);
    false
}

#[cfg(windows)]
pub fn unregister_global_hotkey(hwnd: windows::Win32::Foundation::HWND, id: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;
    unsafe {
        let _ = UnregisterHotKey(hwnd, id);
    }
}

pub fn map_hotkey_id_to_action(id: i32) -> Option<HotkeyAction> {
    match id {
        HOTKEY_ID_SCREENSHOT => Some(HotkeyAction::Screenshot),
        HOTKEY_ID_LONGSHOT => Some(HotkeyAction::Longshot),
        HOTKEY_ID_COLOR => Some(HotkeyAction::ColorPicker),
        HOTKEY_ID_PIN => Some(HotkeyAction::Pin),
        HOTKEY_ID_SETTINGS => Some(HotkeyAction::Settings),
        _ => None,
    }
}
