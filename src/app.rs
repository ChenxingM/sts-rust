//! App module - main application logic and UI

use eframe::egui;
use std::rc::Rc;
use std::sync::OnceLock;
use crate::document::Document;
use crate::ui::{render_cell, CellColors, AboutDialog};
use crate::settings::{ExportSettings, CsvEncoding, ThemeMode, AeKeyframeVersion};
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

pub struct StsApp {
    pub documents: Vec<Document>,
    pub next_doc_id: usize,
    pub active_doc_id: Option<usize>,
    pub show_new_dialog: bool,
    pub new_dialog_focus_name: bool,
    pub closing_doc_id: Option<usize>,
    pub new_name: String,
    pub new_framerate: u32,
    pub new_layer_count: usize,
    pub new_frames_per_page: u32,
    pub new_seconds: u32,
    pub new_frames: u32,
    pub error_message: Option<String>,
    // 应用程序关闭状态
    pub show_exit_dialog: bool,
    pub allowed_to_close: bool,
    // 设置
    pub settings: ExportSettings,
    pub show_settings_dialog: bool,
    pub temp_csv_header_name: String,
    pub temp_csv_encoding: usize, // 0: UTF-8, 1: GB2312, 2: Shift-JIS
    pub temp_auto_save_enabled: bool,
    pub temp_theme_mode: ThemeMode,
    pub temp_ae_keyframe_version: usize, // 0: 6.0, 1: 7.0, 2: 8.0, 3: 9.0
    // 关于对话框
    pub about_dialog: AboutDialog,
}

impl Default for StsApp {
    fn default() -> Self {
        let settings = ExportSettings::load_from_registry();
        let temp_encoding = match settings.csv_encoding {
            CsvEncoding::Utf8 => 0,
            CsvEncoding::Gb2312 => 1,
            CsvEncoding::ShiftJis => 2,
        };
        Self {
            documents: Vec::new(),
            next_doc_id: 0,
            active_doc_id: None,
            show_new_dialog: false,
            new_dialog_focus_name: false,
            closing_doc_id: None,
            new_name: "sheet1".to_string(),
            new_framerate: 24,
            new_layer_count: 12,
            new_frames_per_page: 144,
            new_seconds: 6,
            new_frames: 0,
            error_message: None,
            show_exit_dialog: false,
            allowed_to_close: false,
            temp_csv_header_name: settings.csv_header_name.clone(),
            temp_csv_encoding: temp_encoding,
            temp_auto_save_enabled: settings.auto_save_enabled,
            temp_theme_mode: settings.theme_mode,
            temp_ae_keyframe_version: settings.ae_keyframe_version.index(),
            settings,
            show_settings_dialog: false,
            about_dialog: AboutDialog::default(),
        }
    }
}

impl StsApp {
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

