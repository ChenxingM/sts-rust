// SXF (摄影表) format parser - unified text and binary support

use anyhow::{Context, Result, bail};
use crate::models::timesheet::{TimeSheet, CellValue};

// ============================================================================
// Binary Format Structures
// ============================================================================

/// Layer group information (for multi-section binary format)
#[derive(Debug)]
pub struct LayerGroup {
    pub name: String,
    pub layers: Vec<LayerData>,
}

/// Layer information
#[derive(Debug)]
pub struct LayerData {
    pub name: String,
    pub frames: Vec<String>,  // Frame values as strings (can be numbers, ○, etc.)
}

// ============================================================================
// Binary Format Parser (WBSC format with multi-section support)
// ============================================================================

/// Read a big-endian u16
fn read_u16_be(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        bail!("Offset {} + 2 exceeds data length {}", offset, data.len());
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

/// Parse SXF binary file and return groups (for 原画/台词/动画 format)
pub fn parse_sxf_groups(path: &str) -> Result<Vec<LayerGroup>> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read SXF file: {}", path))?;

    if data.len() < 20 {
        bail!("File too small: {} bytes", data.len());
    }

    // Check magic
    if &data[0..4] != b"WBSC" {
        bail!("Invalid magic: expected 'WBSC'");
    }

    // Read total frame count from header (bytes 18-19, big-endian)
    let total_frames = u16::from_be_bytes([data[18], data[19]]) as usize;

    // Find all 0xFF markers
    let mut markers = Vec::new();
    for (i, &byte) in data.iter().enumerate() {
        if byte == 0xFF {
            markers.push(i);
        }
    }

    let mut groups = Vec::new();

    // Parse section FF 03 (原画)
    if let Some(&section_03_pos) = markers.iter().find(|&&pos| pos + 1 < data.len() && data[pos + 1] == 0x03) {
        let next_marker = markers.iter().find(|&&pos| pos > section_03_pos).copied().unwrap_or(data.len());
        if let Ok(layers) = parse_layer_data_detailed(&data[section_03_pos..next_marker], total_frames) {
            groups.push(LayerGroup {
                name: "原画".to_string(),
                layers,
            });
        }
    }

    // Parse section FF 04 (动画)
    if let Some(&section_04_pos) = markers.iter().find(|&&pos| pos + 1 < data.len() && data[pos + 1] == 0x04) {
        let next_marker = markers.iter().find(|&&pos| pos > section_04_pos).copied().unwrap_or(data.len());
        if let Ok(layers) = parse_layer_data_detailed(&data[section_04_pos..next_marker], total_frames) {
            groups.push(LayerGroup {
                name: "动画".to_string(),
                layers,
            });
        }
    }

    if groups.is_empty() {
        bail!("No layer groups found");
    }

    Ok(groups)
}

/// Parse layer data from a section - returns detailed frame data expanded to total_frames
fn parse_layer_data_detailed(section_data: &[u8], total_frames: usize) -> Result<Vec<LayerData>> {
    let mut layers = Vec::new();

    // Search for all layer markers in the section
    // Format 1: 0x0B [byte] ... (e.g., 0x0B 0xEB, 0x0B 0x7D)
    // Format 2: 02 DB 00 01 [name] ... (alternative marker)
    let mut layer_positions = Vec::new();
    for (i, &byte) in section_data.iter().enumerate() {
        if i + 10 < section_data.len() {
            // Check for 0x0B marker
            if byte == 0x0B {
                layer_positions.push(i);
            }
            // Check for 02 DB 00 01 marker (alternative layer format)
            else if i + 4 < section_data.len()
                && section_data[i] == 0x02
                && section_data[i + 1] == 0xDB
                && section_data[i + 2] == 0x00
                && section_data[i + 3] == 0x01
            {
                // This is a layer marker without 0x0B prefix
                // Adjust position to align with name length location
                layer_positions.push(i);
            }
        }
    }

    // Sort and deduplicate positions
    layer_positions.sort_unstable();
    layer_positions.dedup();

    // Process each potential layer marker
    for &pos in &layer_positions {
        if layers.len() >= 10 {
            break;
        }

        // Try to parse as a layer
        if let Ok(layer) = parse_single_layer(section_data, pos, total_frames) {
            layers.push(layer);
        }
    }

    Ok(layers)
}

