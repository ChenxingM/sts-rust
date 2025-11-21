#![windows_subsystem = "windows"]
#![allow(dead_code)] // Allow unused helper functions for future use

use eframe::egui;
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;
use std::rc::Rc;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("STS 3.0"),
        ..Default::default()
    };

    eframe::run_native(
        "STS 3.0",
        options,
        Box::new(|_cc| Ok(Box::new(StsApp::default()))),
    )
}

const DASH: &str = "-";

// 单元格渲染颜色常量
const BG_EDITING: egui::Color32 = egui::Color32::from_rgb(255, 255, 200);
const BG_SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 220, 255);
const BG_IN_SELECTION: egui::Color32 = egui::Color32::from_rgb(220, 235, 255);
const BG_NORMAL: egui::Color32 = egui::Color32::WHITE;
const BORDER_SELECTION: egui::Color32 = egui::Color32::from_rgb(100, 150, 255);
const BORDER_NORMAL: egui::Color32 = egui::Color32::GRAY;

// 撤销栈限制
const MAX_UNDO_ACTIONS: usize = 100;
const MAX_UNDO_MEMORY_BYTES: usize = 1024 * 1024; // 1MB

// 撤销操作类型
#[derive(Clone)]
enum UndoAction {
    SetCell {
        layer: usize,
        frame: usize,
        old_value: Option<CellValue>,
    },
    SetRange {
        min_layer: usize,
        min_frame: usize,
        old_values: Rc<Vec<Vec<Option<CellValue>>>>,
    },
}

// 编辑状态
struct EditState {
    editing_cell: Option<(usize, usize)>,
    editing_layer_name: Option<usize>,
    // 使用 String 但初始容量更小
    editing_text: String,
    editing_layer_text: String,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            editing_cell: None,
            editing_layer_name: None,
            // 初始不分配内存
            editing_text: String::new(),
            editing_layer_text: String::new(),
        }
    }
}

// 选择状态
struct SelectionState {
    selected_cell: Option<(usize, usize)>,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    is_dragging: bool,
    auto_scroll_to_selection: bool,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            selected_cell: Some((0, 0)),
            selection_start: None,
            selection_end: None,
            is_dragging: false,
            auto_scroll_to_selection: false,
        }
    }
}

// 上下文菜单状态
struct ContextMenuState {
    pos: Option<(usize, usize)>,
    screen_pos: egui::Pos2,
    selection: Option<((usize, usize), (usize, usize))>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            pos: None,
            screen_pos: egui::Pos2::ZERO,
            selection: None,
        }
    }
}

// 剪贴板数据
type ClipboardData = Rc<Vec<Vec<Option<CellValue>>>>;

// 文档结构 - 每个打开的文件对应一个Document
struct Document {
    id: usize,
    timesheet: Box<TimeSheet>,
    file_path: Option<Box<str>>,
    is_modified: bool,
    is_open: bool,
    edit_state: EditState,
    selection_state: SelectionState,
    context_menu: ContextMenuState,
    clipboard: Option<ClipboardData>,
    // 使用 Box 减少 Vec 的栈大小
    undo_stack: Box<Vec<UndoAction>>,
}

impl Document {
    fn new(id: usize, timesheet: TimeSheet, file_path: Option<String>) -> Self {
        Self {
            id,
            timesheet: Box::new(timesheet),
            file_path: file_path.map(|s| s.into_boxed_str()),
            is_modified: false,
            is_open: true,
            edit_state: EditState::default(),
            selection_state: SelectionState::default(),
            context_menu: ContextMenuState::default(),
            clipboard: None,
            undo_stack: Box::new(Vec::new()),
        }
    }

    fn title(&self) -> String {
        let base = if let Some(path) = &self.file_path {
            format!("{} - {}", self.timesheet.name, path)
        } else {
            self.timesheet.name.clone()
        };

        if self.is_modified {
            format!("{}*", base)
        } else {
            base
        }
    }

    fn save(&mut self) -> Result<(), String> {
        if let Some(path) = &self.file_path {
            match sts_rust::write_sts_file(&self.timesheet, path) {
                Ok(_) => {
                    self.is_modified = false;
                    Ok(())
                }
                Err(e) => Err(format!("Failed to save: {}", e)),
            }
        } else {
            Err("No file path".to_string())
        }
    }

