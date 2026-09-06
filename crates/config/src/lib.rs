use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error reading or writing config: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse TOML config: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Failed to serialize TOML config: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub screenshot: String,
    pub pin: String,
    pub color_picker: String,
    pub longshot: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            screenshot: "F1".to_string(),
            pin: "F2".to_string(),
            color_picker: "F3".to_string(),
            longshot: "Ctrl+Alt+S".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_save_dir: String,
    pub default_format: String,
    pub filename_template: String,
    pub jpeg_quality: u8,
}

impl Default for OutputConfig {
    fn default() -> Self {
        let default_dir = dirs::picture_dir()
            .map(|p| p.join("Snipe").to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        Self {
            default_save_dir: default_dir,
            default_format: "png".to_string(),
            filename_template: "Screenshot_{yyyy-MM-dd}_{HH-mm-ss}".to_string(),
            jpeg_quality: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub target_language: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openai-compatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_seconds: 60,
            max_retries: 2,
            target_language: "zh-CN".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub magnifier_zoom: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
            auto_start: false,
            magnifier_zoom: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

fn default_version() -> u32 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: default_version(),
            hotkeys: HotkeyConfig::default(),
            output: OutputConfig::default(),
            ai: AiConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn default_config_path() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("Snipe").join("config").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::default_config_path();
        Self::load_from_path(&path).unwrap_or_else(|err| {
            warn!(
                "Failed to load config from {}: {}. Using defaults.",
                path.display(),
                err
            );
            Self::default()
        })
    }

    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            let default_cfg = Self::default();
            let _ = default_cfg.save_to_path(path);
            return Ok(default_cfg);
        }

        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_config_path();
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        info!("Configuration saved to {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serde() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.schema_version, 1);
        assert_eq!(deserialized.hotkeys.screenshot, "F1");
        assert_eq!(deserialized.ai.model, "gpt-4o-mini");
    }
}