/// Parse a single layer starting at the given position
fn parse_single_layer(section_data: &[u8], pos: usize, total_frames: usize) -> Result<LayerData> {
    // Detect format type and read name length accordingly
    let (name_offset, name_len) = if pos + 4 < section_data.len()
        && section_data[pos] == 0x02
        && section_data[pos + 1] == 0xDB
        && section_data[pos + 2] == 0x00
        && section_data[pos + 3] == 0x01
    {
        // Format 2: 02 DB 00 01 [name]
        // Name length is at pos+2 (but it's part of "00 01" which means length 1)
        let len = section_data[pos + 3] as usize;
        if len > 0 && len < 100 {
            (pos + 4, len)
        } else {
            bail!("Invalid name length in 02 DB format: {}", len);
        }
    } else {
        // Format 1: 0x0B [byte] ... with name length at various offsets
        // Read name length (at pos + 2, or try different offsets for format variants)
        let name_len_result = read_u16_be(section_data, pos + 2);

        if let Ok(len) = name_len_result {
            if len > 0 && len < 1000 {  // Reasonable name length
                (pos + 4, len as usize)
            } else {
                // Try alternative offset (pos + 6) for different format variant
                let alt_len = read_u16_be(section_data, pos + 6)?;
                if alt_len > 0 && alt_len < 1000 {
                    (pos + 8, alt_len as usize)
                } else {
                    bail!("Invalid name length at both offsets");
                }
            }
        } else {
            bail!("Cannot read name length");
        }
    };

    if name_offset + name_len > section_data.len() {
        bail!("Name extends beyond section data");
    }

    // Read name
    let name_bytes = &section_data[name_offset..name_offset + name_len];
    let name = String::from_utf8_lossy(name_bytes).trim().to_string();

    if name.is_empty() {
        bail!("Empty layer name");
    }

    // Parse frames - each frame occupies a 40-byte slot
    let frame_data_start = name_offset + name_len;
    const FRAME_SLOT_SIZE: usize = 40;

    // Ensure we don't go beyond section boundaries
    if frame_data_start >= section_data.len() {
        bail!("Frame data start beyond section");
    }

    let available_bytes = section_data.len() - frame_data_start;
    let max_frames = (available_bytes / FRAME_SLOT_SIZE).min(total_frames);

    if max_frames == 0 {
        bail!("No frame data available");
    }

    let mut frames = Vec::with_capacity(max_frames);
    let mut last_keyframe_value = String::new();

    for frame_idx in 0..max_frames {
        let slot_start = frame_data_start + frame_idx * FRAME_SLOT_SIZE;
        let slot_end = (slot_start + FRAME_SLOT_SIZE).min(section_data.len());

        if slot_end > section_data.len() {
            break;
        }

        let slot = &section_data[slot_start..slot_end];

        // Look for frame pattern: 00 01 [value] within this 40-byte slot
        let mut found_marker = None;
        for i in 0..(slot.len() - 2) {
            if slot[i] == 0x00 && slot[i + 1] == 0x01 {
                let value_byte = slot[i + 2];
                match value_byte {
                    b'0'..=b'9' => {
                        // This is a keyframe - update last keyframe value
                        let num = ((value_byte - b'0') as u32).to_string();
                        found_marker = Some(num);
                        break;
                    }
                    0x02 => {
                        // ○ - not a keyframe, just a marker to hold previous value
                        found_marker = Some("○".to_string());
                        break;
                    }
                    0x04 => {
                        // ● - filled circle marker
                        found_marker = Some("●".to_string());
                        break;
                    }
                    0x08 => {
                        // × - cross marker (empty frame)
                        found_marker = Some("×".to_string());
                        break;
                    }
                    _ => {
                        // Invalid value byte, continue searching
                        continue;
                    }
                }
            }
        }

        // Determine frame value based on what we found
        // KEY FIX: ○, ●, × should use the last keyframe value (keyframe interpolation)
        let frame_value = match found_marker {
            Some(ref marker) if marker == "○" || marker == "●" || marker == "×" => {
                // These are not keyframes - hold the last keyframe value
                last_keyframe_value.clone()
            }
            Some(num) => {
                // This is a keyframe - use it and update last keyframe
                last_keyframe_value = num.clone();
                num
            }
            None => {
                // No marker found - hold the last keyframe value
                last_keyframe_value.clone()
            }
        };

        frames.push(frame_value);
    }

    // Extend to total_frames if needed
    while frames.len() < total_frames {
        frames.push(last_keyframe_value.clone());
    }

    Ok(LayerData { name, frames })
}

