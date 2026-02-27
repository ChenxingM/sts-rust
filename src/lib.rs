pub mod models;
pub mod formats;
// 在文件头部添加：
pub mod i18n;
pub mod theme;

/// Shared constants for resource limits
pub mod limits {
    /// Maximum number of layers allowed in a timesheet
    pub const MAX_LAYERS: usize = 1000;
    /// Maximum number of frames allowed in a timesheet
    pub const MAX_FRAMES: usize = 100_000;
}

// Re-export commonly used types
pub use models::{TimeSheet, Layer};
pub use models::timesheet::CellValue;
pub use formats::{
    parse_ae_keyframe_file, write_ae_keyframe_file,
    parse_sts_file, write_sts_file,
    parse_xdts_file, parse_tdts_file, TdtsParseResult,
    parse_csv_file, write_csv_file, write_csv_file_with_options,
    parse_sxf_file, parse_sxf_binary,
    parse_sxf_groups, write_groups_to_csv, groups_to_timesheet,
    fill_keyframes, CsvEncoding,
};
