use anyhow::{Result, bail, Context};
use crate::models::TimeSheet;
use crate::models::timesheet::CellValue;
use encoding_rs::SHIFT_JIS;
use std::fs::File;
use std::io::{Read, Write};

/// 解析 STS 文件
///
/// STS 文件格式：
/// 1. 文件头（23字节）
/// 2. 帧数据区（layer_count × frame_count × 2字节）
/// 3. 层名称区（每层：1字节长度 + N字节Shift-JIS名称）
pub fn parse_sts_file(path: &str) -> Result<TimeSheet> {
    let mut file = File::open(path)
        .with_context(|| format!("Unable to open: {}", path))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Unable to read: {}", path))?;

    if buffer.len() < 23 {
        bail!("Invalid STS file: too small");
    }

    // 解析文件头
    if buffer[0] != 0x11 {
        bail!("Invalid STS file: invalid signature");
    }

    let header_str = std::str::from_utf8(&buffer[1..18])
        .context("Invalid STS file: invalid header")?;
    if header_str != "ShiraheiTimeSheet" {
        bail!("Invalid STS file: invalid header");
    }

    let layer_count = buffer[18] as usize;
    let frame_count = u16::from_le_bytes([buffer[19], buffer[20]]) as usize;

    if layer_count == 0 || frame_count == 0 {
        bail!("Invalid STS file: invalid layer count or frame count: {} layers, {} frames", layer_count, frame_count);
    }

    // 计算帧数据区大小
    let frame_data_size = layer_count * frame_count * 2;
    let frame_data_end = 23 + frame_data_size;

    if buffer.len() < frame_data_end {
        bail!("Invalid STS file: incomplete frame data");
    }

    // 解析帧数据
    let mut cells = vec![vec![None; frame_count]; layer_count];

    for layer in 0..layer_count {
        for frame in 0..frame_count {
            let offset = 23 + (layer * frame_count + frame) * 2;
            let cell_value = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);

            if cell_value > 0 {
                cells[layer][frame] = Some(CellValue::Number(cell_value as u32));
            }
        }
    }

    // 解析层名称
    let mut layer_names = Vec::new();
    let mut pos = frame_data_end;

    for layer_idx in 0..layer_count {
        if pos >= buffer.len() {
            // 如果名称区不完整，使用默认名称
            layer_names.push(format!("Layer{}", layer_idx + 1));
            continue;
        }

        let name_len = buffer[pos] as usize;
        pos += 1;

        if pos + name_len > buffer.len() {
            layer_names.push(format!("Layer{}", layer_idx + 1));
            break;
        }

        let name_bytes = &buffer[pos..pos + name_len];
        let (name_str, _, _) = SHIFT_JIS.decode(name_bytes);
        layer_names.push(name_str.to_string());

        pos += name_len;
    }

    // 补齐缺失的层名称
    while layer_names.len() < layer_count {
        layer_names.push(format!("Layer{}", layer_names.len() + 1));
    }

    // 提取文件名作为sheet名称
    let sheet_name = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sheet1")
        .to_string();

    Ok(TimeSheet {
        name: sheet_name,
        framerate: 24,  // 默认24fps
        frames_per_page: 144,  // 默认每页144帧
        layer_count,
        layer_names,
        cells,
        source_width: 640,
        source_height: 480,
        source_pixel_aspect_ratio: 1.0,
        comp_pixel_aspect_ratio: 1.0,
    })
}

/// 写入 STS 文件
///
pub fn write_sts_file(timesheet: &TimeSheet, path: &str) -> Result<()> {
    let layer_count = timesheet.layer_count;
    let frame_count = timesheet.total_frames();

    if layer_count > 255 {
        bail!("Too many layers: {}, maximum is 255", layer_count);
    }

    if frame_count > 65535 {
        bail!("Too many frames: {}, maximum is 65535", frame_count);
    }

    let mut file = File::create(path)
        .with_context(|| format!("Unable to create: {}", path))?;

    // === 文件头 (23 bytes) ===

    // STS 标识符
    file.write_all(&[0x11])?;

    // 固定字符串 "ShiraheiTimeSheet"
    file.write_all(b"ShiraheiTimeSheet")?;

    // 层数 (1 byte)
    file.write_all(&[layer_count as u8])?;

    // 帧数 (2 bytes, little-endian)
    file.write_all(&(frame_count as u16).to_le_bytes())?;

    // 填充 (2 bytes)
    file.write_all(&[0x00, 0x00])?;

    // === 帧数据区 (layer_count × frame_count × 2 bytes) ===
    for layer in 0..layer_count {
        for frame in 0..frame_count {
            let cell_value = match timesheet.get_actual_value(layer, frame) {
                Some(n) => n as u16,
                None => 0u16,
            };
            file.write_all(&cell_value.to_le_bytes())?;
        }
    }

    // === 层名称区 ===
    for layer in 0..layer_count {
        let name = &timesheet.layer_names[layer];

        // 编码为 Shift-JIS
        let (name_bytes, _, had_errors) = SHIFT_JIS.encode(name);

        if had_errors {
            eprintln!("Warning: Layer name '{}' contains character that cannot encode to Shift-JIS", name);
        }

        let name_bytes = if name_bytes.len() > 255 {
            eprintln!("Warning: Layer name over 255 characters will be truncated: '{}'", name);
            &name_bytes[..255]
        } else {
            &name_bytes
        };

        // 写入: [1字节长度][N字节名称]
        file.write_all(&[name_bytes.len() as u8])?;
        file.write_all(name_bytes)?;
    }

    Ok(())
}
