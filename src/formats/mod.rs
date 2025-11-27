pub mod ae_keyframe;
pub mod sts;
pub mod tdts;
pub mod xdts;
pub mod csv;
pub mod sxf;

pub use ae_keyframe::{parse_ae_keyframe_file, write_ae_keyframe_file};
pub use sts::{parse_sts_file, write_sts_file};
pub use tdts::parse_tdts_file;
pub use xdts::parse_xdts_file;
pub use csv::{parse_csv_file, write_csv_file, write_csv_file_with_options, CsvEncoding};
pub use sxf::{
    parse_sxf_file,
    parse_sxf_binary,
    parse_sxf_groups,
    write_groups_to_csv,
    groups_to_timesheet,
    LayerGroup,
    LayerData,
};

use crate::models::timesheet::{TimeSheet, CellValue};

/// Fill keyframes into a timesheet layer
/// Each keyframe holds its value until the next keyframe
pub fn fill_keyframes(
    timesheet: &mut TimeSheet,
    layer_idx: usize,
    keyframes: &[(usize, Option<CellValue>)],
    frame_count: usize,
) {
    for i in 0..keyframes.len() {
        let (start_frame, value) = keyframes[i];
        let end_frame = keyframes.get(i + 1).map(|(f, _)| *f).unwrap_or(frame_count);
        for frame_idx in start_frame..end_frame {
            if frame_idx < frame_count {
                timesheet.set_cell(layer_idx, frame_idx, value);
            }
        }
    }
}