    fn save_as(&mut self, path: String) -> Result<(), String> {
        match sts_rust::write_sts_file(&self.timesheet, &path) {
            Ok(_) => {
                self.file_path = Some(path.into_boxed_str());
                self.is_modified = false;
                Ok(())
            }
            Err(e) => Err(format!("Failed to save: {}", e)),
        }
    }

    #[inline]
    fn start_edit(&mut self, layer: usize, frame: usize) {
        self.edit_state.editing_cell = Some((layer, frame));
        self.edit_state.editing_text.clear();

        match self.timesheet.get_cell(layer, frame) {
            Some(CellValue::Number(n)) => {
                let mut buf = itoa::Buffer::new();
                self.edit_state.editing_text.push_str(buf.format(*n));
            }
            Some(CellValue::Same) => {
                if frame > 0 {
                    if let Some(CellValue::Number(n)) = self.timesheet.get_cell(layer, frame - 1) {
                        let mut buf = itoa::Buffer::new();
                        self.edit_state.editing_text.push_str(buf.format(*n));
                    }
                }
            }
            None => {}
        }
    }

    #[inline]
    fn finish_edit(&mut self, move_down: bool, record_undo: bool) {
        if let Some((layer, frame)) = self.edit_state.editing_cell {
            let old_value = self.timesheet.get_cell(layer, frame).copied();

            let value = if self.edit_state.editing_text.trim().is_empty() {
                if frame > 0 {
                    self.timesheet.get_cell(layer, frame - 1).copied()
                } else {
                    None
                }
            } else if let Ok(n) = self.edit_state.editing_text.trim().parse::<u32>() {
                Some(CellValue::Number(n))
            } else {
                None
            };

            if record_undo && old_value != value {
                self.push_undo_set_cell(layer, frame, old_value);
                self.is_modified = true;
            }

            self.timesheet.set_cell(layer, frame, value);

            if move_down {
                self.selection_state.selected_cell = Some((layer, frame + 1));
            }

            self.edit_state.editing_cell = None;
            self.edit_state.editing_text.clear();
        }
    }

    #[inline(always)]
    fn is_cell_in_selection(&self, layer: usize, frame: usize) -> bool {
        if let (Some((start_layer, start_frame)), Some((end_layer, end_frame))) =
            (self.selection_state.selection_start, self.selection_state.selection_end) {
            let min_layer = start_layer.min(end_layer);
            let max_layer = start_layer.max(end_layer);
            let min_frame = start_frame.min(end_frame);
            let max_frame = start_frame.max(end_frame);

            layer >= min_layer && layer <= max_layer &&
            frame >= min_frame && frame <= max_frame
        } else {
            false
        }
    }

