use anyhow::{Result, bail};
use crate::models::TimeSheet;

/// TODO: 解析 STS 文件
///
pub fn parse_sts_file(path: &str) -> Result<TimeSheet> {
    bail!("STS binary format not yet implemented.")
}

/// 写入 STS 文件
///
pub fn write_sts_file(timesheet: &TimeSheet, path: &str) -> Result<()> {
    bail!("STS binary format not yet implemented")
}
