use anyhow::{Result, bail};
use crate::models::TimeSheet;

/// 解析 After Effects 关键帧文件
///
pub fn parse_ae_keyframe_file(_path: &str) -> Result<TimeSheet> {
    bail!("Use native STS format instead.")
}

/// 写入 After Effects 关键帧文件
///
pub fn write_ae_keyframe_file(_timesheet: &TimeSheet, _path: &str) -> Result<()> {
    bail!("AE keyframe export not yet implemented for X-Sheet format")
}