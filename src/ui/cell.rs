//! Cell rendering module

use eframe::egui;
use crate::document::Document;
use crate::theme::ThemeConfig;

pub const DASH: &str = "-";

pub struct CellColors {
    pub bg_normal: egui::Color32,
    pub bg_hover: egui::Color32,
    
    pub bg_selected: egui::Color32,
    pub bg_in_selection: egui::Color32,
    pub bg_editing: egui::Color32,

    pub border_normal: egui::Color32,
    pub border_selection: egui::Color32,

    pub text_normal: egui::Color32,

    pub header_bg: egui::Color32,
    pub header_bg_active: egui::Color32,
    pub header_bg_hover: egui::Color32,
    pub header_text: egui::Color32,
    pub header_bg_editing: egui::Color32,
    pub frame_col_text: egui::Color32,
}

impl CellColors {
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        Self {
            bg_normal: cfg.bg_normal,
            bg_hover: cfg.bg_normal.linear_multiply(1.5), 
            
            bg_selected: cfg.bg_selected,
            bg_in_selection: cfg.bg_in_selection,
            bg_editing: cfg.bg_editing,

            border_normal: cfg.border_normal,
            border_selection: cfg.border_selection,

            text_normal: cfg.text_normal,

            header_bg: cfg.bg_header,
            header_bg_active: cfg.bg_header_active,
            header_bg_hover: cfg.bg_header_hover,
            header_text: cfg.text_header,
            
            header_bg_editing: cfg.bg_header_editing,
            frame_col_text: cfg.text_frame,
        }
    }
}

fn is_selection_bottom_right(doc: &Document, layer: usize, frame: usize) -> bool {
    if let (Some((start_l, start_f)), Some((end_l, end_f))) = 
        (doc.selection_state.selection_start, doc.selection_state.selection_end) 
    {
        let max_l = start_l.max(end_l);
        let max_f = start_f.max(end_f);
        return layer == max_l && frame == max_f;
    }
    false
}

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
) -> (bool, egui::Response) { 
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
    let is_bottom_right = is_selection_bottom_right(doc, layer_idx, frame_idx);

    let bg_color = if is_editing { colors.bg_editing }
        else if is_selected { colors.bg_selected }
        else if is_in_selection { colors.bg_in_selection }
        else if cell_response.hovered() { colors.bg_hover }
        else { colors.bg_normal };

    let border_color = if is_in_selection { colors.border_selection } else { colors.border_normal };

    let painter = ui.painter();
    painter.rect_filled(cell_rect, 0.0, bg_color);
    painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(1.0, border_color));

    if is_bottom_right && !is_editing {
        let handle_size = 6.0;
        let handle_rect = egui::Rect::from_min_size(
            cell_rect.max - egui::vec2(handle_size, handle_size) + egui::vec2(1.0, 1.0),
            egui::vec2(handle_size, handle_size)
        );

        let handle_id = cell_id.with("fill_handle");
        let handle_response = ui.interact(handle_rect, handle_id, egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Crosshair);

        ui.painter().rect_filled(handle_rect, 1.0, colors.border_selection);

        if handle_response.drag_started() {
            doc.selection_state.is_fill_dragging = true;
            doc.selection_state.fill_source_range = doc.get_selection_range();
            started_drag = true;
        }
    }

    if is_editing {
        let text_response = ui.put(
            cell_rect,
            egui::TextEdit::singleline(&mut doc.edit_state.editing_text)
                .desired_width(col_width)
                .horizontal_align(egui::Align::Center)
                .frame(false),
        );

        text_response.request_focus();

        if text_response.ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            doc.finish_edit(true, true);
        }

        let clicked_elsewhere = ui.input(|i| i.pointer.primary_clicked()) && !text_response.hovered();
        
        if (text_response.lost_focus() || clicked_elsewhere) 
            && !ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) 
        {
            doc.finish_edit(false, true);
        }
        
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            doc.edit_state.editing_cell = None;
            doc.edit_state.editing_text.clear();
        }

    } else {
        // 👇 【绝杀修复点】整合了自动横杠逻辑 + A-Z 视觉滤镜 👇
        if let Some(current_val) = doc.timesheet.get_cell(layer_idx, frame_idx) {
            let should_show_dash = frame_idx > 0 &&
                doc.timesheet.get_cell(layer_idx, frame_idx - 1)
                    .map_or(false, |prev| current_val == prev);

            let display_text = if should_show_dash { 
                DASH.to_string() 
            } else {
                // 调用我们在 document.rs 里写好的格式化滤镜
                doc.get_cell_display_string(layer_idx, frame_idx)
            };

            ui.painter().text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                display_text,
                egui::FontId::monospace(11.0),
                colors.text_normal,
            );
        }
    }

    let handle_clicked = is_bottom_right && ui.rect_contains_pointer(cell_rect) && 
        {
             if let Some(pos) = pointer_pos {
                 let handle_zone = egui::Rect::from_min_max(
                     cell_rect.max - egui::vec2(10.0, 10.0),
                     cell_rect.max
                 );
                 handle_zone.contains(pos)
             } else { false }
        };

    if cell_response.secondary_clicked() {
        if !doc.is_cell_in_selection(layer_idx, frame_idx) {
            doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            doc.selection_state.selection_start = Some((layer_idx, frame_idx));
            doc.selection_state.selection_end = Some((layer_idx, frame_idx));
        }

        doc.context_menu.pos = Some((layer_idx, frame_idx));
        if let Some(pos) = cell_response.interact_pointer_pos() {
            doc.context_menu.screen_pos = pos;
        }
        if let (Some(start), Some(end)) = (doc.selection_state.selection_start, doc.selection_state.selection_end) {
            doc.context_menu.selection = Some((start, end));
        } else {
            doc.context_menu.selection = None;
        }

    } else if !doc.selection_state.is_dragging && !doc.selection_state.is_fill_dragging {
        if cell_response.clicked() && !handle_clicked {
            if ui.input(|i| i.modifiers.shift) {
                if doc.selection_state.selection_start.is_none() {
                    doc.selection_state.selection_start = Some((layer_idx, frame_idx));
                }
                doc.selection_state.selection_end = Some((layer_idx, frame_idx));
                doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            } else {
                doc.selection_state.selection_start = Some((layer_idx, frame_idx));
                doc.selection_state.selection_end = Some((layer_idx, frame_idx));
                doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            }
            
            if doc.edit_state.editing_cell.is_some() {
                doc.edit_state.editing_cell = None;
                doc.edit_state.editing_text.clear();
            }
        }
        
        if can_start_drag && cell_response.drag_started_by(egui::PointerButton::Primary) && !handle_clicked {
            doc.selection_state.is_dragging = true;
            doc.selection_state.selection_start = Some((layer_idx, frame_idx));
            doc.selection_state.selection_end = Some((layer_idx, frame_idx));
            doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
            started_drag = true;
            if doc.edit_state.editing_cell.is_some() {
                doc.edit_state.editing_cell = None;
                doc.edit_state.editing_text.clear();
            }
        }
    }

    if (doc.selection_state.is_dragging || doc.selection_state.is_fill_dragging) && pointer_down {
        if let Some(pos) = pointer_pos {
            if cell_rect.contains(pos) {
                if doc.selection_state.selection_end != Some((layer_idx, frame_idx)) {
                    doc.selection_state.selection_end = Some((layer_idx, frame_idx));
                    doc.selection_state.selected_cell = Some((layer_idx, frame_idx));
                }
            }
        }
    }
    
    (started_drag, cell_response)
}