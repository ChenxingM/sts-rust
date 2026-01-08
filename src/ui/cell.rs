//! Cell rendering module

use eframe::egui;
use crate::document::Document;
use sts_rust::models::timesheet::CellValue;

pub const DASH: &str = "-";

/// Theme-aware colors for cells
pub struct CellColors {
    pub bg_editing: egui::Color32,
    pub bg_selected: egui::Color32,
    pub bg_in_selection: egui::Color32,
    pub bg_normal: egui::Color32,
    pub border_selection: egui::Color32,
    pub border_normal: egui::Color32,
    pub text_color: egui::Color32,
    // Header colors
    pub header_bg: egui::Color32,
    pub header_bg_editing: egui::Color32,
    pub header_text: egui::Color32,
    // Frame number column colors
    pub frame_col_text: egui::Color32,
}

impl CellColors {
    pub fn from_visuals(visuals: &egui::Visuals) -> Self {
        if visuals.dark_mode {
            // Dark theme colors
            Self {
                bg_editing: egui::Color32::from_rgb(80, 80, 50),
                bg_selected: egui::Color32::from_rgb(60, 80, 120),
                bg_in_selection: egui::Color32::from_rgb(50, 65, 90),
                bg_normal: egui::Color32::from_rgb(35, 35, 35),
                border_selection: egui::Color32::from_rgb(100, 150, 255),
                border_normal: egui::Color32::from_rgb(80, 80, 80),
                text_color: egui::Color32::from_rgb(220, 220, 220),
                header_bg: egui::Color32::from_rgb(50, 50, 50),
                header_bg_editing: egui::Color32::from_rgb(80, 80, 50),
                header_text: egui::Color32::from_rgb(200, 200, 200),
                frame_col_text: egui::Color32::from_rgb(150, 150, 150),
            }
        } else {
            // Light theme colors
            Self {
                bg_editing: egui::Color32::from_rgb(255, 255, 200),
                bg_selected: egui::Color32::from_rgb(200, 220, 255),
                bg_in_selection: egui::Color32::from_rgb(220, 235, 255),
                bg_normal: egui::Color32::WHITE,
                border_selection: egui::Color32::from_rgb(100, 150, 255),
                border_normal: egui::Color32::GRAY,
                text_color: egui::Color32::BLACK,
                header_bg: egui::Color32::from_rgb(240, 240, 240),
                header_bg_editing: egui::Color32::from_rgb(255, 255, 200),
                header_text: egui::Color32::BLACK,
                frame_col_text: egui::Color32::DARK_GRAY,
            }
        }
    }
}

/// 渲染单个单元格
/// `can_start_drag`: 是否允许开始新的拖拽（防止多窗口同时拖拽）
/// 返回值：是否开始了新的拖拽
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
    colors: &CellColors,
    can_start_drag: bool,
) -> bool {
    let mut started_drag = false;
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
    let bg_color = if is_editing { colors.bg_editing }
        else if is_selected { colors.bg_selected }
        else if is_in_selection { colors.bg_in_selection }
        else { colors.bg_normal };

    let border_color = if is_in_selection { colors.border_selection } else { colors.border_normal };

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
                colors.text_color,
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
    } else if !doc.selection_state.is_dragging {
        // 单击选择 - 使用 egui 响应系统（考虑窗口层级）
        if cell_response.clicked() {
            doc.selection_state.selection_start = Some((layer_idx, frame_idx));
            doc.selection_state.selection_end = Some((layer_idx, frame_idx));
            doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            // 退出编辑模式
            if doc.edit_state.editing_cell.is_some() {
                doc.edit_state.editing_cell = None;
                doc.edit_state.editing_text.clear();
            }
        }
        // 拖拽选择开始 - 使用 egui 响应系统（考虑窗口层级）
        if can_start_drag && cell_response.drag_started_by(egui::PointerButton::Primary) {
            doc.selection_state.is_dragging = true;
            doc.selection_state.selection_start = Some((layer_idx, frame_idx));
            doc.selection_state.selection_end = Some((layer_idx, frame_idx));
            doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            started_drag = true;
            // 退出编辑模式
            if doc.edit_state.editing_cell.is_some() {
                doc.edit_state.editing_cell = None;
                doc.edit_state.editing_text.clear();
            }
        }
    }

    // 拖拽中：检查指针是否在当前格子内（只有正在拖拽的文档会处理）
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

    started_drag
}
