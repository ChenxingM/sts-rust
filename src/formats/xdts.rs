//! XDTS format parser

use anyhow::{Context, Result};
use serde::Deserialize;
use crate::models::timesheet::{TimeSheet, CellValue};
use std::sync::OnceLock;

static RE_NUM: OnceLock<regex::Regex> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct XdtsRoot {
    #[serde(rename = "timeTables")]
    time_tables: Vec<XdtsTimeTable>,
}

#[derive(Debug, Deserialize)]
struct XdtsTimeTable {
    name: String,
    duration: usize,
    #[serde(default)]
    fields: Vec<XdtsField>,
    #[serde(rename = "timeTableHeaders")]
    time_table_headers: Vec<XdtsTimeTableHeader>,
}

#[derive(Debug, Deserialize)]
struct XdtsField {
    #[serde(rename = "fieldId")]
    field_id: u32,
    tracks: Vec<XdtsTrack>,

}

#[derive(Debug, Deserialize)]
struct XdtsTrack {
    #[serde(rename = "trackNo")]
    track_no: usize,
    frames: Vec<XdtsFrame>,
}

#[derive(Debug, Deserialize)]
struct XdtsFrame {
    frame: usize,
    data: Vec<XdtsData>,
}

#[derive(Debug, Deserialize)]
struct XdtsData {
    values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct XdtsTimeTableHeader {
    #[serde(rename = "fieldId")]
    field_id: u32,
    names: Vec<String>,
}

/// Parse XDTS file and return multiple TimeSheets (one per timeTable)
pub fn parse_xdts_file(path: &str) -> Result<Vec<TimeSheet>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read XDTS file: {}", path))?;

    // Skip first line (XDTS header)
    let json_content = content
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");

    let root: XdtsRoot = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse XDTS JSON")?;

    let mut timesheets = Vec::new();
    let re_num = RE_NUM.get_or_init(|| regex::Regex::new(r"\d+$").unwrap());

    for time_table in root.time_tables {
        if time_table.fields.is_empty() {
            continue;
        }

        let name = format!("{}->{}",
            std::path::Path::new(path).file_name().unwrap().to_string_lossy(),
            time_table.name
        );

        // Use first field's fieldId
        let field = &time_table.fields[0];
        let field_id = field.field_id;
        let tracks = &field.tracks;

        // Find names matching this fieldId
        let names = time_table.time_table_headers.iter()
            .find(|h| h.field_id == field_id)
            .map(|h| &h.names);

        if let Some(names) = names {
            let layer_count = tracks.len().max(names.len());
            let frame_count = time_table.duration;

            // Safety: Limit maximum dimensions to prevent crashes
            const MAX_LAYERS: usize = 1000;
            const MAX_FRAMES: usize = 100000;

            if layer_count > MAX_LAYERS {
                anyhow::bail!("Too many layers in XDTS file: {} (max: {})", layer_count, MAX_LAYERS);
            }
            if frame_count > MAX_FRAMES {
                anyhow::bail!("Too many frames in XDTS file: {} (max: {})", frame_count, MAX_FRAMES);
            }

            let mut timesheet = TimeSheet::new(
                name,
                24, // Default framerate
                layer_count,
                144, // Default frames per page
            );
            timesheet.ensure_frames(frame_count);

            // Set layer names
            for (i, name) in names.iter().enumerate() {
                if i < timesheet.layer_names.len() {
                    timesheet.layer_names[i] = name.clone();
                }
            }

            // Parse frame data
            for track in tracks {
                let layer_idx = track.track_no;
                if layer_idx >= layer_count {
                    continue;
                }

                // Collect keyframes (frame_idx, value)
                let mut keyframes: Vec<(usize, Option<CellValue>)> = Vec::new();
                for frame_data in &track.frames {
                    let frame_idx = frame_data.frame;
                    if frame_idx >= frame_count {
                        continue;
                    }

                    if let Some(data) = frame_data.data.first() {
                        if let Some(value_str) = data.values.first() {
                            let cell_value = if value_str == "SYMBOL_NULL_CELL" {
                                Some(CellValue::Number(0))
                            } else if value_str == "SYMBOL_TICK_1"
                                   || value_str == "SYMBOL_TICK_2"
                                   || value_str == "SYMBOL_HYPHEN" {
                                // Skip these special symbols
                                continue;
                            } else {
                                // Try to extract number from end of string
                                if let Some(captures) = re_num.find(value_str) {
                                    if let Ok(num) = captures.as_str().parse::<u32>() {
                                        Some(CellValue::Number(num))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            if let Some(cv) = cell_value {
                                keyframes.push((frame_idx, Some(cv)));
                            }
                        }
                    }
                }

                // Sort by frame index
                keyframes.sort_by_key(|k| k.0);

                // Fill all frames between keyframes
                for i in 0..keyframes.len() {
                    let (start_frame, value) = keyframes[i];
                    let end_frame = if i + 1 < keyframes.len() {
                        keyframes[i + 1].0
                    } else {
                        frame_count
                    };

                    // Safety: ensure valid range
                    if start_frame >= frame_count || end_frame > frame_count || start_frame > end_frame {
                        continue;
                    }

                    // Fill from start_frame to end_frame (exclusive)
                    for frame_idx in start_frame..end_frame {
                        if frame_idx < frame_count && layer_idx < layer_count {
                            timesheet.set_cell(layer_idx, frame_idx, value);
                        }
                    }
                }
            }

            timesheets.push(timesheet);
        }
    }

    Ok(timesheets)
}
