use serde::{Deserialize, Serialize};
use crate::coordinates::DesktopPxRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    Screenshot,
    Longshot,
    ColorPicker,
    Pin,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppMode {
    Idle,
    Capturing,
    ColorPicking,
    LongshotCapturing,
    SettingsOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppCommand {
    TriggerScreenshot,
    TriggerLongshot,
    TriggerColorPicker,
    OpenSettings,
    TogglePauseHotkeys,
    ExitApp,
    SelectionConfirmed(DesktopPxRect),
    SelectionCancelled,
    PinCreated { id: String },
    ClosePin { id: String },
}
