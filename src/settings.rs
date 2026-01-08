//! Settings module - handles application settings and registry storage

#[cfg(feature = "winreg")]
use winreg::enums::*;
#[cfg(feature = "winreg")]
use winreg::RegKey;

// Re-export CsvEncoding from library
pub use sts_rust::CsvEncoding;

const REGISTRY_KEY: &str = r"Software\STS-Rust";

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
    /// Load settings from Windows registry
    #[cfg(feature = "winreg")]
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

    /// Load settings (non-Windows fallback)
    #[cfg(not(feature = "winreg"))]
    pub fn load_from_registry() -> Self {
        Self::default()
    }

    /// Save settings to Windows registry
    #[cfg(feature = "winreg")]
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

    /// Save settings (non-Windows fallback)
    #[cfg(not(feature = "winreg"))]
    pub fn save_to_registry(&self) -> Result<(), String> {
        Ok(())
    }
}

// Keep ExportSettings as alias for backward compatibility
pub type ExportSettings = AppSettings;
