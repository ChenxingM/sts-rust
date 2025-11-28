//! Settings module - handles application settings and registry storage

#[cfg(feature = "winreg")]
use winreg::enums::*;
#[cfg(feature = "winreg")]
use winreg::RegKey;

// Re-export CsvEncoding from library
pub use sts_rust::CsvEncoding;

const REGISTRY_KEY: &str = r"Software\STS-Rust";

/// Application settings (combines all settings)
#[derive(Debug, Clone)]
pub struct AppSettings {
    // CSV export settings
    pub csv_header_name: String,
    pub csv_encoding: CsvEncoding,
    // Auto-save settings
    pub auto_save_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            csv_header_name: "动画".to_string(),
            csv_encoding: CsvEncoding::Gb2312,
            auto_save_enabled: false,
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