    /// Load a file from the given path
    fn load_file_from_path(&mut self, path_str: &str) {
        // 限制最大文档数量
        const MAX_DOCUMENTS: usize = 100;
        if self.documents.len() >= MAX_DOCUMENTS {
            self.error_message = Some(format!("Too many documents open (max: {}). Please close some documents first.", MAX_DOCUMENTS));
            return;
        }

        // 检查文件是否已打开
        if let Some(_existing) = self.documents.iter().find(|d| {
            d.file_path.as_ref().map_or(false, |p| p.as_ref() == path_str)
        }) {
            self.error_message = Some(format!("File is already open: {}", path_str));
            return;
        }

        // Determine file type by extension
        let extension = std::path::Path::new(path_str)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "sts" => {
                match sts_rust::parse_sts_file(path_str) {
                    Ok(ts) => {
                        let doc = Document::new(self.next_doc_id, ts, Some(path_str.to_string()));
                        self.next_doc_id += 1;
                        self.documents.push(doc);
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open: {}", e));
                    }
                }
            }
            "xdts" => {
                match sts_rust::parse_xdts_file(path_str) {
                    Ok(timesheets) => {
                        if timesheets.is_empty() {
                            self.error_message = Some("No timesheets found in XDTS file".to_string());
                        } else {
                            for ts in timesheets {
                                let doc = Document::new(self.next_doc_id, ts, None);
                                self.next_doc_id += 1;
                                self.documents.push(doc);
                            }
                            self.error_message = None;
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open: {}", e));
                    }
                }
            }
            "tdts" => {
                match sts_rust::parse_tdts_file(path_str) {
                    Ok(result) => {
                        if result.timesheets.is_empty() {
                            self.error_message = Some("No timesheets found in TDTS file".to_string());
                        } else {
                            for ts in result.timesheets {
                                let doc = Document::new(self.next_doc_id, ts, None);
                                self.next_doc_id += 1;
                                self.documents.push(doc);
                            }
                            if !result.warnings.is_empty() {
                                self.error_message = Some(format!("Warning: {}", result.warnings.join(", ")));
                            } else {
                                self.error_message = None;
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open: {}", e));
                    }
                }
            }
            "csv" => {
                match sts_rust::parse_csv_file(path_str) {
                    Ok(ts) => {
                        let doc = Document::new(self.next_doc_id, ts, None);
                        self.next_doc_id += 1;
                        self.documents.push(doc);
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open: {}", e));
                    }
                }
            }
            "sxf" => {
                // Use new SXF parser that handles multi-section format
                match sts_rust::parse_sxf_groups(path_str) {
                    Ok(groups) => {
                        // Convert groups to TimeSheet for display
                        let filename = std::path::Path::new(path_str)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("untitled");

                        match sts_rust::groups_to_timesheet(&groups, filename) {
                            Ok(ts) => {
                                let doc = Document::new(self.next_doc_id, ts, None);
                                self.next_doc_id += 1;
                                self.documents.push(doc);
                                self.error_message = None;
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Failed to convert SXF: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open SXF: {}", e));
                    }
                }
            }
            _ => {
                self.error_message = Some(format!("Unsupported file type: {}", extension));
            }
        }
    }

    pub fn open_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Supported", &["sts", "xdts", "tdts", "csv", "sxf"])
            .add_filter("STS Files", &["sts"])
            .add_filter("XDTS Files", &["xdts"])
            .add_filter("TDTS Files", &["tdts"])
            .add_filter("CSV Files", &["csv"])
            .add_filter("SXF Files", &["sxf"])
            .pick_file()
        {
            let path_str = path.to_str().unwrap();
            self.load_file_from_path(path_str);
        }
    }

    pub fn save_document(&mut self, doc_id: usize) {
        if let Some(doc) = self.documents.iter_mut().find(|d| d.id == doc_id) {
            if doc.file_path.is_some() {
                if let Err(e) = doc.save() {
                    self.error_message = Some(e);
                } else {
                    self.error_message = None;
                }
            } else {
                self.save_document_as(doc_id);
            }
        }
    }

    pub fn save_document_as(&mut self, doc_id: usize) {
        let default_name = self.documents.iter()
            .find(|d| d.id == doc_id)
            .map(|d| format!("{}.sts", d.timesheet.name))
            .unwrap_or_else(|| "untitled.sts".to_string());

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STS Files", &["sts"])
            .set_file_name(&default_name)
            .save_file()
        {
            let path_str = path.to_str().unwrap().to_string();
            if let Some(doc) = self.documents.iter_mut().find(|d| d.id == doc_id) {
                if let Err(e) = doc.save_as(path_str) {
                    self.error_message = Some(e);
                } else {
                    self.error_message = None;
                }
            }
        }
    }

    pub fn export_to_csv(&mut self, doc_id: usize) {
        let default_name = self.documents.iter()
            .find(|d| d.id == doc_id)
            .map(|d| format!("{}.csv", d.timesheet.name))
            .unwrap_or_else(|| "export.csv".to_string());

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .set_file_name(&default_name)
            .save_file()
        {
            let path_str = path.to_str().unwrap();
            if let Some(doc) = self.documents.iter().find(|d| d.id == doc_id) {
                match sts_rust::write_csv_file_with_options(
                    &doc.timesheet,
                    path_str,
                    &self.settings.csv_header_name,
                    self.settings.csv_encoding,
                ) {
                    Ok(_) => {
                        self.error_message = Some(format!("Exported to CSV: {}", path_str));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to export CSV: {}", e));
                    }
                }
            }
        }
    }

    /// Auto-save document if auto-save is enabled and document has a file path
    fn auto_save_document(&mut self, doc_idx: usize) {
        if self.settings.auto_save_enabled {
            if let Some(doc) = self.documents.get_mut(doc_idx) {
                doc.auto_save();
            }
        }
    }

    fn apply_theme(ctx: &egui::Context, theme_mode: ThemeMode) {
        let visuals = match theme_mode {
            ThemeMode::Light => egui::Visuals::light(),
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::System => {
                // Try to detect system theme, fallback to light
                if ctx.style().visuals.dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                }
            }
        };
        ctx.set_visuals(visuals);
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 只在首次设置视觉样式
        static STYLE_INIT: OnceLock<()> = OnceLock::new();
        let theme_mode = self.settings.theme_mode;
        STYLE_INIT.get_or_init(|| {
            Self::apply_theme(ctx, theme_mode);

            let mut style = (*ctx.style()).clone();
            style.spacing.window_margin = egui::Margin::same(4.0);
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::proportional(14.0),
            );
            ctx.set_style(style);
        });

        // 检测窗口关闭请求
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.on_close_event() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // 退出确认对话框
        if self.show_exit_dialog {
            let unsaved_docs: Vec<String> = self.documents.iter()
                .filter(|d| d.is_modified && d.is_open)
                .map(|d| d.timesheet.name.clone())
                .collect();

            let unsaved_count = unsaved_docs.len();

            egui::Area::new(egui::Id::new("exit_modal_dimmer"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
                    );
                });

            let mut action: Option<i32> = None; // 0: save all, 1: discard all, 2: cancel

            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if unsaved_count == 1 {
                        ui.label(format!("\"{}\" has unsaved changes.", unsaved_docs[0]));
                    } else {
                        ui.label(format!("{} documents have unsaved changes:", unsaved_count));
                        for name in &unsaved_docs {
                            ui.label(format!("  - {}", name));
                        }
                    }
                    ui.add_space(10.0);

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.add_sized([100.0, 25.0], egui::Button::new("Save All")).clicked() || enter_pressed {
                            action = Some(0);
                        }
                        if ui.add_sized(
                            [100.0, 25.0],
                            egui::Button::new(egui::RichText::new("Discard All").color(egui::Color32::RED))
                        ).clicked() {
                            action = Some(1);
                        }
                        if ui.add_sized([80.0, 25.0], egui::Button::new("Cancel")).clicked() {
                            action = Some(2);
                        }
                    });
                });

            match action {
                Some(0) => {
                    // Save All
                    let doc_ids: Vec<usize> = self.documents.iter()
                        .filter(|d| d.is_modified && d.is_open)
                        .map(|d| d.id)
                        .collect();
                    for doc_id in doc_ids {
                        self.save_document(doc_id);
                    }
                    self.show_exit_dialog = false;
                    self.allowed_to_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(1) => {
                    // Discard All
                    self.show_exit_dialog = false;
                    self.allowed_to_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(2) => {
                    // Cancel
                    self.show_exit_dialog = false;
                }
                _ => {}
            }
        }

        // 全局快捷键
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.show_new_dialog = true;
                self.new_dialog_focus_name = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.open_document();
            }
        });

        // 拖拽文件支持
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        if let Some(path_str) = path.to_str() {
                            self.load_file_from_path(path_str);
                        }
                    }
                }
            }
        });

        // 菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New (Ctrl+N)").clicked() {
                        self.show_new_dialog = true;
                        self.new_dialog_focus_name = true;
                        ui.close_menu();
                    }

                    if ui.button("Open... (Ctrl+O)").clicked() {
                        self.open_document();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Close All").clicked() {
                        self.documents.clear();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Settings...").clicked() {
                        // 初始化临时设置值
                        self.temp_csv_header_name = self.settings.csv_header_name.clone();
                        self.temp_csv_encoding = match self.settings.csv_encoding {
                            CsvEncoding::Utf8 => 0,
                            CsvEncoding::Gb2312 => 1,
                            CsvEncoding::ShiftJis => 2,
                        };
                        self.temp_auto_save_enabled = self.settings.auto_save_enabled;
                        self.temp_theme_mode = self.settings.theme_mode;
                        self.show_settings_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About STS...").clicked() {
                        self.about_dialog.open = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // 设置对话框
        if self.show_settings_dialog {
            egui::Area::new(egui::Id::new("settings_modal_dimmer"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
                    );
                });

            let mut should_save = false;
            let mut should_cancel = false;

            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.heading("CSV Export");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Header name:");
                        ui.text_edit_singleline(&mut self.temp_csv_header_name);
                    });

                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Encoding:");
                        egui::ComboBox::from_id_salt("csv_encoding")
                            .selected_text(match self.temp_csv_encoding {
                                0 => "UTF-8",
                                1 => "GB2312",
                                2 => "Shift-JIS",
                                _ => "GB2312",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.temp_csv_encoding, 0, "UTF-8");
                                ui.selectable_value(&mut self.temp_csv_encoding, 1, "GB2312");
                                ui.selectable_value(&mut self.temp_csv_encoding, 2, "Shift-JIS");
                            });
                    });

                    ui.add_space(15.0);
                    ui.heading("General");
                    ui.add_space(5.0);

                    ui.checkbox(&mut self.temp_auto_save_enabled, "Auto-save (save after each edit)");

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        egui::ComboBox::from_id_salt("theme_mode")
                            .selected_text(match self.temp_theme_mode {
                                ThemeMode::System => "System",
                                ThemeMode::Light => "Light",
                                ThemeMode::Dark => "Dark",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.temp_theme_mode, ThemeMode::System, "System");
                                ui.selectable_value(&mut self.temp_theme_mode, ThemeMode::Light, "Light");
                                ui.selectable_value(&mut self.temp_theme_mode, ThemeMode::Dark, "Dark");
                            });
                    });

                    ui.add_space(15.0);
                    ui.heading("After Effects");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Keyframe version:");
                        egui::ComboBox::from_id_salt("ae_keyframe_version")
                            .selected_text(match self.temp_ae_keyframe_version {
                                0 => "6.0",
                                1 => "7.0",
                                2 => "8.0",
                                _ => "9.0",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.temp_ae_keyframe_version, 0, "6.0");
                                ui.selectable_value(&mut self.temp_ae_keyframe_version, 1, "7.0");
                                ui.selectable_value(&mut self.temp_ae_keyframe_version, 2, "8.0");
                                ui.selectable_value(&mut self.temp_ae_keyframe_version, 3, "9.0");
                            });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() || enter_pressed {
                            should_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });

            if should_save {
                // 更新设置
                self.settings.csv_header_name = self.temp_csv_header_name.clone();
                self.settings.csv_encoding = match self.temp_csv_encoding {
                    0 => CsvEncoding::Utf8,
                    2 => CsvEncoding::ShiftJis,
                    _ => CsvEncoding::Gb2312,
                };
                self.settings.auto_save_enabled = self.temp_auto_save_enabled;
                self.settings.theme_mode = self.temp_theme_mode;
                self.settings.ae_keyframe_version = AeKeyframeVersion::from_index(self.temp_ae_keyframe_version);

                // Apply theme
                Self::apply_theme(ctx, self.settings.theme_mode);

                // 保存到注册表
                if let Err(e) = self.settings.save_to_registry() {
                    self.error_message = Some(format!("Failed to save settings: {}", e));
                }

                self.show_settings_dialog = false;
            }

            if should_cancel {
                self.show_settings_dialog = false;
            }
        }

        // 关于对话框
        self.about_dialog.show(ctx);

        // 新建对话框
        if self.show_new_dialog {
            egui::Area::new(egui::Id::new("modal_dimmer"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    let bg_color = ui.visuals().window_fill();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(
                            bg_color.r(),
                            bg_color.g(),
                            bg_color.b(),
                            200,
                        ),
                    );
                });

            egui::Window::new("New")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let response = ui.text_edit_singleline(&mut self.new_name);
                        // 对话框刚打开时请求焦点
                        if self.new_dialog_focus_name {
                            response.request_focus();
                            self.new_dialog_focus_name = false;
                        }
                        // 每次获得焦点时全选文本
                        if response.gained_focus() {
                            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(self.new_name.chars().count()),
                                )));
                                state.store(ui.ctx(), response.id);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Layers:");
                        ui.add(egui::DragValue::new(&mut self.new_layer_count).range(1..=1000));
                    });
                    ui.horizontal(|ui| {
                        ui.label("FPS:");
                        ui.radio_value(&mut self.new_framerate, 24, "24");
                        ui.radio_value(&mut self.new_framerate, 30, "30");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Frames/Page:");
                        ui.add(egui::DragValue::new(&mut self.new_frames_per_page).range(12..=288));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.add(egui::DragValue::new(&mut self.new_seconds).range(0..=3600).suffix("s"));
                        ui.label("+");
                        ui.add(egui::DragValue::new(&mut self.new_frames).range(0..=self.new_framerate - 1).suffix("k"));
                    });

                    let total_frames = self.new_seconds * self.new_framerate + self.new_frames;
                    let total_pages = if total_frames == 0 {
                        0
                    } else {
                        (total_frames + self.new_frames_per_page - 1) / self.new_frames_per_page
                    };

                    ui.horizontal(|ui| {
                        ui.label("→ Total Frames:");
                        let mut buf1 = itoa::Buffer::new();
                        ui.label(buf1.format(total_frames));
                        ui.separator();
                        ui.label("Pages:");
                        let mut buf2 = itoa::Buffer::new();
                        ui.label(buf2.format(total_pages));
                    });

                    ui.separator();

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("OK").clicked() || enter_pressed {
                        self.create_new_document();
                    }
                });
            return;
        }

        // 错误消息
        if let Some(msg) = &self.error_message {
            egui::TopBottomPanel::bottom("error_panel").show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED, msg);
            });
        }

        // 设置背景
        egui::CentralPanel::default().show(ctx, |_ui| {});

        // 渲染所有文档窗口
        let mut docs_to_save = Vec::new();
        let mut docs_to_save_as = Vec::new();
        let mut docs_to_close = Vec::new();

        let num_docs = self.documents.len();
        for doc_idx in 0..num_docs {
            let (window_title, doc_id_val, is_open_before) = {
                let doc = &self.documents[doc_idx];
                let title = if doc.jump_step > 1 {
                    format!("{} [Step: {}]", doc.title(), doc.jump_step)
                } else {
                    doc.title()
                };
                (title, doc.id, doc.is_open)
            };

            if !is_open_before {
                continue;
            }

            let mut window_open = true;

            let _window_resp = egui::Window::new(&window_title)
                .id(egui::Id::new(format!("doc_{}", doc_id_val)))
                .open(&mut window_open)
                .resizable(true)
                .min_width(400.0)
                .min_height(300.0)
                .default_width(800.0)
                .default_height(600.0)
                .show(ctx, |ui| {
                    // 使用 ScrollArea 包裹所有内容，防止内容大小影响窗口
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // 工具栏
                            ui.horizontal(|ui| {
                                if ui.button("Save (Ctrl+S)").clicked() {
                                    docs_to_save.push(doc_id_val);
                                }
                                if ui.button("Save As...").clicked() {
                                    docs_to_save_as.push(doc_id_val);
                                }
                                ui.separator();
                                if ui.button("Export CSV...").clicked() {
                                    self.export_to_csv(doc_id_val);
                                }
                            });

                            ui.separator();

                            // 文档信息
                            let (name, total_frames, cursor_info) = {
                                let doc = &self.documents[doc_idx];
                                let cursor = if let Some((layer, frame)) = doc.selection_state.selected_cell {
                                    let layer_name = doc.timesheet.layer_names.get(layer)
                                        .map(|s| s.as_str())
                                        .unwrap_or("?");
                                    Some(format!("{} {}K", layer_name, frame + 1))
                                } else {
                                    None
                                };
                                (doc.timesheet.name.clone(), doc.timesheet.total_frames(), cursor)
                            };

                            ui.horizontal(|ui| {
                                ui.label(&name);
                                ui.separator();
                                ui.label("Total Frames:");
                                let mut frames_buf = itoa::Buffer::new();
                                ui.label(frames_buf.format(total_frames));
                                if let Some(ref cursor) = cursor_info {
                                    ui.separator();
                                    ui.label(cursor);
                                }
                            });

                            ui.separator();

                            // 渲染表格
                            self.render_document_content(ctx, ui, doc_idx);
                        });
                });

            if !window_open {
                let doc = &self.documents[doc_idx];
                if doc.is_modified {
                    self.closing_doc_id = Some(doc.id);
                } else {
                    docs_to_close.push(doc_idx);
                }
            }
        }

        // 关闭确认对话框
        if let Some(closing_id) = self.closing_doc_id {
            let mut action: Option<bool> = None;
            let mut cancel = false;

            egui::Window::new("Save Changes?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Do you want to save changes before closing?");
                    ui.add_space(10.0);

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.add_sized([80.0, 25.0], egui::Button::new("Save")).clicked() || enter_pressed {
                            action = Some(true);
                        }
                        if ui.add_sized(
                            [90.0, 25.0],
                            egui::Button::new(egui::RichText::new("Don't Save").color(egui::Color32::RED))
                        ).clicked() {
                            action = Some(false);
                        }
                        if ui.add_sized([80.0, 25.0], egui::Button::new("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });

            if let Some(should_save) = action {
                if should_save {
                    self.save_document(closing_id);
                }
                // 关闭文档
                if let Some(idx) = self.documents.iter().position(|d| d.id == closing_id) {
                    self.documents[idx].is_open = false;
                }
                self.closing_doc_id = None;
            } else if cancel {
                self.closing_doc_id = None;
            }
            return;
        }

        // 关闭文档
        for idx in docs_to_close {
            self.documents[idx].is_open = false;
        }

        // 处理保存请求
        for doc_id in docs_to_save {
            self.save_document(doc_id);
        }
        for doc_id in docs_to_save_as {
            self.save_document_as(doc_id);
        }

        // 移除已关闭的文档
        self.documents.retain(|d| d.is_open);
    }
}

