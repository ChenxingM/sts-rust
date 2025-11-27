//! Settings module - handles application settings and registry storage

#[cfg(feature = "winreg")]
use winreg::enums::*;
#[cfg(feature = "winreg")]
use winreg::RegKey;

/// CSV export encoding options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvEncoding {
    Utf8,
    Gb2312,
    ShiftJis,
}

impl CsvEncoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            CsvEncoding::Utf8 => "UTF-8",
            CsvEncoding::Gb2312 => "GB2312",
            CsvEncoding::ShiftJis => "Shift-JIS",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "UTF-8" => CsvEncoding::Utf8,
            "Shift-JIS" => CsvEncoding::ShiftJis,
            _ => CsvEncoding::Gb2312,
        }
    }

    pub fn encode(&self, s: &str) -> Vec<u8> {
        match self {
            CsvEncoding::Utf8 => s.as_bytes().to_vec(),
            CsvEncoding::Gb2312 => {
                let (encoded, _, _) = encoding_rs::GBK.encode(s);
                encoded.into_owned()
            }
            CsvEncoding::ShiftJis => {
                let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(s);
                encoded.into_owned()
            }
        }
    }
}

/// Export settings
#[derive(Debug, Clone)]
pub struct ExportSettings {
    pub csv_header_name: String,
    pub csv_encoding: CsvEncoding,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            csv_header_name: "动画".to_string(),
            csv_encoding: CsvEncoding::Gb2312,
        }
    }
}

const REGISTRY_KEY: &str = r"Software\STS-Rust";

impl ExportSettings {
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
