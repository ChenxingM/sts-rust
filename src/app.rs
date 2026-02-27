//! App module - main application logic and UI

use eframe::egui;
use std::path::Path;
// 👇 引入了新的 LayerType
use crate::document::{Document, LayerType};
use crate::ui::{render_cell, CellColors, AboutDialog, SequencePlayer, CurveEditor}; 
use crate::settings::{ExportSettings, CsvEncoding, AeKeyframeVersion}; 
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

use crate::i18n::{self, Language, Translation};
use crate::theme::ThemeConfig;
use crate::video_utils;

pub struct StsApp {
    pub documents: Vec<Document>,
    pub next_doc_id: usize,
    pub active_doc_id: Option<usize>,
    pub dragging_doc_id: Option<usize>,
    pub show_new_dialog: bool,
    pub new_dialog_focus_name: bool,
    pub closing_doc_id: Option<usize>,
    pub clearing_doc_id: Option<usize>,
    pub new_name: String,
    pub new_framerate: u32,
    pub new_layer_count: usize,
    pub new_frames_per_page: u32,
    pub new_seconds: u32,
    pub new_frames: u32,
    pub error_message: Option<String>,
    
    pub status_message: Option<(String, egui::Color32)>,
    pub status_timer: f64, // 新增：消息倒计时

    pub show_exit_dialog: bool,
    pub allowed_to_close: bool,
    pub settings: ExportSettings,
    pub show_settings_dialog: bool,
    pub temp_csv_header_name: String,
    pub temp_csv_encoding: usize,
    pub temp_auto_save_enabled: bool,
    pub temp_ae_keyframe_version: usize,
    pub about_dialog: AboutDialog,

    pub language: Language,
    
    pub current_theme: ThemeConfig,
    pub temp_theme_name: String,
    pub available_themes: Vec<ThemeConfig>,
    pub custom_theme_name_input: String,
    pub first_frame: bool, 

    pub player: SequencePlayer, 
    pub curve_editor: CurveEditor, 
    pub show_curve_window: bool,
}

impl Default for StsApp {
    fn default() -> Self {
        let settings = ExportSettings::load_from_registry();
        let temp_encoding = match settings.csv_encoding {
            CsvEncoding::Utf8 => 0,
            CsvEncoding::Gb2312 => 1,
            CsvEncoding::ShiftJis => 2,
        };

        let mut available_themes = vec![
            ThemeConfig::miona_rira(),
            ThemeConfig::light(),
            ThemeConfig::dark(),
            ThemeConfig::sts_pro(),
            ThemeConfig::ae_classic(),
            ThemeConfig::cyberpunk(),
            ThemeConfig::eye_care(),
        ];

        let theme_dir = Path::new("themes");
        if theme_dir.exists() {
            available_themes.extend(ThemeConfig::load_all_custom_themes(theme_dir));
        }

        let current_theme = available_themes
            .iter()
            .find(|t| t.name == settings.theme_name)
            .cloned()
            .unwrap_or_else(|| ThemeConfig::miona_rira());

        let temp_theme_name = current_theme.name.clone();

        Self {
            documents: Vec::new(),
            next_doc_id: 0,
            active_doc_id: None,
            dragging_doc_id: None,
            show_new_dialog: false,
            new_dialog_focus_name: false,
            closing_doc_id: None,
            clearing_doc_id: None,
            new_name: "sheet1".to_string(),
            new_framerate: 24,
            new_layer_count: 12,
            new_frames_per_page: 144,
            new_seconds: 6,
            new_frames: 0,
            error_message: None,
            status_message: None,
            status_timer: 0.0,
            show_exit_dialog: false,
            allowed_to_close: false,
            temp_csv_header_name: settings.csv_header_name.clone(),
            temp_csv_encoding: temp_encoding,
            temp_auto_save_enabled: settings.auto_save_enabled,
            temp_ae_keyframe_version: settings.ae_keyframe_version.index(),
            settings,
            show_settings_dialog: false,
            about_dialog: AboutDialog::default(),
            
            language: Language::Zh, 
            
            current_theme,
            temp_theme_name,
            available_themes,
            custom_theme_name_input: String::new(),
            first_frame: true,

            player: SequencePlayer::new(),
            curve_editor: CurveEditor::new(),
            show_curve_window: false,
        }
    }
}

