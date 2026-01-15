//! Settings module - handles application settings storage
//! - Windows: uses registry
//! - macOS/Linux: uses config file (JSON)

#[cfg(all(windows, feature = "winreg"))]
use winreg::enums::*;
#[cfg(all(windows, feature = "winreg"))]
use winreg::RegKey;

#[cfg(all(not(windows), feature = "dirs"))]
use std::fs;
#[cfg(all(not(windows), feature = "dirs"))]
use std::path::PathBuf;

// Re-export CsvEncoding from library
pub use sts_rust::CsvEncoding;

#[cfg(all(windows, feature = "winreg"))]
const REGISTRY_KEY: &str = r"Software\STS-Rust";

#[cfg(all(not(windows), feature = "dirs"))]
const CONFIG_FILE_NAME: &str = "settings.json";
#[cfg(all(not(windows), feature = "dirs"))]
const APP_NAME: &str = "sts-rust";

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        }
    }
}

/// AE Keyframe Data version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AeKeyframeVersion {
    V6,
    V7,
    V8,
    #[default]
    V9,
}

impl AeKeyframeVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            AeKeyframeVersion::V6 => "6.0",
            AeKeyframeVersion::V7 => "7.0",
            AeKeyframeVersion::V8 => "8.0",
            AeKeyframeVersion::V9 => "9.0",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "6.0" => AeKeyframeVersion::V6,
            "7.0" => AeKeyframeVersion::V7,
            "8.0" => AeKeyframeVersion::V8,
            _ => AeKeyframeVersion::V9,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            AeKeyframeVersion::V6 => 0,
            AeKeyframeVersion::V7 => 1,
            AeKeyframeVersion::V8 => 2,
            AeKeyframeVersion::V9 => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => AeKeyframeVersion::V6,
            1 => AeKeyframeVersion::V7,
            2 => AeKeyframeVersion::V8,
            _ => AeKeyframeVersion::V9,
        }
    }
}

/// Application settings (combines all settings)
#[derive(Debug, Clone)]
pub struct AppSettings {
    // CSV export settings
    pub csv_header_name: String,
    pub csv_encoding: CsvEncoding,
    // Auto-save settings
    pub auto_save_enabled: bool,
    // Theme settings
    pub theme_mode: ThemeMode,
    // AE keyframe settings
    pub ae_keyframe_version: AeKeyframeVersion,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            csv_header_name: "动画".to_string(),
            csv_encoding: CsvEncoding::Gb2312,
            auto_save_enabled: false,
            theme_mode: ThemeMode::System,
            ae_keyframe_version: AeKeyframeVersion::V9,
        }
    }
}

impl AppSettings {
    // ========== Windows: Registry-based storage ==========

    /// Load settings from Windows registry
    #[cfg(all(windows, feature = "winreg"))]
    pub fn load_from_registry() -> Self {
        let mut settings = Self::default();

        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(REGISTRY_KEY) {
            if let Ok(header_name) = hkcu.get_value::<String, _>("CsvHeaderName") {
                settings.csv_header_name = header_name;
            }
            if let Ok(encoding) = hkcu.get_value::<String, _>("CsvEncoding") {
                settings.csv_encoding = CsvEncoding::from_str(&encoding);
            }
            if let Ok(auto_save) = hkcu.get_value::<u32, _>("AutoSaveEnabled") {
                settings.auto_save_enabled = auto_save != 0;
            }
            if let Ok(theme) = hkcu.get_value::<String, _>("ThemeMode") {
                settings.theme_mode = ThemeMode::from_str(&theme);
            }
            if let Ok(ae_version) = hkcu.get_value::<String, _>("AeKeyframeVersion") {
                settings.ae_keyframe_version = AeKeyframeVersion::from_str(&ae_version);
            }
        }

        settings
    }

    /// Save settings to Windows registry
    #[cfg(all(windows, feature = "winreg"))]
    pub fn save_to_registry(&self) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(REGISTRY_KEY)
            .map_err(|e| format!("Failed to create registry key: {}", e))?;

        key.set_value("CsvHeaderName", &self.csv_header_name)
            .map_err(|e| format!("Failed to save CsvHeaderName: {}", e))?;

        key.set_value("CsvEncoding", &self.csv_encoding.as_str())
            .map_err(|e| format!("Failed to save CsvEncoding: {}", e))?;

        key.set_value("AutoSaveEnabled", &(self.auto_save_enabled as u32))
            .map_err(|e| format!("Failed to save AutoSaveEnabled: {}", e))?;

        key.set_value("ThemeMode", &self.theme_mode.as_str())
            .map_err(|e| format!("Failed to save ThemeMode: {}", e))?;

        key.set_value("AeKeyframeVersion", &self.ae_keyframe_version.as_str())
            .map_err(|e| format!("Failed to save AeKeyframeVersion: {}", e))?;

        Ok(())
    }

    // ========== macOS/Linux: File-based storage ==========

    /// Get config file path for non-Windows platforms
    #[cfg(all(not(windows), feature = "dirs"))]
    fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join(APP_NAME).join(CONFIG_FILE_NAME))
    }

    /// Load settings from config file (macOS/Linux)
    #[cfg(all(not(windows), feature = "dirs"))]
    pub fn load_from_registry() -> Self {
        let mut settings = Self::default();

        if let Some(config_path) = Self::config_file_path() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(header_name) = json.get("csv_header_name").and_then(|v| v.as_str()) {
                        settings.csv_header_name = header_name.to_string();
                    }
                    if let Some(encoding) = json.get("csv_encoding").and_then(|v| v.as_str()) {
                        settings.csv_encoding = CsvEncoding::from_str(encoding);
                    }
                    if let Some(auto_save) = json.get("auto_save_enabled").and_then(|v| v.as_bool()) {
                        settings.auto_save_enabled = auto_save;
                    }
                    if let Some(theme) = json.get("theme_mode").and_then(|v| v.as_str()) {
                        settings.theme_mode = ThemeMode::from_str(theme);
                    }
                    if let Some(ae_version) = json.get("ae_keyframe_version").and_then(|v| v.as_str()) {
                        settings.ae_keyframe_version = AeKeyframeVersion::from_str(ae_version);
                    }
                }
            }
        }

        settings
    }

    /// Save settings to config file (macOS/Linux)
    #[cfg(all(not(windows), feature = "dirs"))]
    pub fn save_to_registry(&self) -> Result<(), String> {
        let config_path = Self::config_file_path()
            .ok_or_else(|| "Failed to get config directory".to_string())?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let json = serde_json::json!({
            "csv_header_name": self.csv_header_name,
            "csv_encoding": self.csv_encoding.as_str(),
            "auto_save_enabled": self.auto_save_enabled,
            "theme_mode": self.theme_mode.as_str(),
            "ae_keyframe_version": self.ae_keyframe_version.as_str()
        });

        let content = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }

    // ========== Fallback: No persistent storage ==========

    /// Load settings (fallback when no storage feature is enabled)
    #[cfg(not(any(all(windows, feature = "winreg"), all(not(windows), feature = "dirs"))))]
    pub fn load_from_registry() -> Self {
        Self::default()
    }

    /// Save settings (fallback when no storage feature is enabled)
    #[cfg(not(any(all(windows, feature = "winreg"), all(not(windows), feature = "dirs"))))]
    pub fn save_to_registry(&self) -> Result<(), String> {
        Ok(())
    }
}

// Keep ExportSettings as alias for backward compatibility
pub type ExportSettings = AppSettings;