    #[inline]
    fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        if let (Some((start_layer, start_frame)), Some((end_layer, end_frame))) =
            (self.selection_state.selection_start, self.selection_state.selection_end) {
            let min_layer = start_layer.min(end_layer);
            let max_layer = start_layer.max(end_layer);
            let min_frame = start_frame.min(end_frame);
            let max_frame = start_frame.max(end_frame);
            Some((min_layer, min_frame, max_layer, max_frame))
        } else {
            None
        }
    }

    #[inline]
    fn copy_selection(&mut self, ctx: &egui::Context) {
        let range = self.get_selection_range();

        if let Some((min_layer, min_frame, max_layer, max_frame)) = range {
            let row_count = max_layer - min_layer + 1;
            let col_count = max_frame - min_frame + 1;

            // 预分配容量
            let mut clipboard_data = Vec::with_capacity(row_count);
            let mut clipboard_text = String::with_capacity(row_count * col_count * 4);

            for layer in min_layer..=max_layer {
                let mut row = Vec::with_capacity(col_count);

                for frame in min_frame..=max_frame {
                    let cell = self.timesheet.get_cell(layer, frame).copied();
                    row.push(cell);

                    if frame > min_frame {
                        clipboard_text.push('\t');
                    }
                    match cell {
                        Some(CellValue::Number(n)) => {
                            let mut buf = itoa::Buffer::new();
                            clipboard_text.push_str(buf.format(n));
                        }
                        Some(CellValue::Same) => clipboard_text.push('-'),
                        None => {}
                    }
                }
                clipboard_data.push(row);
                if layer < max_layer {
                    clipboard_text.push('\n');
                }
            }

            if !clipboard_data.is_empty() {
                self.clipboard = Some(Rc::new(clipboard_data));
                ctx.output_mut(|o| o.copied_text = clipboard_text);
            }
        }
    }

    fn cut_selection(&mut self, ctx: &egui::Context) {
        self.copy_selection(ctx);

        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            let mut old_values = Vec::new();
            for layer in min_layer..=max_layer {
                let mut old_row = Vec::new();
                for frame in min_frame..=max_frame {
                    old_row.push(self.timesheet.get_cell(layer, frame).copied());
                }
                old_values.push(old_row);
            }

            self.undo_stack.push(UndoAction::SetRange {
                min_layer,
                min_frame,
                old_values: Rc::new(old_values),
            });
            self.is_modified = true;

            for layer in min_layer..=max_layer {
                for frame in min_frame..=max_frame {
                    self.timesheet.set_cell(layer, frame, None);
                }
            }
        }
    }

    fn delete_selection(&mut self) {
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            let mut old_values = Vec::new();
            for layer in min_layer..=max_layer {
                let mut old_row = Vec::new();
                for frame in min_frame..=max_frame {
                    old_row.push(self.timesheet.get_cell(layer, frame).copied());
                }
                old_values.push(old_row);
            }

            self.undo_stack.push(UndoAction::SetRange {
                min_layer,
                min_frame,
                old_values: Rc::new(old_values),
            });
            self.is_modified = true;

            for layer in min_layer..=max_layer {
                for frame in min_frame..=max_frame {
                    self.timesheet.set_cell(layer, frame, None);
                }
            }
        } else if let Some((layer, frame)) = self.selection_state.selected_cell {
            let old_value = self.timesheet.get_cell(layer, frame).copied();
            self.push_undo_set_cell(layer, frame, old_value);
            self.is_modified = true;
            self.timesheet.set_cell(layer, frame, None);
        }
    }

    fn paste_clipboard(&mut self) {
        if let Some((start_layer, start_frame)) = self.selection_state.selected_cell {
            if let Some(ref clipboard) = self.clipboard {
                let mut old_values = Vec::new();
                for (layer_offset, row) in clipboard.iter().enumerate() {
                    let target_layer = start_layer + layer_offset;
                    let mut old_row = Vec::new();
                    for (frame_offset, _) in row.iter().enumerate() {
                        let target_frame = start_frame + frame_offset;
                        old_row.push(self.timesheet.get_cell(target_layer, target_frame).copied());
                    }
                    old_values.push(old_row);
                }

                self.undo_stack.push(UndoAction::SetRange {
                    min_layer: start_layer,
                    min_frame: start_frame,
                    old_values: Rc::new(old_values),
                });
                self.is_modified = true;

                for (layer_offset, row) in clipboard.iter().enumerate() {
                    let target_layer = start_layer + layer_offset;
                    for (frame_offset, cell) in row.iter().enumerate() {
                        let target_frame = start_frame + frame_offset;
                        self.timesheet.set_cell(target_layer, target_frame, *cell);
                    }
                }
            }
        }
    }

    fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action {
                UndoAction::SetCell { layer, frame, old_value } => {
                    self.timesheet.set_cell(layer, frame, old_value);
                }
                UndoAction::SetRange { min_layer, min_frame, old_values } => {
                    for (layer_offset, row) in old_values.iter().enumerate() {
                        for (frame_offset, value) in row.iter().enumerate() {
                            self.timesheet.set_cell(
                                min_layer + layer_offset,
                                min_frame + frame_offset,
                                *value,
                            );
                        }
                    }
                }
            }
            self.is_modified = true;
        }
    }

    #[inline]
    fn push_undo_set_cell(&mut self, layer: usize, frame: usize, old_value: Option<CellValue>) {
        if self.undo_stack.len() >= MAX_UNDO_ACTIONS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoAction::SetCell {
            layer,
            frame,
            old_value,
        });
    }

    // 估算撤销操作占用的内存
    #[inline]
    fn estimate_undo_memory(&self) -> usize {
        self.undo_stack.iter().map(|action| {
            match action {
                UndoAction::SetCell { .. } => std::mem::size_of::<UndoAction>(),
                UndoAction::SetRange { old_values, .. } => {
                    std::mem::size_of::<UndoAction>() +
                    old_values.len() * old_values.first().map_or(0, |row| row.len() * std::mem::size_of::<Option<CellValue>>())
                }
            }
        }).sum()
    }
}