impl StsApp {
    fn static_text(lang: Language) -> &'static Translation {
        match lang {
            Language::En => &i18n::EN_US,
            Language::Zh => &i18n::ZH_CN,
            Language::Ja => &i18n::JA_JP,
        }
    }

    fn set_success_message(&mut self, msg: String) {
        self.status_message = Some((msg, egui::Color32::from_rgb(100, 255, 100))); // 绿色提示
        self.status_timer = 3.5; // 3.5秒后消失
    }

    fn set_error_message(&mut self, msg: String) {
        self.status_message = Some((msg, egui::Color32::from_rgb(255, 100, 100))); // 警告红色
        self.status_timer = 3.5; // 3.5秒后消失
        }

    pub fn create_new_document(&mut self) {
        let total_frames = (self.new_seconds * self.new_framerate + self.new_frames) as usize;
        let mut ts = TimeSheet::new(
            self.new_name.clone(),
            self.new_framerate,
            self.new_layer_count,
            self.new_frames_per_page,
        );
        ts.ensure_frames(total_frames.max(1));
        let doc = Document::new(self.next_doc_id, ts, None);
        self.next_doc_id += 1;
        self.documents.push(doc);
        self.show_new_dialog = false;
    }

    fn load_file_from_path(&mut self, path_str: &str) {
        const MAX_DOCUMENTS: usize = 100;
        if self.documents.len() >= MAX_DOCUMENTS {
            self.set_error_message(format!("Too many documents open (max: {}).", MAX_DOCUMENTS));
            return;
        }
        if let Some(_existing) = self.documents.iter().find(|d| {
            d.file_path.as_ref().map_or(false, |p| p.as_ref() == path_str)
        }) {
            self.set_error_message(format!("File is already open: {}", path_str));
            return;
        }

        let extension = std::path::Path::new(path_str).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match extension.as_str() {
            "sts" => match sts_rust::parse_sts_file(path_str) {
                Ok(ts) => {
                    let doc = Document::new(self.next_doc_id, ts, Some(path_str.to_string()));
                    self.next_doc_id += 1;
                    self.documents.push(doc);
                    self.status_message = None;
                }
                Err(e) => { self.set_error_message(format!("Failed to open: {}", e)); }
            },
            "xdts" => match sts_rust::parse_xdts_file(path_str) {
                Ok(timesheets) => {
                    if timesheets.is_empty() { self.set_error_message("No timesheets found".to_string()); }
                    else {
                        for ts in timesheets {
                            let doc = Document::new(self.next_doc_id, ts, None);
                            self.next_doc_id += 1;
                            self.documents.push(doc);
                        }
                        self.status_message = None;
                    }
                }
                Err(e) => { self.set_error_message(format!("Failed to open: {}", e)); }
            },
            "tdts" => match sts_rust::parse_tdts_file(path_str) {
                Ok(result) => {
                    if result.timesheets.is_empty() { self.set_error_message("No timesheets found".to_string()); }
                    else {
                        for ts in result.timesheets {
                            let doc = Document::new(self.next_doc_id, ts, None);
                            self.next_doc_id += 1;
                            self.documents.push(doc);
                        }
                        self.status_message = None;
                    }
                }
                Err(e) => { self.set_error_message(format!("Failed to open: {}", e)); }
            },
            "csv" => match sts_rust::parse_csv_file(path_str) {
                Ok(ts) => {
                    let doc = Document::new(self.next_doc_id, ts, None);
                    self.next_doc_id += 1;
                    self.documents.push(doc);
                    self.status_message = None;
                }
                Err(e) => { self.set_error_message(format!("Failed to open: {}", e)); }
            },
            "sxf" => match sts_rust::parse_sxf_groups(path_str) {
                Ok(groups) => {
                    let filename = std::path::Path::new(path_str).file_name().and_then(|n| n.to_str()).unwrap_or("untitled");
                    match sts_rust::groups_to_timesheet(&groups, filename) {
                        Ok(ts) => {
                            let doc = Document::new(self.next_doc_id, ts, None);
                            self.next_doc_id += 1;
                            self.documents.push(doc);
                            self.status_message = None;
                        }
                        Err(e) => { self.set_error_message(format!("Failed to convert SXF: {}", e)); }
                    }
                }
                Err(e) => { self.set_error_message(format!("Failed to open SXF: {}", e)); }
            },
            _ => { self.set_error_message(format!("Unsupported file type: {}", extension)); }
        }
    }

    pub fn open_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Supported", &["sts", "xdts", "tdts", "csv", "sxf"])
            .add_filter("STS Files", &["sts"])
            .pick_file() {
            self.load_file_from_path(path.to_str().unwrap());
        }
    }

    pub fn save_document(&mut self, doc_id: usize) {
        let text = Self::static_text(self.language); // 获取字典
        if let Some(doc) = self.documents.iter_mut().find(|d| d.id == doc_id) {
            if doc.file_path.is_some() {
                if let Err(e) = doc.save() { self.set_error_message(e); } 
                else { self.set_success_message(text.msg_saved.to_string()); } 
            } else {
                self.save_document_as(doc_id);
            }
        }
    }

    pub fn save_document_as(&mut self, doc_id: usize) {
        let text = Self::static_text(self.language); // 获取字典
        let default_name = self.documents.iter().find(|d| d.id == doc_id).map(|d| format!("{}.sts", d.timesheet.name)).unwrap_or_else(|| "untitled.sts".to_string());
        if let Some(path) = rfd::FileDialog::new().add_filter("STS Files", &["sts"]).set_file_name(&default_name).save_file() {
            let path_str = path.to_str().unwrap().to_string();
            if let Some(doc) = self.documents.iter_mut().find(|d| d.id == doc_id) {
                if let Err(e) = doc.save_as(path_str) { self.set_error_message(e); } 
                else { self.set_success_message(text.msg_saved.to_string()); } // 
            }
        }
    }

    pub fn export_to_csv(&mut self, doc_id: usize) {
        let default_name = self.documents.iter().find(|d| d.id == doc_id).map(|d| format!("{}.csv", d.timesheet.name)).unwrap_or_else(|| "export.csv".to_string());
        if let Some(path) = rfd::FileDialog::new().add_filter("CSV Files", &["csv"]).set_file_name(&default_name).save_file() {
            let path_str = path.to_str().unwrap();
            if let Some(doc) = self.documents.iter().find(|d| d.id == doc_id) {
                match sts_rust::write_csv_file_with_options(&doc.timesheet, path_str, &self.settings.csv_header_name, self.settings.csv_encoding) {
                    Ok(_) => { self.set_success_message(format!("Exported to CSV: {}", path_str)); }
                    Err(e) => { self.set_error_message(format!("Failed to export CSV: {}", e)); }
                }
            }
        }
    }

    fn apply_theme(ctx: &egui::Context, theme: &ThemeConfig) {
        let mut visuals = if theme.is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        
        visuals.window_fill = theme.bg_normal;
        visuals.panel_fill = theme.bg_normal;
        visuals.faint_bg_color = theme.bg_header;
        visuals.extreme_bg_color = theme.bg_header_active;
        
        visuals.window_rounding = egui::Rounding::same(6.0);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 4.0, 
            spread: 2.0,
            color: egui::Color32::from_black_alpha(30),
        };
        visuals.popup_shadow = visuals.window_shadow;
        
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.window_margin = egui::Margin::same(6.0);
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(14.0));
        ctx.set_style(style);
    }

    fn calculate_string(input: &str) -> Option<String> {
        let text = input.trim();
        if text.is_empty() { return None; }
        let chars: Vec<char> = text.chars().collect();
        let mut op_idx = None;
        let mut op_char = ' ';
        for (i, &c) in chars.iter().enumerate() {
            if i > 0 && "+-*/".contains(c) {
                op_idx = Some(i); op_char = c; break;
            }
        }
        if let Some(idx) = op_idx {
            let left_str = &text[0..idx].trim();
            let right_str = &text[idx+1..].trim();
            let left_val = if left_str.is_empty() { 0.0 } else { left_str.parse::<f64>().ok()? };
            let right_val = right_str.parse::<f64>().ok()?;
            let result = match op_char {
                '+' => left_val + right_val, '-' => left_val - right_val, '*' => left_val * right_val, '/' => if right_val != 0.0 { left_val / right_val } else { left_val }, _ => return None,
            };
            return Some(result.round().to_string());
        }
        None
    }

    fn ui_drag_edit(ui: &mut egui::Ui, label: &str, value_str: &mut String, speed: f64) -> bool {
        let mut enter_consumed = false;
        ui.horizontal(|ui| {
            let label_resp = ui.add(egui::Label::new(label).sense(egui::Sense::drag()));
            if label_resp.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
            if label_resp.dragged() {
                let current_val = value_str.parse::<f64>().unwrap_or(0.0);
                let delta = label_resp.drag_delta().x as f64 * speed;
                let new_val = (current_val + delta).round().max(0.0);
                *value_str = new_val.to_string();
            }
            let text_resp = ui.add(egui::TextEdit::singleline(value_str).desired_width(60.0));
            if text_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(result) = Self::calculate_string(value_str) {
                    *value_str = result;
                    enter_consumed = true; 
                }
            }
        });
        enter_consumed
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        if self.first_frame {
            Self::apply_theme(ctx, &self.current_theme);
            self.first_frame = false;
        }

        let text = Self::static_text(self.language);
        if self.status_timer > 0.0 {
            self.status_timer -= ctx.input(|i| i.stable_dt) as f64;
            if self.status_timer <= 0.0 {
                self.status_message = None; // 时间到，清空消息
            } else {
                ctx.request_repaint(); // 强制 UI 每帧刷新，保证倒计时平滑进行
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.on_close_event() { ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose); }
        }

        if self.show_new_dialog {
             egui::Area::new(egui::Id::new("modal_dimmer")).fixed_pos(egui::pos2(0.0, 0.0)).order(egui::Order::Background).show(ctx, |ui| {
                ui.painter().rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 200));
             });
             egui::Window::new(text.action_new).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0,0.0]).order(egui::Order::Foreground).show(ctx, |ui| {
                  ui.horizontal(|ui| { ui.label(text.label_name); let r = ui.text_edit_singleline(&mut self.new_name); if self.new_dialog_focus_name { r.request_focus(); self.new_dialog_focus_name = false; }});
                  ui.horizontal(|ui| { ui.label(text.label_layers); ui.add(egui::DragValue::new(&mut self.new_layer_count).range(1..=1000)); });
                  
                  ui.horizontal(|ui| { 
                      ui.label(text.label_fps); 
                      ui.radio_value(&mut self.new_framerate, 24, "24"); 
                      ui.radio_value(&mut self.new_framerate, 30, "30"); 
                      ui.separator();
                      ui.label("Custom:");
                      ui.add(egui::DragValue::new(&mut self.new_framerate).range(1..=240).suffix(" fps"));
                  });

                  ui.horizontal(|ui| { ui.label(text.label_f_per_page); ui.add(egui::DragValue::new(&mut self.new_frames_per_page).range(12..=288)); });
                  ui.separator();
                  ui.horizontal(|ui| { 
                      ui.label(text.label_duration); ui.add(egui::DragValue::new(&mut self.new_seconds).range(0..=3600).suffix("s")); 
                      ui.label("+"); ui.add(egui::DragValue::new(&mut self.new_frames).range(0..=self.new_framerate - 1).suffix("k"));
                  });
                  let total = self.new_seconds * self.new_framerate + self.new_frames;
                  let pages = if total == 0 { 0 } else { (total + self.new_frames_per_page - 1) / self.new_frames_per_page };
                  ui.horizontal(|ui| { ui.label(format!("→ {} : {} f", text.label_total, total)); ui.separator(); ui.label(format!("{} : {}", text.label_pages, pages)); });
                  ui.separator();
                  
                  ui.horizontal(|ui| {
                      if ui.button(text.btn_create).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.create_new_document(); }
                      if ui.button(text.btn_cancel).clicked() { self.show_new_dialog = false; }
                  });
             });
        }

        if self.show_exit_dialog {
            let unsaved_docs: Vec<String> = self.documents.iter().filter(|d| d.is_modified && d.is_open).map(|d| d.timesheet.name.clone()).collect();
            egui::Area::new(egui::Id::new("exit_modal_dimmer")).fixed_pos(egui::pos2(0.0, 0.0)).order(egui::Order::Background).show(ctx, |ui| {
                ui.painter().rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 150));
            });
            let mut action: Option<i32> = None;
            egui::Window::new(text.dialog_unsaved_title).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).order(egui::Order::Foreground).show(ctx, |ui| {
                ui.label(format!("{} {}", unsaved_docs.len(), text.dialog_unsaved_body));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(text.btn_save_all).clicked() { action = Some(0); }
                    if ui.button(text.btn_discard_all).clicked() { action = Some(1); }
                    if ui.button(text.btn_cancel).clicked() { action = Some(2); }
                });
            });
            match action {
                Some(0) => {
                    let doc_ids: Vec<usize> = self.documents.iter().filter(|d| d.is_modified && d.is_open).map(|d| d.id).collect();
                    for doc_id in doc_ids { self.save_document(doc_id); }
                    self.show_exit_dialog = false; self.allowed_to_close = true; ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(1) => { self.show_exit_dialog = false; self.allowed_to_close = true; ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                Some(2) => { self.show_exit_dialog = false; }
                _ => {}
            }
        }

        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) { self.show_new_dialog = true; self.new_dialog_focus_name = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) { self.open_document(); }
        });

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path { if let Some(path_str) = path.to_str() { self.load_file_from_path(path_str); } }
                }
            }
        });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(text.menu_file, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend); 
                    
                    // 👇 加入 .shortcut_text() 魔法 👇
                    if ui.add(egui::Button::new(text.action_new).shortcut_text("Ctrl+N")).clicked() { self.show_new_dialog = true; self.new_dialog_focus_name = true; ui.close_menu(); }
                    if ui.add(egui::Button::new(text.action_open).shortcut_text("Ctrl+O")).clicked() { self.open_document(); ui.close_menu(); }
                    ui.separator();
                    // 关闭全部没有快捷键，保持原样
                    if ui.add(egui::Button::new(text.action_close_all)).clicked() { self.documents.clear(); ui.close_menu(); }
                });
                
                ui.menu_button(text.menu_edit, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if ui.add(egui::Button::new(text.action_settings)).clicked() {
                        // ... 设置相关代码保持不变 ...
                        self.temp_csv_header_name = self.settings.csv_header_name.clone();
                        self.temp_csv_encoding = match self.settings.csv_encoding { CsvEncoding::Utf8 => 0, CsvEncoding::Gb2312 => 1, CsvEncoding::ShiftJis => 2 };
                        self.temp_auto_save_enabled = self.settings.auto_save_enabled;
                        self.temp_ae_keyframe_version = self.settings.ae_keyframe_version.index();
                        self.show_settings_dialog = true;
                        ui.close_menu();
                    }
                });
                
                ui.menu_button(text.menu_help, |ui| {
                    // 👇 同样注入自适应魔法
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    
                    if ui.button(text.action_about).clicked() { self.about_dialog.open = true; ui.close_menu(); }
                });
            });
        });

        if self.show_settings_dialog {
             let mut should_save = false; let mut should_cancel = false;
             egui::Window::new(text.settings_title)
                .collapsible(false).resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0,0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                 ui.heading(text.settings_csv);
                 ui.horizontal(|ui| { ui.label("Header name:"); ui.text_edit_singleline(&mut self.temp_csv_header_name); });
                 ui.horizontal(|ui| {
                     ui.label("Encoding:");
                     egui::ComboBox::from_id_salt("encoding").selected_text(match self.temp_csv_encoding { 0=>"UTF-8", 1=>"GB2312", 2=>"Shift-JIS", _=>"?" })
                     .show_ui(ui, |ui| { ui.selectable_value(&mut self.temp_csv_encoding, 0, "UTF-8"); ui.selectable_value(&mut self.temp_csv_encoding, 1, "GB2312"); ui.selectable_value(&mut self.temp_csv_encoding, 2, "Shift-JIS"); });
                 });
                 ui.separator();
                 ui.heading(text.settings_general);
                 
                 ui.checkbox(&mut self.temp_auto_save_enabled, text.settings_autosave);
                 
                 ui.heading(text.settings_appearance);
                 ui.horizontal(|ui| {
                     ui.label(text.settings_language);
                     ui.radio_value(&mut self.language, Language::En, "English");
                     ui.radio_value(&mut self.language, Language::Zh, "中文");
                     ui.radio_value(&mut self.language, Language::Ja, "日本語");
                 });

                 ui.horizontal(|ui| {
                     ui.label("Theme:");
                     egui::ComboBox::from_id_salt("theme_combo")
                         .selected_text(&self.temp_theme_name)
                         .show_ui(ui, |ui| {
                             for t in &self.available_themes {
                                 if ui.selectable_value(&mut self.temp_theme_name, t.name.clone(), &t.name).clicked() {
                                     self.current_theme = t.clone();
                                     Self::apply_theme(ctx, &self.current_theme); 
                                 }
                             }
                         });
                 });

                 // 
                 ui.collapsing(text.theme_customize, |ui| {
                     let mut theme_dirty = false;
                     
                     {
                         let t = &mut self.current_theme;
                         ui.horizontal(|ui| { 
                             ui.label(text.theme_base_mode); 
                             if ui.checkbox(&mut t.is_dark, text.theme_dark_mode).changed() { theme_dirty = true; } 
                         });

                         egui::Grid::new("color_picker_grid").num_columns(4).spacing([15.0, 4.0]).show(ui, |ui| {
                             let mut color_row = |ui: &mut egui::Ui, label: &str, color: &mut egui::Color32| {
                                 ui.label(label);
                                 if ui.color_edit_button_srgba(color).changed() { theme_dirty = true; }
                             };
                             
                             color_row(ui, "Background", &mut t.bg_normal);
                             color_row(ui, "Border Normal", &mut t.border_normal);
                             ui.end_row();

                             color_row(ui, "Header", &mut t.bg_header);
                             color_row(ui, "Border Select", &mut t.border_selection);
                             ui.end_row();

                             color_row(ui, "Header Hover", &mut t.bg_header_hover);
                             color_row(ui, "Text Normal", &mut t.text_normal);
                             ui.end_row();

                             color_row(ui, "Header Active", &mut t.bg_header_active);
                             color_row(ui, "Text Header", &mut t.text_header);
                             ui.end_row();

                             color_row(ui, "BG Selected", &mut t.bg_selected);
                             color_row(ui, "Text Frame", &mut t.text_frame);
                             ui.end_row();
                             
                             color_row(ui, "BG Editing", &mut t.bg_editing);
                             color_row(ui, "Timecode Text", &mut t.text_timecode);
                             ui.end_row();

                             color_row(ui, "BG In Select", &mut t.bg_in_selection);
                         });
                     } 

                     if theme_dirty { Self::apply_theme(ctx, &self.current_theme); }

                     ui.separator();
                     
                     ui.horizontal(|ui| {
                         ui.label(text.theme_save_as);
                         ui.add(egui::TextEdit::singleline(&mut self.custom_theme_name_input).desired_width(120.0));
                         if ui.button(text.theme_save_btn).clicked() {
                             if !self.custom_theme_name_input.trim().is_empty() {
                                 
                                 self.current_theme.name = self.custom_theme_name_input.trim().to_string();
                                 let theme_to_save = self.current_theme.clone();
                                 
                                 let dir = Path::new("themes");
                                 let _ = std::fs::create_dir_all(dir);
                                 let path = dir.join(format!("{}.json", theme_to_save.name));
                                 
                                 if let Ok(_) = theme_to_save.save_to_file(&path) {
                                     self.available_themes.retain(|x| x.name != theme_to_save.name); 
                                     self.available_themes.push(theme_to_save.clone());
                                     
                                     self.temp_theme_name = theme_to_save.name.clone();
                                     self.custom_theme_name_input.clear();
                                     self.set_success_message(format!("Theme saved as {}", theme_to_save.name));
                                 } else {
                                     self.set_error_message("Failed to save JSON".to_string());
                                 }
                             }
                         }
                     });
                 });
                

                 ui.horizontal(|ui| {
                     ui.label("AE Keyframe:");
                     egui::ComboBox::from_id_salt("ae_version").selected_text(AeKeyframeVersion::from_index(self.temp_ae_keyframe_version).as_str())
                     .show_ui(ui, |ui| { ui.selectable_value(&mut self.temp_ae_keyframe_version, 0, "6.0"); ui.selectable_value(&mut self.temp_ae_keyframe_version, 1, "7.0"); });
                 });
                 ui.separator();
                 ui.horizontal(|ui| { if ui.button(text.btn_ok).clicked() { should_save = true; } if ui.button(text.btn_cancel).clicked() { should_cancel = true; } });
             });
             
             if should_save {
                 self.settings.csv_header_name = self.temp_csv_header_name.clone();
                 self.settings.csv_encoding = match self.temp_csv_encoding { 1 => CsvEncoding::Gb2312, 2 => CsvEncoding::ShiftJis, _ => CsvEncoding::Utf8 };
                 self.settings.auto_save_enabled = self.temp_auto_save_enabled;
                 self.settings.theme_name = self.temp_theme_name.clone(); 
                 self.settings.ae_keyframe_version = AeKeyframeVersion::from_index(self.temp_ae_keyframe_version);
                 let _ = self.settings.save_to_registry();
                 self.show_settings_dialog = false;
             }
             if should_cancel { 
                 if let Some(t) = self.available_themes.iter().find(|t| t.name == self.settings.theme_name) {
                     self.current_theme = t.clone();
                     Self::apply_theme(ctx, &self.current_theme);
                 }
                 self.show_settings_dialog = false; 
             }
        }
        
        self.about_dialog.show(ctx);

        if let Some((msg, color)) = &self.status_message {
            egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| { 
                ui.colored_label(*color, msg); 
            });
        }

        egui::CentralPanel::default().show(ctx, |_ui| {});

        let mut docs_to_save = Vec::new();
        let mut docs_to_save_as = Vec::new();
        let mut docs_to_close = Vec::new();

        let num_docs = self.documents.len();
        
        for doc_idx in 0..num_docs {
            let mut pending_import_video = None; 

            let (window_title, doc_id_val, is_open_before) = {
                let doc = &self.documents[doc_idx];
                let title = if doc.jump_step > 1 { format!("{} [Step: {}]", doc.title(), doc.jump_step) } else { doc.title() };
                (title, doc.id, doc.is_open)
            };

            if !is_open_before { continue; }
            let mut window_open = true;
            
            // 智能计算当前表需要多宽 👇
            // 公式：左侧帧数栏(36) + (轨道数 * 每列宽度36) + 右侧留一点点呼吸空间(40)
            let layer_count = self.documents[doc_idx].timesheet.layer_count;
            let ideal_width = 36.0 + (layer_count as f32 * 36.0) + 40.0;
            
            egui::Window::new(&window_title)
                .id(egui::Id::new(format!("doc_{}", doc_id_val)))
                .open(&mut window_open)
                .resizable(true)
                .default_width(ideal_width) // <--- 用智能宽度替换掉 800.0
                .default_height(600.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                        // 1. 保存与导出 (第一个参数传 false，代表它只是个点击按钮，不保持常亮)
                        if ui.selectable_label(false, text.btn_save)
                            .on_hover_text(format!("{} (Ctrl+S)", text.btn_save)) // 鼠标放上去才显示
                            .clicked() { docs_to_save.push(doc_id_val); }
                            
                        if ui.selectable_label(false, text.action_save_as)
                            .on_hover_text(format!("{} (Ctrl+Shift+S)", text.action_save_as))
                            .clicked() { docs_to_save_as.push(doc_id_val); }
                        
                        ui.separator();
                        
                        // 顺手给导出和清空也加上优雅的说明提示，为以后换图标做准备
                        if ui.selectable_label(false, text.action_export)
                            .on_hover_text(text.hover_export)
                            .clicked() { self.export_to_csv(doc_id_val); }
                            
                        ui.add(egui::Separator::default().vertical());
                        
                        if ui.selectable_label(false, text.btn_clear_all)
                            .on_hover_text(text.hover_clear)
                            .clicked() { self.clearing_doc_id = Some(doc_id_val); }

                        ui.separator();
                        let doc = &mut self.documents[doc_idx];
                        
                        // 2. 状态切换类按钮 (因为它们有开/关状态，所以第一个参数传变量，保持原样)
                        let player_text = if doc.show_player { text.btn_player_close } else { text.btn_player_open };
                        if ui.selectable_label(doc.show_player, player_text).clicked() {
                            doc.show_player = !doc.show_player;
                        }

                        if ui.selectable_label(self.show_curve_window, text.btn_curve_tool).clicked() {
                            self.show_curve_window = !self.show_curve_window;
                            self.active_doc_id = Some(doc_id_val); 
                        }

                        ui.separator();
                        
                        // 3. 导入参考视频按钮 (同样传 false)
                        let import_text = match self.language { Language::En => "Import Video", Language::Zh => "导入视频", Language::Ja => "動画の読み込み" };
                        if ui.selectable_label(false, import_text).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Video", &["mp4", "mov", "avi", "mkv"])
                                .pick_file() {
                                pending_import_video = Some(path);
                            }
                        }
                    });

                    ui.separator();
                    
                    let (current_doc_name, total_frames, selection_info, _framerate) = {
                        let doc = &self.documents[doc_idx];
                        let fps = doc.timesheet.framerate.max(1); 
                        
                        let info = if let Some((layer, frame)) = doc.selection_state.selected_cell {
                            let layer_name = doc.timesheet.layer_names.get(layer).cloned().unwrap_or("?".to_string());
                            let frames_pp = doc.timesheet.frames_per_page.max(1) as usize;
                            let page_num = frame / frames_pp + 1;
                            let abs_frame = frame + 1; 
                            
                            let sec = frame as u32 / fps;
                            let rem = frame as u32 % fps;
                            let time_code = format!("{} + {}", sec, rem);

                            Some((layer_name, frame, abs_frame, page_num, time_code))
                        } else { None };
                        
                        (doc.timesheet.name.clone(), doc.timesheet.total_frames(), info, fps)
                    };

                    ui.horizontal(|ui| {
                        let doc = &mut self.documents[doc_idx];

                        if doc.edit_state.renaming_document {
                            let resp = ui.add(egui::TextEdit::singleline(&mut doc.edit_state.rename_doc_buffer).desired_width(100.0));
                            resp.request_focus();

                            if resp.lost_focus() || resp.ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if !doc.edit_state.rename_doc_buffer.trim().is_empty() {
                                    doc.timesheet.name = doc.edit_state.rename_doc_buffer.clone();
                                    doc.is_modified = true;
                                }
                                doc.edit_state.renaming_document = false;
                            }
                            if resp.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                                doc.edit_state.renaming_document = false;
                            }
                        } else {
                            let resp = ui.add(egui::Label::new(
                                egui::RichText::new(&current_doc_name).monospace().strong()
                            ).sense(egui::Sense::click()));
                            
                            if resp.double_clicked() {
                                doc.edit_state.renaming_document = true;
                                doc.edit_state.rename_doc_buffer = current_doc_name.clone();
                            }
                            resp.on_hover_text("Double-click to rename");
                        }

                        ui.separator();
                        ui.monospace(format!("{} : {}", text.label_total, total_frames));
                        
                        if let Some((l_name, _f_raw, f_abs, pg, time_code)) = selection_info {
                            ui.separator();
                            ui.monospace(format!("{} : {}", text.info_layer, l_name));
                            
                            ui.separator();
                            ui.monospace(format!("{} : {:03}", text.info_frame, f_abs));
                            
                            ui.separator();
                            ui.monospace(format!("{} : {}", text.info_page, pg));

                            ui.separator();
                            ui.label(egui::RichText::new(format!("Time : {}", time_code))
                                .monospace()
                                .color(self.current_theme.text_timecode));
                        }
                    });
                        
                    ui.separator();
                    
                    egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
                        self.render_document_content(ctx, ui, doc_idx);
                    });
                }); 

            if let Some(path) = pending_import_video {
                let path_str = path.to_str().unwrap();
                let parent_dir = path.parent().unwrap();
                let current_fps = self.documents[doc_idx].timesheet.framerate; 
                
                match video_utils::extract_frames(path_str, parent_dir, current_fps) {
                    Ok(seq_dir) => {
                        self.documents[doc_idx].reference_image_dir = Some(seq_dir);
                        self.set_success_message("Video converted to sequence and loaded.".to_string());
                    }
                    Err(e) => {
                        self.set_error_message(format!("Import failed: {}", e));
                    }
                }
            }

            // 👇 1. 先准备一个变量接住烘焙消息
            let mut bake_msg = None;

            // 👇 2. 用大括号把 doc 框起来，用完立刻归还所有权！
            {
                let doc = &mut self.documents[doc_idx];
                
                if doc.show_player {
                    bake_msg = self.player.show(ctx, doc, text);
                }

                if self.active_doc_id == Some(doc_id_val) {
                     self.curve_editor.show(ctx, doc, &mut self.show_curve_window, text, &self.current_theme);
                }
            } // 到达这里，doc 已经被释放，self 彻底自由

            //  调用 self 的方法弹窗了
            if let Some(res) = bake_msg {
                match res {
                    Ok(msg) => self.set_success_message(msg),
                    Err(msg) => self.set_error_message(msg),
                }
            }

            // 判断窗口关闭的代码保持不变，如果需要用 documents 重新借用即可
            if !window_open {
                if self.documents[doc_idx].is_modified { self.closing_doc_id = Some(self.documents[doc_idx].id); }
                else { docs_to_close.push(doc_idx); }
            }
        } // 这个大括号是原本 for 循环的结尾
 
        if let Some(clearing_id) = self.clearing_doc_id {
             egui::Area::new(egui::Id::new("clear_modal_dimmer")).fixed_pos(egui::pos2(0.0, 0.0)).order(egui::Order::Foreground).show(ctx, |ui| {
                ui.painter().rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 200));
             });
             egui::Window::new(text.dialog_clear_title).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0,0.0]).order(egui::Order::Tooltip).show(ctx, |ui| {
                 ui.label(text.dialog_clear_body);
                 ui.add_space(10.0);
                 ui.horizontal(|ui| {
                     if ui.button(text.btn_clear_confirm).clicked() {
                         if let Some(doc) = self.documents.iter_mut().find(|d| d.id == clearing_id) { doc.clear_all_cells(); }
                         self.clearing_doc_id = None;
                         self.set_success_message(text.msg_cleared.to_string());
                     }
                     if ui.button(text.btn_cancel).clicked() { self.clearing_doc_id = None; }
                 });
             });
             return;
        }

        if let Some(closing_id) = self.closing_doc_id {
             egui::Area::new(egui::Id::new("close_doc_dimmer")).fixed_pos(egui::pos2(0.0, 0.0)).order(egui::Order::Foreground).show(ctx, |ui| {
                ui.painter().rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 200));
             });
             egui::Window::new(text.dialog_unsaved_title).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0,0.0]).order(egui::Order::Tooltip).show(ctx, |ui| {
                 ui.label(text.dialog_unsaved_body);
                 ui.add_space(12.0);
                 ui.horizontal(|ui| {
                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                         if ui.button(text.btn_cancel).clicked() { self.closing_doc_id = None; }
                         ui.add_space(4.0);
                         if ui.button(text.btn_dont_save).clicked() { 
                            self.closing_doc_id = None;
                            if let Some(idx) = self.documents.iter().position(|d| d.id == closing_id) { self.documents[idx].is_open = false; }
                         }
                         ui.add_space(4.0);
                         if ui.button(text.btn_save).clicked() { 
                            self.save_document(closing_id); 
                            self.closing_doc_id = None; 
                            if let Some(idx) = self.documents.iter().position(|d| d.id == closing_id) { self.documents[idx].is_open = false; }
                         }
                     });
                 });
             });
             return;
        }

        for idx in docs_to_close { self.documents[idx].is_open = false; }
        for doc_id in docs_to_save { self.save_document(doc_id); }
        for doc_id in docs_to_save_as { self.save_document_as(doc_id); }
        self.documents.retain(|d| d.is_open);
    }
}

