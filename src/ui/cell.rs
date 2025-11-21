//! Cell rendering module

use eframe::egui;
use crate::document::Document;
use sts_rust::models::timesheet::CellValue;

// 单元格渲染颜色常量
pub const BG_EDITING: egui::Color32 = egui::Color32::from_rgb(255, 255, 200);
pub const BG_SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 220, 255);
pub const BG_IN_SELECTION: egui::Color32 = egui::Color32::from_rgb(220, 235, 255);
pub const BG_NORMAL: egui::Color32 = egui::Color32::WHITE;
pub const BORDER_SELECTION: egui::Color32 = egui::Color32::from_rgb(100, 150, 255);
pub const BORDER_NORMAL: egui::Color32 = egui::Color32::GRAY;

pub const DASH: &str = "-";

/// 渲染单个单元格
#[inline]
pub fn render_cell(
    ui: &mut egui::Ui,
    doc: &mut Document,
    layer_idx: usize,
    frame_idx: usize,
    col_width: f32,
    row_height: f32,
    pointer_pos: Option<egui::Pos2>,
    pointer_down: bool,
) {
    let is_selected = doc.selection_state.selected_cell == Some((layer_idx, frame_idx));
    let is_editing = doc.edit_state.editing_cell == Some((layer_idx, frame_idx));

    let (cell_id, cell_rect) = ui.allocate_space(egui::vec2(col_width, row_height));
    let cell_response = ui.interact(
        cell_rect,
        cell_id,
        egui::Sense::click().union(egui::Sense::drag()),
    );

    if (is_selected || is_editing) && doc.selection_state.auto_scroll_to_selection {
        cell_response.scroll_to_me(None);
        doc.selection_state.auto_scroll_to_selection = false;
    }

    let is_in_selection = doc.is_cell_in_selection(layer_idx, frame_idx);

    // 合并背景和边框绘制调用
    let bg_color = if is_editing { BG_EDITING }
        else if is_selected { BG_SELECTED }
        else if is_in_selection { BG_IN_SELECTION }
        else { BG_NORMAL };

    let border_color = if is_in_selection { BORDER_SELECTION } else { BORDER_NORMAL };

    let painter = ui.painter();
    painter.rect_filled(cell_rect, 0.0, bg_color);
    painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(1.0, border_color));

    // 内容
    if is_editing {
        let text_response = ui.put(
            cell_rect,
            egui::TextEdit::singleline(&mut doc.edit_state.editing_text)
                .desired_width(col_width)
                .horizontal_align(egui::Align::Center)
                .frame(false),
        );

        text_response.request_focus();

        if text_response.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
            doc.finish_edit(false, true);
        }
    } else {
        if let Some(current_val) = doc.timesheet.get_cell(layer_idx, frame_idx) {
            let should_show_dash = frame_idx > 0 &&
                doc.timesheet.get_cell(layer_idx, frame_idx - 1)
                    .map_or(false, |prev| current_val == prev);

            let mut num_buf = itoa::Buffer::new();
            let display_text = if should_show_dash {
                DASH
            } else {
                match current_val {
                    CellValue::Number(n) => num_buf.format(*n),
                    CellValue::Same => DASH,
                }
            };

            ui.painter().text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                display_text,
                egui::FontId::monospace(11.0),
                egui::Color32::BLACK,
            );
        }
    }

    // 右键菜单
    if cell_response.secondary_clicked() {
        doc.context_menu.pos = Some((layer_idx, frame_idx));
        if let Some(pos) = cell_response.interact_pointer_pos() {
            doc.context_menu.screen_pos = pos;
        }
        if let (Some(start), Some(end)) = (doc.selection_state.selection_start, doc.selection_state.selection_end) {
            doc.context_menu.selection = Some((start, end));
        } else {
            doc.context_menu.selection = None;
        }
        if !doc.is_cell_in_selection(layer_idx, frame_idx) {
            doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
        }
    } else {
        // 左键拖拽选择
        if let Some(pos) = pointer_pos {
            if pointer_down && cell_rect.contains(pos) {
                if !doc.selection_state.is_dragging {
                    // 开始拖拽
                    doc.selection_state.is_dragging = true;
                    doc.selection_state.selection_start = Some((layer_idx, frame_idx));
                    doc.selection_state.selection_end = Some((layer_idx, frame_idx));
                    doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
                    // 退出编辑模式
                    if doc.edit_state.editing_cell.is_some() {
                        doc.edit_state.editing_cell = None;
                        doc.edit_state.editing_text.clear();
                    }
                }
            }
        }
    }

    // 拖拽中：检查指针是否在当前格子内
    if doc.selection_state.is_dragging && pointer_down {
        if let Some(pos) = pointer_pos {
            if cell_rect.contains(pos) {
                if doc.selection_state.selection_end != Some((layer_idx, frame_idx)) {
                    doc.selection_state.selection_end = Some((layer_idx, frame_idx));
                    doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
                }
            }
        }
    }
}
