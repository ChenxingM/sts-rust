//! TDTS format parser

use anyhow::{Context, Result};
use serde::Deserialize;
use crate::models::timesheet::{TimeSheet, CellValue};
use crate::limits::{MAX_LAYERS, MAX_FRAMES};
use super::fill_keyframes;

#[derive(Debug, Deserialize)]
struct TdtsRoot {
    #[serde(rename = "timeSheets")]
    time_sheets: Vec<TdtsTimeSheet>,
}

#[derive(Debug, Deserialize)]
struct TdtsTimeSheet {
    header: TdtsHeader,
    #[serde(rename = "timeTables")]
    time_tables: Vec<TdtsTimeTable>,
}

#[derive(Debug, Deserialize)]
struct TdtsHeader {
    cut: String,
}

#[derive(Debug, Deserialize)]
struct TdtsTimeTable {
    name: String,
    duration: usize,
    #[serde(default)]
    fields: Vec<TdtsField>,
    #[serde(rename = "timeTableHeaders")]
    time_table_headers: Vec<TdtsTimeTableHeader>,
}

#[derive(Debug, Deserialize)]
struct TdtsField {
    #[serde(rename = "fieldId")]
    field_id: u32,
    tracks: Vec<TdtsTrack>,
}

#[derive(Debug, Deserialize)]
struct TdtsTrack {
    #[serde(rename = "trackNo")]
    track_no: usize,
    frames: Vec<TdtsFrame>,
}

#[derive(Debug, Deserialize)]
struct TdtsFrame {
    frame: i32,
    data: Vec<TdtsData>,
}

#[derive(Debug, Deserialize)]
struct TdtsData {
    values: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TdtsTimeTableHeader {
    #[serde(rename = "fieldId")]
    field_id: u32,
    names: Vec<String>,
}

/// Parse result containing timesheets and warnings
pub struct TdtsParseResult {
    pub timesheets: Vec<TimeSheet>,
    pub warnings: Vec<String>,
}

/// Parse TDTS file and return multiple TimeSheets (one per timeTable)
pub fn parse_tdts_file(path: &str) -> Result<TdtsParseResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read TDTS file: {}", path))?;

    // Skip first line (TDTS header)
    let json_content = content
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");

    let root: TdtsRoot = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse TDTS JSON")?;

    let mut timesheets = Vec::new();
    let mut warnings = Vec::new();

    for time_sheet in root.time_sheets {
        let cut_name = &time_sheet.header.cut;

        for time_table in time_sheet.time_tables {
            if time_table.fields.is_empty() {
                continue;
            }

            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled");
            let name = format!("{}->{}->{}", file_name, cut_name, time_table.name);

            // Find field with fieldId = 4
            let tracks = time_table.fields.iter()
                .find(|f| f.field_id == 4)
                .map(|f| &f.tracks);

            // Find names with fieldId = 4
            let names = time_table.time_table_headers.iter()
                .find(|h| h.field_id == 4)
                .map(|h| &h.names);

            if let (Some(tracks), Some(names)) = (tracks, names) {
                let layer_count = tracks.len().max(names.len());
                let frame_count = time_table.duration;

                if layer_count > MAX_LAYERS {
                    anyhow::bail!("Too many layers in TDTS file: {} (max: {})", layer_count, MAX_LAYERS);
                }
                if frame_count > MAX_FRAMES {
                    anyhow::bail!("Too many frames in TDTS file: {} (max: {})", frame_count, MAX_FRAMES);
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
                        if frame_data.frame < 0 {
                            warnings.push(format!("Negative frame {} found, skipping", frame_data.frame));
                            continue;
                        }
                        let frame_idx = frame_data.frame as usize;
                        if frame_idx >= frame_count {
                            continue;
                        }

                        if let Some(data) = frame_data.data.first() {
                            if let Some(value_str) = data.values.first() {
                                let cell_value = if value_str == "SYMBOL_NULL_CELL" {
                                    None
                                } else if let Ok(num) = value_str.parse::<u32>() {
                                    Some(CellValue::Number(num))
                                } else {
                                    None
                                };
                                keyframes.push((frame_idx, cell_value));
                            }
                        }
                    }

                    // Sort by frame index and fill
                    keyframes.sort_by_key(|k| k.0);
                    fill_keyframes(&mut timesheet, layer_idx, &keyframes, frame_count);
                }

                timesheets.push(timesheet);
            }
        }
    }

    Ok(TdtsParseResult { timesheets, warnings })
}
