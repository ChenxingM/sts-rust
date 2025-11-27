//! CSV format parser for animation timesheets

use anyhow::{Context, Result};
use crate::models::timesheet::{TimeSheet, CellValue};
use std::path::Path;

/// Try to decode bytes with multiple encodings
pub(crate) fn decode_with_fallback(bytes: &[u8]) -> Result<String> {
    // Try encodings in order: UTF-8, GBK (GB2312), Shift-JIS
    let encodings = [
        encoding_rs::UTF_8,
        encoding_rs::GBK,
        encoding_rs::SHIFT_JIS,
    ];

    for encoding in &encodings {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if !had_errors {
            return Ok(decoded.into_owned());
        }
    }

    // If all fail, force decode with UTF-8 (replacing invalid chars)
    let (decoded, _, _) = encoding_rs::UTF_8.decode(bytes);
    Ok(decoded.into_owned())
}

/// Parse CSV file and return TimeSheet
///
/// CSV Format:
/// - First row: headers (Frame, layer names...)
/// - Second row: layer labels (ignored for Frame column, used as layer names)
/// - Data rows: Frame number in first column, values in subsequent columns
///
/// Value rules:
/// - Number: Set cell to that number
/// - Empty string: Hold previous frame's value (including None after ×)
/// - "×": Set cell to None (empty), and subsequent empty strings continue to hold None
pub fn parse_csv_file(path: &str) -> Result<TimeSheet> {
    // Read raw bytes
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read CSV file: {}", path))?;

    // Decode with multiple encoding support
    let content = decode_with_fallback(&bytes)
        .with_context(|| "Failed to decode CSV file")?;

    let mut reader = csv::Reader::from_reader(content.as_bytes());

    // Read all records first
    let records: Vec<csv::StringRecord> = reader.records()
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "Failed to parse CSV")?;

    if records.len() < 2 {
        anyhow::bail!("CSV file must have at least 2 rows (header + layer names)");
    }

    // First row is headers, second row contains layer names
    let layer_name_row = &records[0];
    let data_rows = &records[1..];

    // Count layers (exclude first column which is Frame)
    let layer_count = layer_name_row.len().saturating_sub(1);
    if layer_count == 0 {
        anyhow::bail!("CSV file must have at least one layer column");
    }

    // Determine frame count from data rows
    let frame_count = data_rows.len();

    // Safety: Limit maximum dimensions to prevent crashes
    const MAX_LAYERS: usize = 1000;
    const MAX_FRAMES: usize = 100000;

    if layer_count > MAX_LAYERS {
        anyhow::bail!("Too many layers in CSV file: {} (max: {})", layer_count, MAX_LAYERS);
    }
    if frame_count > MAX_FRAMES {
        anyhow::bail!("Too many frames in CSV file: {} (max: {})", frame_count, MAX_FRAMES);
    }

    // Extract layer names from first row (skip "Frame" column)
    let layer_names: Vec<String> = (1..layer_name_row.len())
        .map(|i| layer_name_row.get(i).unwrap_or("").to_string())
        .collect();

    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");

    let mut timesheet = TimeSheet::new(
        filename.to_string(),
        24,
        layer_count,
        144,
    );
    timesheet.ensure_frames(frame_count);

    // Set layer names
    for (i, name) in layer_names.iter().enumerate() {
        if i < timesheet.layer_names.len() {
            timesheet.layer_names[i] = name.clone();
        }
    }

    // Parse data rows
    // Track the last value for each layer (for hold logic)
    let mut last_values: Vec<Option<CellValue>> = vec![None; layer_count];

    for (frame_idx, record) in data_rows.iter().enumerate() {
        // Process each layer column (skip Frame column at index 0)
        for layer_idx in 0..layer_count {
            let col_idx = layer_idx + 1; // +1 because first column is Frame
            let cell_str = record.get(col_idx).unwrap_or("").trim();

            let new_value = if cell_str == "×" {
                // × means None (empty)
                None
            } else if cell_str.is_empty() {
                // Empty string: hold previous value
                last_values[layer_idx]
            } else {
                // Try to parse as number
                if let Ok(num) = cell_str.parse::<u32>() {
                    Some(CellValue::Number(num))
                } else {
                    // If not a number, treat as hold
                    last_values[layer_idx]
                }
            };

            // Update last value for this layer
            last_values[layer_idx] = new_value;

            // Set cell in timesheet
            timesheet.set_cell(layer_idx, frame_idx, new_value);
        }
    }

    Ok(timesheet)
}

/// CSV export encoding options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvEncoding {
    Utf8,
    Gb2312,
    ShiftJis,
}

impl CsvEncoding {
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

/// Write TimeSheet to CSV file with custom header and encoding
/// Only outputs keyframes (when value changes), uses "×" for transition to empty
pub fn write_csv_file_with_options(
    timesheet: &TimeSheet,
    path: &str,
    header_name: &str,
    encoding: CsvEncoding,
) -> Result<()> {
    use std::io::Write;

    let mut csv_content = String::new();

    // First row: Frame, header_name, empty cells...
    csv_content.push_str("Frame,");
    csv_content.push_str(header_name);
    for _ in 1..timesheet.layer_count {
        csv_content.push(',');
    }
    csv_content.push('\n');

    // Second row: empty, layer names...
    csv_content.push(',');
    for (i, layer_name) in timesheet.layer_names.iter().enumerate() {
        csv_content.push_str(layer_name);
        if i < timesheet.layer_count - 1 {
            csv_content.push(',');
        }
    }
    csv_content.push('\n');

    // Track previous actual values for each layer
    let mut prev_values: Vec<Option<u32>> = vec![None; timesheet.layer_count];

    // Data rows
    let frame_count = timesheet.total_frames();
    for frame_idx in 0..frame_count {
        // Frame number (1-indexed)
        csv_content.push_str(&(frame_idx + 1).to_string());

        for layer_idx in 0..timesheet.layer_count {
            csv_content.push(',');

            // Get the actual value for this cell
            let current_value = timesheet.get_actual_value(layer_idx, frame_idx);
            let prev_value = prev_values[layer_idx];

            if current_value != prev_value {
                // Value changed - output it
                match current_value {
                    Some(n) => csv_content.push_str(&n.to_string()),
                    None => {
                        // Changed from having a value to no value - output ×
                        if prev_value.is_some() {
                            csv_content.push('×');
                        }
                    }
                }
                prev_values[layer_idx] = current_value;
            }
            // If value is the same as previous, output nothing (empty)
        }
        csv_content.push('\n');
    }

    // Encode and write to file
    let encoded_bytes = encoding.encode(&csv_content);
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create CSV file: {}", path))?;
    file.write_all(&encoded_bytes)
        .with_context(|| "Failed to write CSV file")?;

    Ok(())
}

/// Write TimeSheet to CSV file (legacy function for compatibility)
pub fn write_csv_file(timesheet: &TimeSheet, path: &str) -> Result<()> {
    write_csv_file_with_options(timesheet, path, "动画", CsvEncoding::Gb2312)
}