/// Write groups to CSV file in the 原画/台词/动画 format
pub fn write_groups_to_csv(groups: &[LayerGroup], path: &str) -> Result<()> {
    use std::io::Write;

    let mut output = std::fs::File::create(path)
        .with_context(|| format!("Failed to create CSV file: {}", path))?;

    // Determine max frame count
    let max_frames = groups.iter()
        .flat_map(|g| &g.layers)
        .map(|l| l.frames.len())
        .max()
        .unwrap_or(0);

    // Write first row: group headers
    write!(output, "\"Frame\"")?;
    for group in groups {
        // First column of group gets the group name, rest are empty
        write!(output, ",\"{}\"", group.name)?;
        for _ in 1..group.layers.len() {
            write!(output, ",\"\"")?;
        }
        if group.name == "原画" {
            // Add 台词 header after 原画
            write!(output, ",\"\"")?;
            write!(output, ",\"台词\"")?;
        }
    }
    writeln!(output)?;

    // Write second row: layer names
    write!(output, "\"\"")?;  // Empty under Frame
    for group in groups {
        for layer in &group.layers {
            write!(output, ",\"{}\"", layer.name)?;
        }
        if group.name == "原画" {
            // Add empty column under the separator/台词 group header
            write!(output, ",\"\"")?;
        }
    }
    writeln!(output)?;

    // Write data rows
    for frame_idx in 0..max_frames {
        write!(output, "\"{}\"", frame_idx + 1)?;  // Frame number (1-indexed)

        for group in groups {
            // For 动画 group, only write first 6 layers (skip the last F layer)
            let layer_count = if group.name == "动画" {
                group.layers.len().min(6)
            } else {
                group.layers.len()
            };

            for layer_idx in 0..layer_count {
                let value = group.layers[layer_idx].frames.get(frame_idx).map(|s| s.as_str()).unwrap_or("");
                write!(output, ",\"{}\"", value)?;
            }

            if group.name == "原画" {
                // Add 台词 column - copy from 原画 A layer
                let taci_value = group.layers.first()
                    .and_then(|l| l.frames.get(frame_idx))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                write!(output, ",\"{}\"", taci_value)?;
            }
        }

        writeln!(output)?;
    }

    Ok(())
}

/// Convert SXF groups to a single TimeSheet for GUI display
/// Combines all layers from all groups into one timesheet
pub fn groups_to_timesheet(groups: &[LayerGroup], filename: &str) -> Result<TimeSheet> {
    if groups.is_empty() {
        bail!("No groups to convert");
    }

    // Calculate total layer count and frame count
    let total_layers: usize = groups.iter().map(|g| g.layers.len()).sum();
    let frame_count = groups.first()
        .and_then(|g| g.layers.first())
        .map(|l| l.frames.len())
        .unwrap_or(0);

    if total_layers == 0 || frame_count == 0 {
        bail!("No layer data found");
    }

    // Create timesheet
    let mut timesheet = TimeSheet::new(
        filename.to_string(),
        24,  // Default framerate
        total_layers,
        frame_count.min(144) as u32,  // frames_per_page
    );
    timesheet.ensure_frames(frame_count);

    // Fill in layer names and data
    let mut layer_idx = 0;
    for group in groups {
        for layer in &group.layers {
            if layer_idx < timesheet.layer_names.len() {
                // Prefix layer name with group name for clarity
                timesheet.layer_names[layer_idx] = format!("{}_{}", group.name, layer.name);

                // Fill in frame data
                for (frame_idx, value_str) in layer.frames.iter().enumerate() {
                    if frame_idx >= frame_count {
                        break;
                    }

                    let cell_value = if value_str.is_empty() {
                        None
                    } else if value_str == "○" || value_str == "●" {
                        // Treat special markers as empty for now (can be extended)
                        None
                    } else if let Ok(num) = value_str.parse::<u32>() {
                        Some(CellValue::Number(num))
                    } else {
                        None
                    };

                    timesheet.set_cell(layer_idx, frame_idx, cell_value);
                }
            }
            layer_idx += 1;
        }
    }

    Ok(timesheet)
}