impl StsApp {
    fn on_close_event(&mut self) -> bool {
        // 检查是否有未保存的文档
        let has_unsaved = self.documents.iter().any(|d| d.is_modified && d.is_open);

        if has_unsaved && !self.allowed_to_close {
            self.show_exit_dialog = true;
            false // 阻止关闭
        } else {
            true // 允许关闭
        }
    }

    fn render_document_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, doc_idx: usize) {
        let auto_save_enabled = self.settings.auto_save_enabled;
        let colors = CellColors::from_visuals(ui.visuals());
        let doc = &mut self.documents[doc_idx];

        let row_height = 16.0;
        let col_width = 36.0;
        let page_col_width = 36.0;
        let layer_count = doc.timesheet.layer_count;

        // 用于延迟执行的列操作
        let mut pending_insert: Option<usize> = None;
        let mut pending_delete: Option<usize> = None;

        // 表头
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let (_corner_id, corner_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));
            ui.painter().rect_stroke(
                corner_rect,
                0.0,
                egui::Stroke::new(0.0, colors.border_normal),
            );

            for i in 0..layer_count {
                let (id, rect) = ui.allocate_space(egui::vec2(col_width, row_height));
                let is_editing = doc.edit_state.editing_layer_name == Some(i);

                let bg_color = if is_editing {
                    colors.header_bg_editing
                } else {
                    colors.header_bg
                };
                ui.painter().rect_filled(rect, 0.0, bg_color);
                ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, colors.border_normal));

                if is_editing {
                    let resp = ui.put(
                        rect,
                        egui::TextEdit::singleline(&mut doc.edit_state.editing_layer_text)
                            .desired_width(col_width)
                            .horizontal_align(egui::Align::Center)
                            .frame(false),
                    );
                    resp.request_focus();

                    if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        doc.timesheet.layer_names[i] = doc.edit_state.editing_layer_text.clone();
                        doc.is_modified = true;
                        doc.edit_state.editing_layer_name = None;
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        doc.edit_state.editing_layer_name = None;
                    }
                } else {
                    let resp = ui.interact(rect, id, egui::Sense::click());
                    let layer_name = &doc.timesheet.layer_names[i];
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        layer_name,
                        egui::FontId::proportional(11.0),
                        colors.header_text,
                    );

                    if resp.clicked() {
                        doc.edit_state.editing_layer_name = Some(i);
                        doc.edit_state.editing_layer_text = layer_name.clone();
                    }

                    // 列标题右键菜单
                    resp.context_menu(|ui| {
                        if ui.button("Insert Column Left").clicked() {
                            pending_insert = Some(i);
                            ui.close_menu();
                        }
                        if ui.button("Insert Column Right").clicked() {
                            pending_insert = Some(i + 1);
                            ui.close_menu();
                        }
                        ui.separator();
                        let can_delete = doc.timesheet.layer_count > 1;
                        if ui.add_enabled(can_delete, egui::Button::new("Delete Column")).clicked() {
                            pending_delete = Some(i);
                            ui.close_menu();
                        }
                    });
                }
            }
        });

        // 执行延迟的列操作（在渲染循环外执行）
        let doc = &mut self.documents[doc_idx];
        if let Some(index) = pending_insert {
            doc.insert_layer(index);
            if auto_save_enabled {
                doc.auto_save();
            }
            // 列操作后立即返回，让下一帧重新渲染
            return;
        }
        if let Some(index) = pending_delete {
            doc.delete_layer(index);
            if auto_save_enabled {
                doc.auto_save();
            }
            // 列操作后立即返回，让下一帧重新渲染
            return;
        }

        ui.separator();

        // Store colors for use in closures
        let colors = CellColors::from_visuals(ui.visuals());

        // 数据区域
        let total_frames = {
            let total = doc.timesheet.total_frames().max(1);
            doc.timesheet.ensure_frames(total);
            total
        };

        ui.spacing_mut().item_spacing.y = 0.0;

        let (pointer_pos, pointer_down) = ui.input(|i| {
            (i.pointer.interact_pos(), i.pointer.primary_down())
        });

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_frames, |ui, row_range| {
                let doc = &mut self.documents[doc_idx];

                for frame_idx in row_range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                        let (page, frame_in_page) = doc.timesheet.get_page_and_frame(frame_idx);
                        let mut page_buf_local = itoa::Buffer::new();
                        let mut frame_buf_local = itoa::Buffer::new();
                        let page_str = page_buf_local.format(page);
                        let frame_str = frame_buf_local.format(frame_in_page);

                        let (_page_id, page_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));
                        ui.painter().rect_stroke(
                            page_rect,
                            0.0,
                            egui::Stroke::new(1.0, colors.border_normal),
                        );

                        ui.painter().text(
                            page_rect.left_center() + egui::vec2(3.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            page_str,
                            egui::FontId::monospace(11.0),
                            colors.frame_col_text,
                        );

                        if !frame_str.is_empty() {
                            ui.painter().text(
                                page_rect.right_center() - egui::vec2(3.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                frame_str,
                                egui::FontId::monospace(11.0),
                                colors.frame_col_text,
                            );
                        }

                        // 单元格渲染
                        for layer_idx in 0..layer_count {
                            render_cell(ui, doc, layer_idx, frame_idx, col_width, row_height, pointer_pos, pointer_down, &colors);
                        }
                    });
                }
            });

        // 鼠标释放
        let doc = &mut self.documents[doc_idx];
        ctx.input(|i| {
            if !i.pointer.primary_down() && doc.selection_state.is_dragging {
                doc.selection_state.is_dragging = false;
            }
        });

        // 右键菜单
        if let Some(_menu_pos) = doc.context_menu.pos {
            // 检查是否有选择范围
            let has_selection = doc.context_menu.selection.is_some();
            // 检查是否为单列选择
            let is_single_column = if let Some(((start_layer, _), (end_layer, _))) = doc.context_menu.selection {
                start_layer == end_layer
            } else {
                false
            };

            let menu_result = egui::Area::new(egui::Id::new(format!("context_menu_{}", doc.id)))
                .order(egui::Order::Foreground)
                .fixed_pos(doc.context_menu.screen_pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(120.0);

                        let copy = ui.button("Copy (Ctrl+C)").clicked();
                        let cut = ui.button("Cut (Ctrl+X)").clicked();
                        let paste = ui.button("Paste (Ctrl+V)").clicked();

                        ui.separator();

                        let undo = ui.button("Undo (Ctrl+Z)").clicked();

                        ui.separator();

                        // Repeat 和 Reverse 只在有选择时可用
                        let repeat = ui.add_enabled(has_selection && is_single_column, egui::Button::new("Repeat...")).clicked();
                        let reverse = ui.add_enabled(has_selection && is_single_column, egui::Button::new("Reverse")).clicked();
                        let sequence_fill = ui.button("Sequence Fill...").clicked();

                        ui.separator();

                        let copy_ae = ui.button("Copy AE Keyframes").clicked();

                        (copy, cut, paste, undo, repeat, reverse, sequence_fill, copy_ae)
                    }).inner
                });

            let (copy_clicked, cut_clicked, paste_clicked, undo_clicked, repeat_clicked, reverse_clicked, sequence_fill_clicked, copy_ae_clicked) = menu_result.inner;
            let menu_response = menu_result.response;

            let doc = &mut self.documents[doc_idx];

            if copy_clicked {
                if let Some((start, end)) = doc.context_menu.selection {
                    doc.selection_state.selection_start = Some(start);
                    doc.selection_state.selection_end = Some(end);
                    doc.copy_selection(ctx);
                } else if let Some((layer, frame)) = doc.context_menu.pos {
                    let cell = doc.timesheet.get_cell(layer, frame).copied();
                    doc.clipboard = Some(Rc::new(vec![vec![cell]]));
                    let text = match cell {
                        Some(CellValue::Number(n)) => n.to_string(),
                        Some(CellValue::Same) => "-".to_string(),
                        None => "".to_string(),
                    };
                    ctx.output_mut(|o| o.copied_text = text);
                }
                doc.context_menu.pos = None;
            } else if cut_clicked {
                if let Some((start, end)) = doc.context_menu.selection {
                    doc.selection_state.selection_start = Some(start);
                    doc.selection_state.selection_end = Some(end);
                    doc.cut_selection(ctx);
                    if auto_save_enabled { doc.auto_save(); }
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                } else if let Some((layer, frame)) = doc.context_menu.pos {
                    doc.selection_state.selection_start = Some((layer, frame));
                    doc.selection_state.selection_end = Some((layer, frame));
                    doc.cut_selection(ctx);
                    if auto_save_enabled { doc.auto_save(); }
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                }
                doc.context_menu.pos = None;
            } else if paste_clicked {
                if let Some((layer, frame)) = doc.context_menu.pos {
                    doc.selection_state.selected_cell = Some((layer, frame));
                }
                doc.paste_clipboard();
                if auto_save_enabled { doc.auto_save(); }
                doc.context_menu.pos = None;
            } else if undo_clicked {
                doc.undo();
                if auto_save_enabled { doc.auto_save(); }
                doc.context_menu.pos = None;
            } else if repeat_clicked {
                // 打开 Repeat 弹窗
                if let Some(((start_layer, start_frame), (end_layer, end_frame))) = doc.context_menu.selection {
                    let min_frame = start_frame.min(end_frame);
                    let max_frame = start_frame.max(end_frame);
                    doc.repeat_dialog.layer = start_layer.min(end_layer);
                    doc.repeat_dialog.start_frame = min_frame;
                    doc.repeat_dialog.end_frame = max_frame;
                    doc.repeat_dialog.repeat_count = 1;
                    doc.repeat_dialog.repeat_until_end = false;
                    doc.repeat_dialog.open = true;
                }
                doc.context_menu.pos = None;
            } else if reverse_clicked {
                // 执行 Reverse
                if let Some((start, end)) = doc.context_menu.selection {
                    doc.selection_state.selection_start = Some(start);
                    doc.selection_state.selection_end = Some(end);
                    if let Err(e) = doc.reverse_selection() {
                        self.error_message = Some(e.to_string());
                    } else if auto_save_enabled {
                        doc.auto_save();
                    }
                }
                doc.context_menu.pos = None;
            } else if sequence_fill_clicked {
                // 打开 Sequence Fill 弹窗
                if let Some((layer, frame)) = doc.context_menu.pos {
                    doc.sequence_fill_dialog.layer = layer;
                    doc.sequence_fill_dialog.start_frame = frame;
                    doc.sequence_fill_dialog.open = true;
                }
                doc.context_menu.pos = None;
            } else if copy_ae_clicked {
                // Copy AE Keyframes - use clicked cell's layer
                if let Some((layer, _frame)) = doc.context_menu.pos {
                    let ae_version = self.settings.ae_keyframe_version.as_str();
                    if let Err(e) = doc.copy_ae_keyframes(ctx, layer, ae_version) {
                        self.error_message = Some(e.to_string());
                    } else {
                        self.error_message = Some("AE Time Remap keyframes copied".to_string());
                    }
                }
                doc.context_menu.pos = None;
            }

            // 点击菜单外部关闭
            if !copy_clicked && !cut_clicked && !paste_clicked && !undo_clicked && !repeat_clicked && !reverse_clicked && !sequence_fill_clicked && !copy_ae_clicked {
                let clicked_outside = ctx.input(|i| {
                    if i.pointer.primary_clicked() {
                        if let Some(pos) = i.pointer.interact_pos() {
                            !menu_response.rect.contains(pos)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });

                if clicked_outside {
                    doc.context_menu.pos = None;
                }
            }

            // ESC键关闭菜单
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                doc.context_menu.pos = None;
            }
        }

        // Repeat 弹窗
        let doc = &mut self.documents[doc_idx];
        if doc.repeat_dialog.open {
            let mut should_execute = false;
            let mut should_cancel = false;

            egui::Window::new("Repeat")
                .collapsible(false)
                .resizable(false)
                .open(&mut doc.repeat_dialog.open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Selection:");
                        ui.label(format!("frames {} - {}", doc.repeat_dialog.start_frame + 1, doc.repeat_dialog.end_frame + 1));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let repeat_count_enabled = !doc.repeat_dialog.repeat_until_end;
                        ui.add_enabled_ui(repeat_count_enabled, |ui| {
                            ui.label("Repeat count:");
                            ui.add(egui::DragValue::new(&mut doc.repeat_dialog.repeat_count).range(1..=1000));
                        });
                    });

                    ui.checkbox(&mut doc.repeat_dialog.repeat_until_end, "Repeat until end");

                    ui.separator();

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() || enter_pressed {
                            should_execute = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });

            if should_cancel {
                doc.repeat_dialog.open = false;
            }

            if should_execute {
                // 设置选择范围
                doc.selection_state.selection_start = Some((doc.repeat_dialog.layer, doc.repeat_dialog.start_frame));
                doc.selection_state.selection_end = Some((doc.repeat_dialog.layer, doc.repeat_dialog.end_frame));

                let repeat_count = doc.repeat_dialog.repeat_count;
                let repeat_until_end = doc.repeat_dialog.repeat_until_end;

                if let Err(e) = doc.repeat_selection(repeat_count, repeat_until_end) {
                    self.error_message = Some(e.to_string());
                } else if auto_save_enabled {
                    doc.auto_save();
                }
                doc.repeat_dialog.open = false;
            }
        }

        // Sequence Fill 弹窗
        let doc = &mut self.documents[doc_idx];
        if doc.sequence_fill_dialog.open {
            let mut should_execute = false;
            let mut should_cancel = false;

            egui::Window::new("Sequence Fill")
                .collapsible(false)
                .resizable(false)
                .open(&mut doc.sequence_fill_dialog.open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Start value:");
                        ui.add(egui::DragValue::new(&mut doc.sequence_fill_dialog.start_value).range(0..=9999));
                    });

                    ui.horizontal(|ui| {
                        ui.label("End value:");
                        ui.add(egui::DragValue::new(&mut doc.sequence_fill_dialog.end_value).range(0..=9999));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Hold frames:");
                        ui.add(egui::DragValue::new(&mut doc.sequence_fill_dialog.hold_frames).range(1..=100));
                    });

                    // 预览信息
                    let value_count = if doc.sequence_fill_dialog.end_value >= doc.sequence_fill_dialog.start_value {
                        doc.sequence_fill_dialog.end_value - doc.sequence_fill_dialog.start_value + 1
                    } else {
                        doc.sequence_fill_dialog.start_value - doc.sequence_fill_dialog.end_value + 1
                    };
                    let total_frames = value_count * doc.sequence_fill_dialog.hold_frames;
                    ui.label(format!("Total: {} frames", total_frames));

                    ui.separator();

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() || enter_pressed {
                            should_execute = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });

            if should_cancel {
                doc.sequence_fill_dialog.open = false;
            }

            if should_execute {
                let layer = doc.sequence_fill_dialog.layer;
                let start_frame = doc.sequence_fill_dialog.start_frame;
                let start_value = doc.sequence_fill_dialog.start_value;
                let end_value = doc.sequence_fill_dialog.end_value;
                let hold_frames = doc.sequence_fill_dialog.hold_frames;

                if let Err(e) = doc.sequence_fill(layer, start_frame, start_value, end_value, hold_frames) {
                    self.error_message = Some(e.to_string());
                } else if auto_save_enabled {
                    doc.auto_save();
                }
                doc.sequence_fill_dialog.open = false;
            }
        }

        // 检测鼠标交互，更新活跃文档
        let doc = &self.documents[doc_idx];
        if ui.ui_contains_pointer() || doc.edit_state.editing_cell.is_some() {
            self.active_doc_id = Some(doc.id);
        }

        // 处理快捷键 - 只处理活跃文档
        if self.active_doc_id == Some(doc.id) {
            self.handle_document_shortcuts(ctx, doc_idx, layer_count);
        }
    }


    fn handle_document_shortcuts(&mut self, ctx: &egui::Context, doc_idx: usize, layer_count: usize) {
        let auto_save_enabled = self.settings.auto_save_enabled;
        let doc = &mut self.documents[doc_idx];

        // 如果有对话框打开，不处理键盘事件
        if doc.repeat_dialog.open || doc.sequence_fill_dialog.open {
            return;
        }

        let doc_id = doc.id;

        let mut should_copy = false;
        let mut should_cut = false;
        let mut should_paste = false;
        let mut paste_text: Option<String> = None;
        let mut should_undo = false;
        let mut should_delete = false;
        let mut should_save = false;

        let is_editing = doc.edit_state.editing_cell.is_some() || doc.edit_state.editing_layer_name.is_some();
        let mut jump_step_delta: i32 = 0;

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Copy => should_copy = true,
                    egui::Event::Cut => should_cut = true,
                    egui::Event::Paste(text) => {
                        should_paste = true;
                        paste_text = Some(text.clone());
                    }
                    // Detect / and * characters for jump step (only when not editing)
                    egui::Event::Text(text) if !is_editing => {
                        if text == "/" {
                            jump_step_delta = -1;
                        } else if text == "*" {
                            jump_step_delta = 1;
                        }
                    }
                    _ => {}
                }
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                should_undo = true;
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                should_save = true;
            }

            if i.key_pressed(egui::Key::Delete) {
                should_delete = true;
            }
        });

        if should_save {
            self.save_document(doc_id);
            return;
        }

        // Update jump step (only when not editing)
        if jump_step_delta != 0 {
            let new_step = (doc.jump_step as i32 + jump_step_delta).max(1) as usize;
            doc.jump_step = new_step;
        }

        if should_undo {
            doc.undo();
            if auto_save_enabled { doc.auto_save(); }
        }

        if !is_editing && should_delete {
            doc.delete_selection();
            if auto_save_enabled { doc.auto_save(); }
        }

        if !is_editing && (should_copy || should_cut || should_paste) {
            if should_copy {
                if doc.selection_state.selection_start.is_some() && doc.selection_state.selection_end.is_some() {
                    doc.copy_selection(ctx);
                } else if let Some((layer, frame)) = doc.selection_state.selected_cell {
                    doc.selection_state.selection_start = Some((layer, frame));
                    doc.selection_state.selection_end = Some((layer, frame));
                    doc.copy_selection(ctx);
                }
            } else if should_cut {
                if doc.selection_state.selection_start.is_some() && doc.selection_state.selection_end.is_some() {
                    doc.cut_selection(ctx);
                    if auto_save_enabled { doc.auto_save(); }
                } else if let Some((layer, frame)) = doc.selection_state.selected_cell {
                    doc.selection_state.selection_start = Some((layer, frame));
                    doc.selection_state.selection_end = Some((layer, frame));
                    doc.cut_selection(ctx);
                    if auto_save_enabled { doc.auto_save(); }
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                }
            } else if should_paste {
                // 优先从系统剪贴板文本粘贴，失败则回退到内部剪贴板
                let pasted = if let Some(ref text) = paste_text {
                    doc.paste_from_text(text)
                } else {
                    false
                };
                if !pasted {
                    doc.paste_clipboard();
                }
                if auto_save_enabled { doc.auto_save(); }
            }
        }

        // 编辑模式键盘处理
        if let Some((layer, frame)) = doc.edit_state.editing_cell {
            let has_input = !doc.edit_state.editing_text.is_empty();
            let total_frames = doc.timesheet.total_frames();
            let mut did_edit = false;

            ctx.input(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    doc.finish_edit(true, true);
                    doc.selection_state.auto_scroll_to_selection = true;
                    did_edit = true;
                } else if i.key_pressed(egui::Key::Escape) {
                    doc.edit_state.editing_cell = None;
                    doc.edit_state.editing_text.clear();
                } else {
                    let new_pos = if i.key_pressed(egui::Key::ArrowUp) && frame > 0 {
                        Some((layer, frame - 1))
                    } else if i.key_pressed(egui::Key::ArrowDown) && frame + 1 < total_frames {
                        Some((layer, frame + 1))
                    } else if i.key_pressed(egui::Key::ArrowLeft) && layer > 0 {
                        Some((layer - 1, frame))
                    } else if i.key_pressed(egui::Key::ArrowRight) && layer < layer_count - 1 {
                        Some((layer + 1, frame))
                    } else {
                        None
                    };

                    if let Some(pos) = new_pos {
                        if has_input {
                            doc.finish_edit(false, true);
                            doc.start_edit(pos.0, pos.1);
                            did_edit = true;
                        } else {
                            doc.edit_state.editing_cell = None;
                            doc.edit_state.editing_text.clear();
                        }
                        doc.selection_state.selected_cell = Some(pos);
                        doc.selection_state.auto_scroll_to_selection = true;
                    }
                }
            });

            if did_edit && auto_save_enabled {
                doc.auto_save();
            }
        } else if let Some((layer, frame)) = doc.selection_state.selected_cell {
            let total_frames = doc.timesheet.total_frames();
            let mut did_modify = false;

            ctx.input(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    let (old_value, new_value) = if frame > 0 {
                        let old = doc.timesheet.get_cell(layer, frame).copied();
                        let new = doc.timesheet.get_cell(layer, frame - 1).copied();
                        (old, new)
                    } else {
                        (None, None)
                    };

                    if old_value != new_value && new_value.is_some() {
                        doc.push_undo_set_cell(layer, frame, old_value);
                        doc.is_modified = true;
                        doc.timesheet.set_cell(layer, frame, new_value);
                        did_modify = true;
                    }

                    // 使用 jump_step 计算新位置
                    let new_frame = frame + doc.jump_step;
                    // 当 step > 1 时，填充跳过的格子为 Same 标记
                    if doc.jump_step > 1 && new_value.is_some() {
                        for skip_frame in (frame + 1)..new_frame.min(total_frames) {
                            let old_skip_value = doc.timesheet.get_cell(layer, skip_frame).copied();
                            if old_skip_value != Some(CellValue::Same) {
                                doc.push_undo_set_cell(layer, skip_frame, old_skip_value);
                                doc.timesheet.set_cell(layer, skip_frame, Some(CellValue::Same));
                            }
                        }
                        did_modify = true;
                    }
                    if new_frame < total_frames {
                        doc.selection_state.selected_cell = Some((layer, new_frame));
                        doc.selection_state.auto_scroll_to_selection = true;
                    }
                } else if i.key_pressed(egui::Key::Tab) && layer < layer_count - 1 {
                    doc.selection_state.selected_cell = Some((layer + 1, frame));
                    doc.selection_state.auto_scroll_to_selection = true;
                } else {
                    let new_pos = if i.key_pressed(egui::Key::ArrowUp) && frame > 0 {
                        Some((layer, frame - 1))
                    } else if i.key_pressed(egui::Key::ArrowDown) && frame + 1 < total_frames {
                        Some((layer, frame + 1))
                    } else if i.key_pressed(egui::Key::ArrowLeft) && layer > 0 {
                        Some((layer - 1, frame))
                    } else if i.key_pressed(egui::Key::ArrowRight) && layer < layer_count - 1 {
                        Some((layer + 1, frame))
                    } else {
                        None
                    };

                    if let Some(pos) = new_pos {
                        doc.selection_state.selected_cell = Some(pos);
                        doc.selection_state.auto_scroll_to_selection = true;
                    } else {
                        for event in &i.events {
                            if let egui::Event::Text(text) = event {
                                if text.chars().all(|c| c.is_ascii_digit()) {
                                    // 如果有选区，使用批量编辑模式
                                    if doc.get_selection_range().is_some() {
                                        doc.start_batch_edit(layer, frame);
                                    } else {
                                        doc.start_edit(layer, frame);
                                    }
                                    doc.edit_state.editing_text = text.clone();
                                    break;
                                }
                            }
                        }
                    }
                }
            });

            if did_modify && auto_save_enabled {
                doc.auto_save();
            }
        }
    }
}