struct StsApp {
    documents: Vec<Document>,
    next_doc_id: usize,
    active_doc_id: Option<usize>,
    show_new_dialog: bool,
    closing_doc_id: Option<usize>,
    new_name: String,
    new_framerate: u32,
    new_layer_count: usize,
    new_frames_per_page: u32,
    new_seconds: u32,
    new_frames: u32,
    error_message: Option<String>,
}

impl Default for StsApp {
    fn default() -> Self {
        Self {
            documents: Vec::new(),
            next_doc_id: 0,
            active_doc_id: None,
            show_new_dialog: false,
            closing_doc_id: None,
            new_name: "sheet1".to_string(),
            new_framerate: 24,
            new_layer_count: 12,
            new_frames_per_page: 144,
            new_seconds: 6,
            new_frames: 0,
            error_message: None,
        }
    }
}

impl StsApp {
    fn create_new_document(&mut self) {
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

    fn open_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STS Files", &["sts"])
            .pick_file()
        {
            let path_str = path.to_str().unwrap();

            // 检查文件是否已打开
            if let Some(_existing) = self.documents.iter().find(|d| {
                d.file_path.as_ref().map_or(false, |p| p.as_ref() == path_str)
            }) {
                self.error_message = Some(format!("File is already open: {}", path_str));
                return;
            }

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
    }

    fn save_document(&mut self, doc_id: usize) {
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

    fn save_document_as(&mut self, doc_id: usize) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STS Files", &["sts"])
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
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 只在首次设置视觉样式
        static mut STYLE_INITIALIZED: bool = false;
        unsafe {
            if !STYLE_INITIALIZED {
                ctx.set_visuals(egui::Visuals::light());

                let mut style = (*ctx.style()).clone();
                style.spacing.window_margin = egui::Margin::same(4.0);
                style.text_styles.insert(
                    egui::TextStyle::Heading,
                    egui::FontId::proportional(14.0),
                );
                ctx.set_style(style);

                STYLE_INITIALIZED = true;
            }
        }

        // 全局快捷键
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.show_new_dialog = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.open_document();
            }
        });

        // 菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New (Ctrl+N)").clicked() {
                        self.show_new_dialog = true;
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
            });
        });

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
                        ui.text_edit_singleline(&mut self.new_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Layers:");
                        ui.add(egui::DragValue::new(&mut self.new_layer_count).range(1..=26));
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

                    if ui.button("OK").clicked() {
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
                (doc.title(), doc.id, doc.is_open)
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
                            });

                            ui.separator();

                            // 文档信息
                            let (name, total_frames) = {
                                let doc = &self.documents[doc_idx];
                                (doc.timesheet.name.clone(), doc.timesheet.total_frames())
                            };

                            ui.horizontal(|ui| {
                                ui.label(&name);
                                ui.separator();
                                ui.label("Total Frames:");
                                let mut frames_buf = itoa::Buffer::new();
                                ui.label(frames_buf.format(total_frames));
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

                    ui.horizontal(|ui| {
                        if ui.add_sized([80.0, 25.0], egui::Button::new("Save")).clicked() {
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
    fn render_document_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, doc_idx: usize) {
        let doc = &mut self.documents[doc_idx];

        let row_height = 16.0;
        let col_width = 36.0;
        let page_col_width = 36.0;
        let layer_count = doc.timesheet.layer_count;

        // 表头
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let (_corner_id, corner_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));
            ui.painter().rect_stroke(
                corner_rect,
                0.0,
                egui::Stroke::new(0.0, egui::Color32::GRAY),
            );

            for i in 0..layer_count {
                let (id, rect) = ui.allocate_space(egui::vec2(col_width, row_height));
                let is_editing = doc.edit_state.editing_layer_name == Some(i);

                let bg_color = if is_editing {
                    egui::Color32::from_rgb(255, 255, 200)
                } else {
                    egui::Color32::from_rgb(240, 240, 240)
                };
                ui.painter().rect_filled(rect, 0.0, bg_color);
                ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

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
                        egui::Color32::BLACK,
                    );

                    if resp.clicked() {
                        doc.edit_state.editing_layer_name = Some(i);
                        doc.edit_state.editing_layer_text = layer_name.clone();
                    }
                }
            }
        });

        ui.separator();

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
                            egui::Stroke::new(1.0, egui::Color32::GRAY),
                        );

                        ui.painter().text(
                            page_rect.left_center() + egui::vec2(3.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            page_str,
                            egui::FontId::monospace(11.0),
                            egui::Color32::DARK_GRAY,
                        );

                        if !frame_str.is_empty() {
                            ui.painter().text(
                                page_rect.right_center() - egui::vec2(3.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                frame_str,
                                egui::FontId::monospace(11.0),
                                egui::Color32::DARK_GRAY,
                            );
                        }

                        // 单元格渲染
                        for layer_idx in 0..layer_count {
                            render_cell_inline(ui, doc, layer_idx, frame_idx, col_width, row_height, pointer_pos, pointer_down);
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

                        (copy, cut, paste, undo)
                    }).inner
                });

            let (copy_clicked, cut_clicked, paste_clicked, undo_clicked) = menu_result.inner;
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
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                } else if let Some((layer, frame)) = doc.context_menu.pos {
                    doc.selection_state.selection_start = Some((layer, frame));
                    doc.selection_state.selection_end = Some((layer, frame));
                    doc.cut_selection(ctx);
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                }
                doc.context_menu.pos = None;
            } else if paste_clicked {
                if let Some((layer, frame)) = doc.context_menu.pos {
                    doc.selection_state.selected_cell = Some((layer, frame));
                }
                doc.paste_clipboard();
                doc.context_menu.pos = None;
            } else if undo_clicked {
                doc.undo();
                doc.context_menu.pos = None;
            }

            // 点击菜单外部关闭
            if !copy_clicked && !cut_clicked && !paste_clicked && !undo_clicked {
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
        let doc = &mut self.documents[doc_idx];
        let doc_id = doc.id;

        let mut should_copy = false;
        let mut should_cut = false;
        let mut should_paste = false;
        let mut should_undo = false;
        let mut should_delete = false;
        let mut should_save = false;

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Copy => should_copy = true,
                    egui::Event::Cut => should_cut = true,
                    egui::Event::Paste(_) => should_paste = true,
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

        let is_editing = doc.edit_state.editing_cell.is_some() || doc.edit_state.editing_layer_name.is_some();

        if should_undo {
            doc.undo();
        }

        if !is_editing && should_delete {
            doc.delete_selection();
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
                } else if let Some((layer, frame)) = doc.selection_state.selected_cell {
                    doc.selection_state.selection_start = Some((layer, frame));
                    doc.selection_state.selection_end = Some((layer, frame));
                    doc.cut_selection(ctx);
                    doc.selection_state.selection_start = None;
                    doc.selection_state.selection_end = None;
                }
            } else if should_paste {
                doc.paste_clipboard();
            }
        }

        // 编辑模式键盘处理
        if let Some((layer, frame)) = doc.edit_state.editing_cell {
            let has_input = !doc.edit_state.editing_text.is_empty();

            ctx.input(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    doc.finish_edit(true, true);
                    doc.selection_state.auto_scroll_to_selection = true;
                } else if i.key_pressed(egui::Key::Escape) {
                    doc.edit_state.editing_cell = None;
                    doc.edit_state.editing_text.clear();
                } else {
                    let new_pos = if i.key_pressed(egui::Key::ArrowUp) && frame > 0 {
                        Some((layer, frame - 1))
                    } else if i.key_pressed(egui::Key::ArrowDown) {
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
                        } else {
                            doc.edit_state.editing_cell = None;
                            doc.edit_state.editing_text.clear();
                        }
                        doc.selection_state.selected_cell = Some(pos);
                        doc.selection_state.auto_scroll_to_selection = true;
                    }
                }
            });
        } else if let Some((layer, frame)) = doc.selection_state.selected_cell {
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
                    }

                    doc.selection_state.selected_cell = Some((layer, frame + 1));
                    doc.selection_state.auto_scroll_to_selection = true;
                } else if i.key_pressed(egui::Key::Tab) && layer < layer_count - 1 {
                    doc.selection_state.selected_cell = Some((layer + 1, frame));
                    doc.selection_state.auto_scroll_to_selection = true;
                } else {
                    let new_pos = if i.key_pressed(egui::Key::ArrowUp) && frame > 0 {
                        Some((layer, frame - 1))
                    } else if i.key_pressed(egui::Key::ArrowDown) {
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
                                    doc.start_edit(layer, frame);
                                    doc.edit_state.editing_text = text.clone();
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}

// 独立的单元格渲染函数
#[inline]
fn render_cell_inline(
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