/// Parse SXF binary file and return a single TimeSheet (legacy compatibility)
pub fn parse_sxf_binary(path: &str) -> Result<TimeSheet> {
    let groups = parse_sxf_groups(path)?;

    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");

    groups_to_timesheet(&groups, filename)
}

// ============================================================================
// Text Format Parser (for legacy text-based SXF files)
// ============================================================================

/// Parse text-based SXF file and return TimeSheet
pub fn parse_sxf_file(path: &str) -> Result<TimeSheet> {
    // Try binary format first
    if let Ok(ts) = parse_sxf_binary(path) {
        return Ok(ts);
    }

    // Fall back to text format parsing
    parse_sxf_text_format(path)
}

/// Parse text-based SXF format (internal implementation)
fn parse_sxf_text_format(path: &str) -> Result<TimeSheet> {
    // Read file as binary
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read SXF file: {}", path))?;

    // Convert special bytes to readable characters
    let mut processed = Vec::with_capacity(bytes.len());
    for &b in &bytes {
        match b {
            0x01 => processed.push(b'#'),
            0x00 => processed.push(b'~'),
            0x02 => {
                // ○ - use placeholder
                processed.extend_from_slice("○".as_bytes());
            }
            0x04 => {
                // ●
                processed.extend_from_slice("●".as_bytes());
            }
            0x08 => {
                // ×
                processed.extend_from_slice("×".as_bytes());
            }
            0x20 => {}, // space, skip
            _ => processed.push(b),
        }
    }

    // Try multiple encodings to decode
    let content = crate::formats::csv::decode_with_fallback(&processed)
        .with_context(|| "Failed to decode SXF file")?;

    // Limit file size to prevent issues with large files
    const MAX_CONTENT_SIZE: usize = 1_000_000; // 1MB
    if content.len() > MAX_CONTENT_SIZE {
        bail!("SXF file too large: {} bytes (max: {})", content.len(), MAX_CONTENT_SIZE);
    }

    // Split by lines
    let lines: Vec<&str> = content.lines().collect();

    // Limit line count
    const MAX_LINES: usize = 10_000;
    if lines.len() > MAX_LINES {
        bail!("Too many lines in SXF file: {} (max: {})", lines.len(), MAX_LINES);
    }

    let mut cell_array: Vec<String> = Vec::new();
    let mut frame_array: Vec<Vec<String>> = Vec::new();

    // Process each line
    for &line in &lines {
        // Limit single line length
        if line.len() > 10_000 {
            continue;
        }

        // Look for layer name markers at end of line
        // Try to find ~~~#~~~ or ~~~~~~
        let marker_info = if let Some(pos) = line.rfind("~~~#~~~") {
            Some((pos, "~~~#~~~", '#'))
        } else if let Some(pos) = line.rfind("~~~~~~") {
            Some((pos, "~~~~~~", '~'))
        } else {
            None
        };

        // Process each line, extract layer name and frame data
        let chars_line: Vec<char> = line.chars().collect();

        let (frame_data_chars, cell_name) = if let Some((marker_pos, marker_str, _separator)) = marker_info {
            // Find marker position (character index)
            let marker_char_pos = line[..marker_pos].chars().count();

            // Extract part before marker
            let before_marker_chars = &chars_line[..marker_char_pos];

            // For ~~~~~~ marker, special processing needed
            // Pattern: [uppercase letter]+[non-#~○●× characters]* then ~~~~~~
            let (name_start_pos, name) = if marker_str == "~~~~~~" {
                let end_pos = before_marker_chars.len();

                // Search backwards for pattern: starts with uppercase + non-excluded chars
                let mut name_end = end_pos;
                let mut name_start = end_pos;

                // Find last non-excluded character (exclude #~○●×%)
                for i in (0..end_pos).rev() {
                    let ch = before_marker_chars[i];
                    if ch != '#' && ch != '~' && ch != '○' && ch != '●' && ch != '×' && ch != '%' {
                        name_end = i + 1;
                        break;
                    }
                }

                // From name_end backwards, find first uppercase letter or position before it
                for i in (0..name_end).rev() {
                    let ch = before_marker_chars[i];
                    // If we encounter excluded char, name starts after this
                    if ch == '#' || ch == '~' || ch == '○' || ch == '●' || ch == '×' || ch == '%' {
                        name_start = i + 1;
                        break;
                    }
                    // If we reach the beginning
                    if i == 0 {
                        name_start = 0;
                    }
                }

                let name: String = before_marker_chars[name_start..name_end].iter().collect();
                let name = name.trim().to_string();
                (name_start, name)
            } else {
                // For ~~~#~~~ marker, search backwards for #
                let mut name_start = before_marker_chars.len();
                for i in (0..before_marker_chars.len()).rev() {
                    if before_marker_chars[i] == '#' {
                        name_start = i;
                        break;
                    }
                }

                let name = if name_start < before_marker_chars.len() {
                    let name_chars = &before_marker_chars[name_start + 1..];
                    name_chars.iter().collect::<String>().trim().to_string()
                } else {
                    String::new()
                };
                (name_start, name)
            };

            // Frame data is characters from start of line to name start position
            let frame_chars = &before_marker_chars[..name_start_pos];
            (frame_chars.to_vec(), if name.is_empty() { None } else { Some(name) })
        } else {
            // No marker found, entire line is frame data
            (chars_line.clone(), None)
        };

        // Extract frame data (if not first row)
        if !cell_array.is_empty() && frame_data_chars.len() > 1 {
            let mut frame: Vec<String> = Vec::new();
            let char_count = frame_data_chars.len();

            // Start from position 1, extract every 10 characters
            let mut f = 1;
            let max_iter = (char_count / 10).min(1000);
            for _ in 0..max_iter {
                if f + 10 > char_count {
                    break;
                }

                // Extract 10 characters
                let slice: String = frame_data_chars[f..f + 10].iter().collect();

                // Replace BG with 1
                let slice = if slice.contains("BG") {
                    slice.replace("BG", "1")
                } else {
                    slice
                };

                // Check if contains uppercase letter (end marker)
                if slice.chars().any(|c| c.is_ascii_uppercase() && c != 'G') {
                    break;
                }

                // Extract number or special symbol
                let num: String = slice
                    .split('~')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '○' || *c == '●' || *c == '×')
                    .collect();

                frame.push(num);
                f += 10;
            }

            if !frame.is_empty() {
                frame_array.push(frame);
            }
        }

        // Add layer name
        if let Some(name) = cell_name {
            if !name.is_empty() {
                cell_array.push(name);
            }
        }
    }

    // Check data
    if cell_array.is_empty() || frame_array.is_empty() {
        bail!(
            "No valid data found in SXF file. Lines: {}, Cells found: {}, Frames found: {}",
            lines.len(),
            cell_array.len(),
            frame_array.len()
        );
    }

    // Calculate layer count and frame count
    let layer_count = cell_array.len();
    let frame_count = frame_array.get(0).map(|f| f.len()).unwrap_or(0);

    // Create TimeSheet
    let filename = std::path::Path::new(path)
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
    for (i, name) in cell_array.iter().enumerate() {
        if i < timesheet.layer_names.len() {
            timesheet.layer_names[i] = name.clone();
        }
    }

    // Fill data
    for (layer_idx, frames) in frame_array.iter().enumerate() {
        if layer_idx >= layer_count {
            continue;
        }

        for (frame_idx, value_str) in frames.iter().enumerate() {
            if frame_idx >= frame_count {
                continue;
            }

            let cell_value = if value_str.is_empty() {
                None
            } else if value_str == "×" {
                None
            } else if value_str == "○" || value_str == "●" {
                // Special symbols, treat as empty for now
                None
            } else if let Ok(num) = value_str.parse::<u32>() {
                Some(CellValue::Number(num))
            } else {
                // Try to extract number
                let num_str: String = value_str.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = num_str.parse::<u32>() {
                    Some(CellValue::Number(num))
                } else {
                    None
                }
            };

            timesheet.set_cell(layer_idx, frame_idx, cell_value);
        }
    }

    Ok(timesheet)
}
