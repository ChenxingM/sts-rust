// src/ui/curve_editor.rs

use eframe::egui;
use crate::document::Document;
use crate::i18n::Translation; 

pub struct CurveEditor {
    pub control_p1: egui::Pos2,
    pub control_p2: egui::Pos2,
    pub start_value: u32,
    pub num_drawings: u32,
    pub default_duration: u32,
}

impl Default for CurveEditor {
    fn default() -> Self {
        Self {
            control_p1: egui::pos2(0.33, 0.0),
            control_p2: egui::pos2(0.66, 1.0),
            start_value: 1,
            num_drawings: 3,
            default_duration: 12, 
        }
    }
}

impl CurveEditor {
    pub fn new() -> Self {
        Self::default()
    }

    // 👇 魔法 1：接收主题配置 theme: &crate::theme::ThemeConfig
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document, is_open: &mut bool, text: &Translation, theme: &crate::theme::ThemeConfig) {
        if !*is_open { return; }

        egui::Window::new(text.curve_title)
            .open(is_open)
            .resizable(false)
            .order(egui::Order::Foreground)
            .default_size([320.0, 500.0])
            .show(ctx, |ui| {
                
                let active_target = if let Some((min_l, min_f, _max_l, max_f)) = doc.get_selection_range() {
                    Some((min_l, min_f, (max_f - min_f + 1) as usize))
                } else if let Some((l, f)) = doc.selection_state.selected_cell {
                    Some((l, f, self.default_duration as usize))
                } else {
                    None
                };

                ui.heading(text.curve_section_selection);
                ui.separator();

                if let Some((layer_idx, start_frame, length)) = active_target {
                    let layer_name = doc.timesheet.layer_names.get(layer_idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Layer {}", layer_idx + 1));
                    
                    let end_frame = start_frame + length - 1;

                    egui::Grid::new("curve_info").num_columns(2).spacing([12.0, 6.0]).striped(true).show(ui, |ui| {
                        ui.label(text.curve_target_layer);
                        // 👇 魔法 2：废除刺眼的 LIGHT_BLUE，使用温和抗压缩的 theme.text_timecode
                        ui.label(egui::RichText::new(format!("{} (L{})", layer_name, layer_idx + 1)).monospace().strong().color(theme.text_timecode));
                        ui.end_row();

                        ui.label(text.curve_frame_range);
                        // 👇 魔法 3：废除刺眼的 LIGHT_GREEN，同样统一为 theme.text_timecode
                        ui.label(egui::RichText::new(format!("{} ➡ {} ({}f)", start_frame + 1, end_frame + 1, length)).monospace().strong().color(theme.text_timecode));
                        ui.end_row();
                    });
                } else {
                    ui.label(egui::RichText::new(text.curve_no_selection).color(egui::Color32::from_gray(120)));
                    ui.small(text.curve_no_selection_tip);
                }
                
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // === 曲线控制 ===
                ui.horizontal(|ui| {
                    if ui.button(text.curve_btn_linear).clicked() { self.set_curve(0.17, 0.17, 0.83, 0.83); }
                    if ui.button(text.curve_btn_ease_in).clicked() { self.set_curve(0.42, 0.0, 1.0, 1.0); }
                    if ui.button(text.curve_btn_ease_out).clicked() { self.set_curve(0.0, 0.0, 0.58, 1.0); }
                    if ui.button(text.curve_btn_ease_in_out).clicked() { self.set_curve(0.42, 0.0, 0.58, 1.0); }
                });
                
                ui.add_space(10.0);
                self.draw_curve_graph(ui, theme); // 顺便把 theme 也传给图表，以便未来你可能想改网格线颜色
                ui.add_space(10.0);
                
                // === 参数设置 ===
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(text.curve_label_start);
                        ui.add(egui::DragValue::new(&mut self.start_value).range(0..=9999));
                    });
                    
                    let has_range_selection = doc.get_selection_range().is_some();
                    ui.horizontal(|ui| {
                        ui.label(text.curve_label_duration);
                        if has_range_selection {
                            if let Some((_, _, len)) = active_target {
                                let mut l = len as u32;
                                ui.add_enabled(false, egui::DragValue::new(&mut l).suffix(" f"));
                            }
                        } else {
                            ui.add(egui::DragValue::new(&mut self.default_duration).range(1..=10000).suffix(" f"));
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(text.curve_label_drawings);
                        ui.add(egui::DragValue::new(&mut self.num_drawings).range(1..=1000));
                    });

                    if self.duration() > 0 && self.num_drawings > 0 {
                        let ratio = self.duration() as f32 / self.num_drawings as f32;
                        ui.small(format_curve_ratio(text.curve_info_ratio, ratio));
                    }
                });

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("P1:");
                    ui.add(egui::DragValue::new(&mut self.control_p1.x).speed(0.01).range(0.0..=1.0));
                    ui.add(egui::DragValue::new(&mut self.control_p1.y).speed(0.01).range(0.0..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("P2:");
                    ui.add(egui::DragValue::new(&mut self.control_p2.x).speed(0.01).range(0.0..=1.0));
                    ui.add(egui::DragValue::new(&mut self.control_p2.y).speed(0.01).range(0.0..=1.0));
                });

                ui.separator();

                // === 应用按钮 ===
                if ui.add_enabled(active_target.is_some(), egui::Button::new(text.curve_btn_apply)).clicked() {
                    if let Some((layer, start, len)) = active_target {
                        doc.set_keyframe_curve(
                            layer, 
                            start, 
                            self.control_p1, 
                            self.control_p2, 
                            self.start_value, 
                            self.num_drawings,
                            len as u32
                        );
                        doc.is_modified = true;
                    }
                }
            });
    }

    fn duration(&self) -> u32 {
        self.default_duration
    }

    fn set_curve(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.control_p1 = egui::pos2(x1, y1);
        self.control_p2 = egui::pos2(x2, y2);
    }

    fn draw_curve_graph(&mut self, ui: &mut egui::Ui, theme: &crate::theme::ThemeConfig) {
        let canvas_size = egui::vec2(220.0, 220.0);
        let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::drag());
        let rect = response.rect;
        
        // 使用更温和的背景色
        painter.rect_filled(rect, 0.0, theme.bg_header_active); 
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, theme.border_normal));
        
        for i in 1..4 {
            let t = i as f32 / 4.0;
            let x = rect.left() + rect.width() * t;
            let y = rect.bottom() - rect.height() * t;
            painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], egui::Stroke::new(1.0, egui::Color32::from_gray(60)));
            painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(1.0, egui::Color32::from_gray(60)));
        }
        
        let to_screen = |pos: egui::Pos2| -> egui::Pos2 {
            egui::pos2(rect.left() + pos.x * rect.width(), rect.bottom() - pos.y * rect.height())
        };
        
        let p0 = egui::pos2(0.0, 0.0);
        let p3 = egui::pos2(1.0, 1.0);
        let cp1_scr = to_screen(self.control_p1);
        let cp2_scr = to_screen(self.control_p2);
        
        painter.line_segment([to_screen(p0), cp1_scr], egui::Stroke::new(1.5, egui::Color32::from_gray(120)));
        painter.line_segment([to_screen(p3), cp2_scr], egui::Stroke::new(1.5, egui::Color32::from_gray(120)));
        
        let curve = egui::epaint::CubicBezierShape::from_points_stroke(
            [to_screen(p0), cp1_scr, cp2_scr, to_screen(p3)], false, egui::Color32::TRANSPARENT, 
            egui::Stroke::new(2.5, theme.text_timecode), // 曲线本身也统一用时间轴颜色！
        );
        painter.add(curve);
        
        let handle_radius = 6.0;
        painter.circle_filled(cp1_scr, handle_radius, theme.text_timecode);
        painter.circle_filled(cp2_scr, handle_radius, theme.text_timecode);
        
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let drag_delta = response.drag_delta();
                let delta_x = drag_delta.x / rect.width();
                let delta_y = -drag_delta.y / rect.height();
                if pointer_pos.distance(cp1_scr) < 30.0 {
                    self.control_p1.x = (self.control_p1.x + delta_x).clamp(0.0, 1.0);
                    self.control_p1.y = (self.control_p1.y + delta_y).clamp(0.0, 1.0);
                } else if pointer_pos.distance(cp2_scr) < 30.0 {
                    self.control_p2.x = (self.control_p2.x + delta_x).clamp(0.0, 1.0);
                    self.control_p2.y = (self.control_p2.y + delta_y).clamp(0.0, 1.0);
                }
            }
        }
    }
}

fn format_curve_ratio(template: &str, ratio: f32) -> String {
    template.replace("{:.1}", &format!("{:.1}", ratio))
}