impl StsApp {
    fn on_close_event(&mut self) -> bool {
        let has_unsaved = self.documents.iter().any(|d| d.is_modified && d.is_open);
        if has_unsaved && !self.allowed_to_close { self.show_exit_dialog = true; false } else { true }
    }

    fn render_document_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, doc_idx: usize) {
        let auto_save_enabled = self.settings.auto_save_enabled;
        let colors = CellColors::from_config(&self.current_theme); 
        let text = Self::static_text(self.language);
        let doc = &mut self.documents[doc_idx];

        let row_height = 16.0;
        let col_width = 36.0;
        let page_col_width = 36.0;
        let layer_count = doc.timesheet.layer_count;
        let frames_per_page = doc.timesheet.frames_per_page as usize;

        // 👇 新增：用于接收轨道类型切换操作的临时变量
        let mut pending_insert: Option<usize> = None;
        let mut pending_delete: Option<usize> = None;
        let mut pending_type_change: Option<(usize, LayerType)> = None;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let (_corner_id, corner_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));
            ui.painter().rect_stroke(corner_rect, 0.0, egui::Stroke::new(0.0, colors.border_normal));

            for i in 0..layer_count {
                let (id, rect) = ui.allocate_space(egui::vec2(col_width, row_height));
                let is_editing = doc.edit_state.editing_layer_name == Some(i);
                
                let is_active_col = doc.selection_state.selected_cell.map(|(l,_)| l == i).unwrap_or(false);
                
                // 获取当前轨道的身份类型
                let layer_type = doc.layer_types.get(&i).copied().unwrap_or(LayerType::Cel);

                let bg_color = if is_editing { colors.header_bg_editing } 
                               else if is_active_col { colors.header_bg_active } 
                               else { colors.header_bg };
                               
                ui.painter().rect_filled(rect, 0.0, bg_color);
                ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, colors.border_normal));
                
                if is_active_col {
                    ui.painter().line_segment(
                        [rect.left_bottom(), rect.right_bottom()],
                        egui::Stroke::new(2.0, colors.border_selection)
                    );
                }

                if is_editing {
                    let resp = ui.put(rect, egui::TextEdit::singleline(&mut doc.edit_state.editing_layer_text).desired_width(col_width).horizontal_align(egui::Align::Center).frame(false));
                    resp.request_focus();
                    let clicked_elsewhere = ui.input(|i| i.pointer.primary_clicked()) && !resp.hovered();
                    if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) || clicked_elsewhere {
                        doc.timesheet.layer_names[i] = doc.edit_state.editing_layer_text.clone();
                        doc.is_modified = true; doc.edit_state.editing_layer_name = None;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) { doc.edit_state.editing_layer_name = None; }
                } else {
                    let resp = ui.interact(rect, id, egui::Sense::click());
                    
                    if resp.hovered() && !is_active_col && !is_editing {
                        ui.painter().rect_filled(rect, 0.0, colors.header_bg_hover);
                    }

                    // 👇 视觉增强：给不同身份的轨道加上专属图标 👇
                    let layer_name = &doc.timesheet.layer_names[i];
                    let display_name = match layer_type {
                        LayerType::Cel => layer_name.clone(),
                        LayerType::Pan => format!("▼ {}", layer_name),
                        LayerType::Opacity => format!("🌓 {}", layer_name),
                    };

                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, display_name, egui::FontId::proportional(11.0), colors.header_text);

                    if resp.double_clicked() {
                        doc.edit_state.editing_layer_name = Some(i);
                        doc.edit_state.editing_layer_text = layer_name.clone();
                    } else if resp.clicked() {
                        let total_frames = doc.timesheet.total_frames();
                        if total_frames > 0 {
                            if ui.input(|i| i.modifiers.shift) {
                                let start = doc.selection_state.selection_start.map(|(l, _)| l).unwrap_or(i);
                                let start_row = doc.selection_state.selection_start.map(|(_, f)| f).unwrap_or(0);
                                doc.selection_state.selection_start = Some((start, start_row));
                                doc.selection_state.selection_end = Some((i, total_frames - 1));
                            } else {
                                doc.selection_state.selection_start = Some((i, 0));
                                doc.selection_state.selection_end = Some((i, total_frames - 1));
                                doc.selection_state.selected_cell = Some((i, 0));
                            }
                            doc.selection_state.auto_scroll_to_selection = true;
                        }
                    }

                    // 👇 右键菜单：增加切换轨道类型的选项 👇
                    resp.context_menu(|ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        
                        let mut current_type = layer_type;
                        ui.menu_button("Layer Type", |ui| {
                            if ui.radio_value(&mut current_type, LayerType::Cel, "Cel").clicked() { pending_type_change = Some((i, LayerType::Cel)); ui.close_menu(); }
                            if ui.radio_value(&mut current_type, LayerType::Pan, "▼ Pan").clicked() { pending_type_change = Some((i, LayerType::Pan)); ui.close_menu(); }
                            if ui.radio_value(&mut current_type, LayerType::Opacity, "🌓 Opacity").clicked() { pending_type_change = Some((i, LayerType::Opacity)); ui.close_menu(); }
                        });
                        ui.separator();

                        if ui.button(text.ctx_insert_col_l).clicked() { pending_insert = Some(i); ui.close_menu(); }
                        if ui.button(text.ctx_insert_col_r).clicked() { pending_insert = Some(i + 1); ui.close_menu(); }
                        ui.separator();
                        let can_delete = doc.timesheet.layer_count > 1;
                        if ui.add_enabled(can_delete, egui::Button::new(text.ctx_del_col)).clicked() { pending_delete = Some(i); ui.close_menu(); }
                    });
                }
            }
        });

        let doc = &mut self.documents[doc_idx];
        
        //  立即执行刚刚收集到的状态切换指令 
        if let Some((idx, l_type)) = pending_type_change {
            doc.layer_types.insert(idx, l_type);
            doc.is_modified = true;
            if auto_save_enabled { doc.auto_save(); }
        }

        if let Some(index) = pending_insert { doc.insert_layer(index); if auto_save_enabled { doc.auto_save(); } return; }
        if let Some(index) = pending_delete { doc.delete_layer(index); if auto_save_enabled { doc.auto_save(); } return; }

        // 修复 ：算出表格的精确物理宽度
        let table_width = page_col_width + (layer_count as f32 * col_width);
        ui.add_sized([table_width, 4.0], egui::Separator::default().horizontal());

        let colors = CellColors::from_config(&self.current_theme);
        let total_frames = { let total = doc.timesheet.total_frames().max(1); doc.timesheet.ensure_frames(total); total };
        ui.spacing_mut().item_spacing.y = 0.0;
        let (pointer_pos, pointer_down) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
        let doc_id = self.documents[doc_idx].id;
        let can_start_drag = self.dragging_doc_id.is_none() || self.dragging_doc_id == Some(doc_id);
        
        let mut any_started_drag = false;
        let mut any_interacted = false;

        let scroll_output = egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_height, total_frames, |ui, row_range| {
            let doc = &mut self.documents[doc_idx];
            for frame_idx in row_range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let frame_str = (frame_idx + 1).to_string();

                    let (_page_id, page_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));
                    let page_response = ui.interact(page_rect, _page_id, egui::Sense::click());
                    
                    let stroke = egui::Stroke::new(1.0, colors.border_normal);
                    let mut text_color = colors.frame_col_text;

                    let is_active_row = doc.selection_state.selected_cell.map(|(_,f)| f == frame_idx).unwrap_or(false);
                    if is_active_row {
                        ui.painter().rect_filled(page_rect, 0.0, colors.header_bg_active);
                        ui.painter().line_segment(
                            [page_rect.left_bottom(), page_rect.right_bottom()],
                            egui::Stroke::new(1.5, colors.border_selection)
                        );
                    }

                    if frames_per_page > 0 && frame_idx % frames_per_page == 0 {
                        ui.painter().line_segment(
                            [page_rect.left_top(), page_rect.right_top() + egui::vec2(layer_count as f32 * col_width, 0.0)],
                            egui::Stroke::new(2.0, colors.border_selection)
                        );
                        text_color = colors.border_selection; 
                    } else {
                        ui.painter().rect_stroke(page_rect, 0.0, stroke);
                    }

                    if page_response.hovered() && !is_active_row {
                        ui.painter().rect_filled(page_rect, 0.0, colors.header_bg_hover);
                    }

                    if page_response.clicked() {
                        if layer_count > 0 {
                            if ui.input(|i| i.modifiers.shift) {
                                let start_row = doc.selection_state.selection_start.map(|(_, f)| f).unwrap_or(frame_idx);
                                let start_layer = doc.selection_state.selection_start.map(|(l, _)| l).unwrap_or(0);
                                doc.selection_state.selection_start = Some((start_layer, start_row));
                                doc.selection_state.selection_end = Some((layer_count - 1, frame_idx));
                            } else {
                                doc.selection_state.selection_start = Some((0, frame_idx));
                                doc.selection_state.selection_end = Some((layer_count - 1, frame_idx));
                                doc.selection_state.selected_cell = Some((0, frame_idx));
                            }
                        }
                        any_interacted = true;
                    }

                    ui.painter().text(page_rect.center(), egui::Align2::CENTER_CENTER, frame_str, egui::FontId::monospace(11.0), text_color);

                    for layer_idx in 0..layer_count {
                        let (started, resp) = render_cell(ui, doc, layer_idx, frame_idx, col_width, row_height, pointer_pos, pointer_down, &colors, can_start_drag);
                        if started { any_started_drag = true; }
                        if resp.clicked() || resp.dragged() || resp.has_focus() { any_interacted = true; }
                    }
                });
            }
        });

        let pointer_clicked = ui.input(|i| i.pointer.primary_clicked());
        let pointer_pos = ui.input(|i| i.pointer.interact_pos());
        
        if pointer_clicked && !any_started_drag && !any_interacted && !ctx.wants_pointer_input() {
            if let Some(pos) = pointer_pos {
                if scroll_output.inner_rect.contains(pos) {
                    let doc = &mut self.documents[doc_idx];
                    if doc.edit_state.editing_cell.is_some() {
                        doc.finish_edit(false, true);
                    }
                    doc.selection_state.selected_cell = None;
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                }
            }
        }

        if any_started_drag { self.dragging_doc_id = Some(doc_id); }

        let doc = &mut self.documents[doc_idx];
        let was_fill_dragging = doc.selection_state.is_fill_dragging;
        ctx.input(|i| {
            if !i.pointer.primary_down() {
                if doc.selection_state.is_fill_dragging { doc.apply_smart_fill(); }
                doc.selection_state.is_dragging = false;
            }
        });
        if was_fill_dragging && !doc.selection_state.is_fill_dragging { self.dragging_doc_id = None; }
        if !doc.selection_state.is_dragging && !doc.selection_state.is_fill_dragging { self.dragging_doc_id = None; }

        if let Some(_menu_pos) = doc.context_menu.pos {
            let has_selection = doc.context_menu.selection.is_some();
            let is_single_column = if let Some(((start_layer, _), (end_layer, _))) = doc.context_menu.selection { start_layer == end_layer } else { false };
            
            let can_smart_fill = if let Some(((start_l, _start_f), (end_l, _end_f))) = doc.context_menu.selection {
                start_l == end_l 
            } else { false };

            let menu_result = egui::Area::new(egui::Id::new(format!("context_menu_{}", doc.id))).order(egui::Order::Foreground).fixed_pos(doc.context_menu.screen_pos).show(ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    
                    // 👇 魔法 1 升级：必须同时清空 bg_fill 和 weak_bg_fill！
                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT; // <--- 关键杀手锏就是这句！
                    ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    
                    // 👇 魔法 2：悬停和点击时只要无边框，保留系统默认的高亮灰底色
                    ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;
                    
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                        ui.set_min_width(180.0); 

                        let copy = ui.add(egui::Button::new(text.ctx_copy).shortcut_text("Ctrl+C")).clicked();
                        let cut = ui.add(egui::Button::new(text.ctx_cut).shortcut_text("Ctrl+X")).clicked();
                        let paste = ui.add(egui::Button::new(text.ctx_paste).shortcut_text("Ctrl+V")).clicked();
                        ui.separator();
                        let undo = ui.add(egui::Button::new(text.ctx_undo).shortcut_text("Ctrl+Z")).clicked();
                        ui.separator();
                        
                        let repeat = ui.add_enabled(has_selection && is_single_column, egui::Button::new(text.ctx_repeat)).clicked();
                        let reverse = ui.add_enabled(has_selection && is_single_column, egui::Button::new(text.ctx_reverse)).clicked();
                        let smart_fill = ui.add_enabled(can_smart_fill, egui::Button::new(text.ctx_smart_fill)).clicked();
                        let sequence_fill = ui.add(egui::Button::new(text.ctx_sequence_fill)).clicked();
                        ui.separator();
                        // 这里也确保快捷键改成了 Ctrl+Alt+C
                        let copy_ae = ui.add(egui::Button::new(text.ctx_copy_ae).shortcut_text("Ctrl+Alt+C")).clicked();

                        (copy, cut, paste, undo, repeat, reverse, smart_fill, sequence_fill, copy_ae)
                    }).inner 
                }).inner     
            });             

            let (copy, cut, paste, undo, repeat, reverse, smart_fill, sequence_fill, copy_ae) = menu_result.inner;
            let menu_resp = menu_result.response;
            let doc = &mut self.documents[doc_idx];

            if copy { doc.copy_selection(ctx); doc.context_menu.pos = None; }
            else if cut { doc.cut_selection(ctx); if auto_save_enabled { doc.auto_save(); } doc.context_menu.pos = None; }
            else if paste { doc.paste_clipboard(); if auto_save_enabled { doc.auto_save(); } doc.context_menu.pos = None; }
            else if undo { doc.undo(); if auto_save_enabled { doc.auto_save(); } doc.context_menu.pos = None; }
            else if repeat {
                if let Some(((s_l, s_f), (e_l, e_f))) = doc.context_menu.selection {
                    doc.repeat_dialog.layer = s_l.min(e_l); doc.repeat_dialog.start_frame = s_f.min(e_f); doc.repeat_dialog.end_frame = s_f.max(e_f);
                    doc.repeat_dialog.repeat_count_str = "1".to_string(); 
                    doc.repeat_dialog.open = true;
                }
                doc.context_menu.pos = None;
            }
            else if reverse { if let Err(e) = doc.reverse_selection() { self.error_message = Some(e.to_string()); } else if auto_save_enabled { doc.auto_save(); } doc.context_menu.pos = None; }
            else if smart_fill {
                if let Err(e) = doc.smart_fill_auto() { self.error_message = Some(e.to_string()); }
                else if auto_save_enabled { doc.auto_save(); }
                doc.context_menu.pos = None;
            }
            else if sequence_fill {
                 if let Some((l, f)) = doc.context_menu.pos { doc.sequence_fill_dialog.layer = l; doc.sequence_fill_dialog.start_frame = f; doc.sequence_fill_dialog.open = true; }
                 doc.context_menu.pos = None;
            }
            else if copy_ae {
                if let Some((l, _)) = doc.context_menu.pos { let v = self.settings.ae_keyframe_version.as_str(); let _ = doc.copy_ae_keyframes(ctx, l, v); }
                doc.context_menu.pos = None;
            }

            if !copy && !cut && !paste && !undo && !repeat && !reverse && !smart_fill && !sequence_fill && !copy_ae {
                if ctx.input(|i| i.pointer.primary_clicked()) && !menu_resp.hovered() { doc.context_menu.pos = None; }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { doc.context_menu.pos = None; }
        }

        let doc = &mut self.documents[doc_idx];
        if doc.repeat_dialog.open {
            let mut should_execute = false;
            let mut should_cancel = false;
            let mut is_open = doc.repeat_dialog.open;
            let mut enter_consumed = false;

            egui::Window::new(text.ctx_repeat).collapsible(false).resizable(false).open(&mut is_open).show(ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.horizontal(|ui| { ui.label(text.dialog_repeat_count); }); 
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!doc.repeat_dialog.repeat_until_end, |ui| {
                        if Self::ui_drag_edit(ui, "", &mut doc.repeat_dialog.repeat_count_str, 0.1) { enter_consumed = true; }
                    });
                });
                ui.checkbox(&mut doc.repeat_dialog.repeat_until_end, text.dialog_repeat_until_end);
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button(text.btn_ok).clicked() || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && !enter_consumed) { should_execute = true; }
                    if ui.button(text.btn_cancel).clicked() { should_cancel = true; }
                });
            });
            doc.repeat_dialog.open = is_open;
            if should_cancel { doc.repeat_dialog.open = false; }
            if should_execute {
                // [修改] 只有普通数字可以使用重复运算，防止 A 这种字母导致解析崩溃
                let count = doc.parse_math_input(LayerType::Cel, &doc.repeat_dialog.repeat_count_str, 1).unwrap_or(1);
                doc.repeat_dialog.open = false;
                doc.selection_state.selection_start = Some((doc.repeat_dialog.layer, doc.repeat_dialog.start_frame));
                doc.selection_state.selection_end = Some((doc.repeat_dialog.layer, doc.repeat_dialog.end_frame));
                if let Err(e) = doc.repeat_selection(count, doc.repeat_dialog.repeat_until_end) { self.error_message = Some(e.to_string()); }
                else if auto_save_enabled { doc.auto_save(); }
            }
        }

        let doc = &mut self.documents[doc_idx];
        if doc.sequence_fill_dialog.open {
            let mut should_execute = false;
            let mut should_cancel = false;
            let base_val = doc.timesheet.get_cell(doc.sequence_fill_dialog.layer, doc.sequence_fill_dialog.start_frame).and_then(|c| match c { CellValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0);
            let mut is_open = doc.sequence_fill_dialog.open;
            let mut enter_consumed = false;

            egui::Window::new(text.ctx_sequence_fill).collapsible(false).resizable(false).open(&mut is_open).show(ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                if Self::ui_drag_edit(ui, text.dialog_seq_start, &mut doc.sequence_fill_dialog.start_value_str, 0.5) { enter_consumed = true; }
                if Self::ui_drag_edit(ui, text.dialog_seq_end, &mut doc.sequence_fill_dialog.end_value_str, 0.5) { enter_consumed = true; }
                if Self::ui_drag_edit(ui, text.dialog_seq_hold, &mut doc.sequence_fill_dialog.hold_frames_str, 0.1) { enter_consumed = true; }
                
                // 👇 [修改] Sequence Fill 工具完美适配 A-Z 模式 👇
                let l_type = doc.layer_types.get(&doc.sequence_fill_dialog.layer).copied().unwrap_or(LayerType::Cel);
                let s_v = doc.parse_math_input(l_type, &doc.sequence_fill_dialog.start_value_str, base_val).unwrap_or(1);
                let e_v = doc.parse_math_input(l_type, &doc.sequence_fill_dialog.end_value_str, s_v as i32).unwrap_or(s_v);
                let h_v = doc.parse_math_input(LayerType::Cel, &doc.sequence_fill_dialog.hold_frames_str, 1).unwrap_or(1);
                
                let val_count = if e_v >= s_v { e_v - s_v + 1 } else { s_v - e_v + 1 };
                ui.label(format!("{} : {} frames", text.label_total, val_count * h_v));
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button(text.btn_ok).clicked() || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && !enter_consumed) { should_execute = true; }
                    if ui.button(text.btn_cancel).clicked() { should_cancel = true; }
                });
            });
            doc.sequence_fill_dialog.open = is_open;
            if should_cancel { doc.sequence_fill_dialog.open = false; }
            if should_execute {
                let l_type = doc.layer_types.get(&doc.sequence_fill_dialog.layer).copied().unwrap_or(LayerType::Cel);
                let s_v = doc.parse_math_input(l_type, &doc.sequence_fill_dialog.start_value_str, base_val).unwrap_or(1);
                let e_v = doc.parse_math_input(l_type, &doc.sequence_fill_dialog.end_value_str, s_v as i32).unwrap_or(s_v);
                let h_v = doc.parse_math_input(LayerType::Cel, &doc.sequence_fill_dialog.hold_frames_str, 1).unwrap_or(1);
                
                doc.sequence_fill_dialog.open = false;
                if let Err(e) = doc.sequence_fill(doc.sequence_fill_dialog.layer, doc.sequence_fill_dialog.start_frame, s_v, e_v, h_v) { self.error_message = Some(e.to_string()); }
                else if auto_save_enabled { doc.auto_save(); }
            }
        }

        let doc = &self.documents[doc_idx];
        if ui.ui_contains_pointer() || doc.edit_state.editing_cell.is_some() { self.active_doc_id = Some(doc.id); }
        if self.active_doc_id == Some(doc.id) { self.handle_document_shortcuts(ctx, doc_idx, layer_count); }
    }

    fn handle_document_shortcuts(&mut self, ctx: &egui::Context, doc_idx: usize, layer_count: usize) {
        let auto_save_enabled = self.settings.auto_save_enabled;
        let doc = &mut self.documents[doc_idx];

        if doc.repeat_dialog.open || doc.sequence_fill_dialog.open || doc.edit_state.renaming_document { return; }

        let doc_id = doc.id;
        let mut should_copy = false;
        let mut should_cut = false;
        let mut should_paste = false;
        let mut paste_text: Option<String> = None;
        let mut should_undo = false;
        let mut should_delete = false;
        let mut should_save = false;
        let mut should_copy_ae = false;

        let is_editing = doc.edit_state.editing_cell.is_some() || doc.edit_state.editing_layer_name.is_some();
        let mut jump_step_delta: i32 = 0;

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Copy => should_copy = true,
                    egui::Event::Cut => should_cut = true,
                    egui::Event::Paste(text) => { should_paste = true; paste_text = Some(text.clone()); }
                    egui::Event::Text(text) if !is_editing => {
                        if text == "/" { jump_step_delta = -1; } else if text == "*" { jump_step_delta = 1; }
                    }
                    _ => {}
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift { should_undo = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) { should_save = true; }
            
            if i.modifiers.ctrl && i.modifiers.alt && i.key_pressed(egui::Key::C) { should_copy_ae = true; }
            
            else if i.modifiers.ctrl && i.key_pressed(egui::Key::C) && !i.modifiers.alt && !i.modifiers.shift { should_copy = true; }
            if i.key_pressed(egui::Key::Delete) { should_delete = true; }
        });

        if should_save { self.save_document(doc_id); return; }
        if jump_step_delta != 0 {
            let new_step = (doc.jump_step as i32 + jump_step_delta).max(1) as usize;
            doc.jump_step = new_step;
        }
        if should_undo { doc.undo(); if auto_save_enabled { doc.auto_save(); } }
        if !is_editing && should_delete { doc.delete_selection(); if auto_save_enabled { doc.auto_save(); } }
        if !is_editing && (should_copy || should_cut || should_paste) {
            if should_copy { doc.copy_selection(ctx); } 
            else if should_cut { doc.cut_selection(ctx); if auto_save_enabled { doc.auto_save(); } }  // <--- 就改了这里
            else if should_paste {
                let pasted = if let Some(ref text) = paste_text { doc.paste_from_text(text) } else { false };
                if !pasted { doc.paste_clipboard(); }
                if auto_save_enabled { doc.auto_save(); }
            }
        }

        if let Some((layer, frame)) = doc.selection_state.selected_cell {
            if !is_editing {
                 let total_frames = doc.timesheet.total_frames();
                 ctx.input(|i| {
                    if i.key_pressed(egui::Key::ArrowUp) && frame > 0 { doc.selection_state.selected_cell = Some((layer, frame - 1)); doc.selection_state.auto_scroll_to_selection = true; }
                    else if i.key_pressed(egui::Key::ArrowDown) && frame + 1 < total_frames { doc.selection_state.selected_cell = Some((layer, frame + 1)); doc.selection_state.auto_scroll_to_selection = true; }
                    else if i.key_pressed(egui::Key::ArrowLeft) && layer > 0 { doc.selection_state.selected_cell = Some((layer - 1, frame)); doc.selection_state.auto_scroll_to_selection = true; }
                    else if i.key_pressed(egui::Key::ArrowRight) && layer < layer_count - 1 { doc.selection_state.selected_cell = Some((layer + 1, frame)); doc.selection_state.auto_scroll_to_selection = true; }
                    else if i.key_pressed(egui::Key::Enter) {
                        if doc.get_selection_range().is_some() { doc.start_batch_edit(layer, frame); } 
                        else { doc.start_edit(layer, frame); }
                    }
                    else {
                        for event in &i.events {
                            if let egui::Event::Text(text) = event {
                                if text.chars().all(|c| c.is_ascii_graphic()) {
                                    if doc.get_selection_range().is_some() { doc.start_batch_edit(layer, frame); } 
                                    else { doc.start_edit(layer, frame); }
                                    doc.edit_state.editing_text = text.clone();
                                    break;
                                }
                            }
                        }
                    }
                 });
            }
        }
    }
}