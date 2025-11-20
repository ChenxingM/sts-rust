#![windows_subsystem = "windows"]

use eframe::egui;
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 800.0])
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

// 撤销操作类型
#[derive(Clone)]
enum UndoAction {
    SetCell {
        layer: usize,
        frame: usize,
        old_value: Option<CellValue>,
        new_value: Option<CellValue>,
    },
    SetRange {
        min_layer: usize,
        min_frame: usize,
        old_values: Vec<Vec<Option<CellValue>>>,
    },
}

#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
}

struct StsApp {
    timesheet: Option<TimeSheet>,
    current_file_path: Option<String>,        // 当前文件路径
    is_modified: bool,                        // 文件是否被修改
    show_new_dialog: bool,
    show_confirm_dialog: bool,
    pending_action: Option<PendingAction>,
    new_name: String,
    new_framerate: u32,
    new_layer_count: usize,
    new_frames_per_page: u32,
    new_seconds: u32,
    new_frames: u32,
    selected_cell: Option<(usize, usize)>,
    editing_cell: Option<(usize, usize)>,
    editing_text: String,
    editing_layer_name: Option<usize>,
    editing_layer_text: String,
    error_message: Option<String>,
    auto_scroll_to_selection: bool,
    // 多选相关
    selection_start: Option<(usize, usize)>,  // 选区起点
    selection_end: Option<(usize, usize)>,    // 选区终点
    is_dragging: bool,                         // 是否正在拖拽选择
    // 剪贴板
    clipboard: Vec<Vec<Option<CellValue>>>,   // 剪贴板数据 [layer][frame]
    // 撤销栈
    undo_stack: Vec<UndoAction>,              // 撤销历史
    // 右键菜单
    context_menu_pos: Option<(usize, usize)>, // 右键点击位置 (layer, frame)
    context_menu_screen_pos: egui::Pos2,      // 菜单屏幕位置
    context_menu_selection: Option<((usize, usize), (usize, usize))>, // 右键时的选区状态
}

impl Default for StsApp {
    fn default() -> Self {
        Self {
            timesheet: None,
            current_file_path: None,
            is_modified: false,
            show_new_dialog: true,
            show_confirm_dialog: false,
            pending_action: None,
            new_name: "sheet1".to_string(),
            new_framerate: 24,
            new_layer_count: 12,
            new_frames_per_page: 144,
            new_seconds: 6,           // 默认 6 秒
            new_frames: 0,            // 默认 0 帧
            selected_cell: None,
            editing_cell: None,
            editing_text: String::new(),
            editing_layer_name: None,
            editing_layer_text: String::new(),
            error_message: None,
            auto_scroll_to_selection: false,
            selection_start: None,
            selection_end: None,
            is_dragging: false,
            clipboard: Vec::new(),
            undo_stack: Vec::new(),
            context_menu_pos: None,
            context_menu_screen_pos: egui::Pos2::ZERO,
            context_menu_selection: None,
        }
    }
}

impl StsApp {
    /// 打开文件
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("STS Files", &["sts"])
            .pick_file()
        {
            let path_str = path.to_str().unwrap();
            match sts_rust::parse_sts_file(path_str) {
                Ok(ts) => {
                    self.timesheet = Some(ts);
                    self.current_file_path = Some(path_str.to_string());
                    self.is_modified = false;
                    self.selected_cell = Some((0, 0));
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to open: {}", e));
                }
            }
        }
    }

    /// 保存文件
    fn save_file(&mut self) {
        if let Some(ts) = &self.timesheet {
            // 如果已经有文件路径，直接保存
            if let Some(path) = &self.current_file_path {
                match sts_rust::write_sts_file(ts, path) {
                    Ok(_) => {
                        self.error_message = None;
                        self.is_modified = false;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to save: {}", e));
                    }
                }
            } else {
                // 没有路径，弹出对话框
                self.save_file_as();
            }
        }
    }

    /// 另存为
    fn save_file_as(&mut self) {
        if let Some(ts) = &self.timesheet {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STS Files", &["sts"])
                .save_file()
            {
                let path_str = path.to_str().unwrap();
                match sts_rust::write_sts_file(ts, path_str) {
                    Ok(_) => {
                        self.current_file_path = Some(path_str.to_string());
                        self.is_modified = false;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to save: {}", e));
                    }
                }
            }
        }
    }

    /// 创建新摄影表
    #[inline]
    fn create_new_timesheet(&mut self) {
        // 计算总帧数 = 秒 * fps + 帧
        let total_frames = (self.new_seconds * self.new_framerate + self.new_frames) as usize;

        let mut ts = TimeSheet::new(
            self.new_name.clone(),
            self.new_framerate,
            self.new_layer_count,
            self.new_frames_per_page,
        );
        ts.ensure_frames(total_frames.max(1)); // 至少 1 帧
        self.timesheet = Some(ts);
        self.current_file_path = None;  // 新建时清除文件路径
        self.is_modified = false;       // 新建时清除修改标志
        self.show_new_dialog = false;
        self.selected_cell = Some((0, 0));
    }

    /// 开始编辑单元格
    #[inline]
    fn start_edit(&mut self, layer: usize, frame: usize) {
        if let Some(ts) = &self.timesheet {
            self.editing_cell = Some((layer, frame));
            self.editing_text = match ts.get_cell(layer, frame) {
                Some(CellValue::Number(n)) => n.to_string(),
                Some(CellValue::Same) => {
                    if frame > 0 {
                        if let Some(CellValue::Number(n)) = ts.get_cell(layer, frame - 1) {
                            n.to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                }
                None => String::new(),
            };
        }
    }

    /// 完成编辑 - record_undo 控制是否记录撤销
    #[inline]
    fn finish_edit(&mut self, move_down: bool, record_undo: bool) {
        if let Some((layer, frame)) = self.editing_cell {
            // 先获取旧值和新值
            let (old_value, new_value) = if let Some(ts) = &self.timesheet {
                let old_value = ts.get_cell(layer, frame).copied();

                let value = if self.editing_text.trim().is_empty() {
                    // 空输入 → 复制上一格的值
                    if frame > 0 {
                        ts.get_cell(layer, frame - 1).copied()
                    } else {
                        None
                    }
                } else if let Ok(n) = self.editing_text.trim().parse::<u32>() {
                    Some(CellValue::Number(n))
                } else {
                    None
                };

                (old_value, value)
            } else {
                (None, None)
            };

            // 只在值改变且需要记录时记录撤销
            if record_undo && old_value != new_value {
                self.push_undo_set_cell(layer, frame, old_value, new_value);
                self.is_modified = true;
            }

            // 设置新值
            if let Some(ts) = &mut self.timesheet {
                ts.set_cell(layer, frame, new_value);
            }

            if move_down {
                self.selected_cell = Some((layer, frame + 1));
            }

            self.editing_cell = None;
            self.editing_text.clear();
        }
    }

    /// 检查单元格是否在选区内
    #[inline(always)]
    fn is_cell_in_selection(&self, layer: usize, frame: usize) -> bool {
        if let (Some((start_layer, start_frame)), Some((end_layer, end_frame))) =
            (self.selection_start, self.selection_end) {
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

    /// 获取选区范围 (min_layer, min_frame, max_layer, max_frame)
    #[inline]
    fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        if let (Some((start_layer, start_frame)), Some((end_layer, end_frame))) =
            (self.selection_start, self.selection_end) {
            let min_layer = start_layer.min(end_layer);
            let max_layer = start_layer.max(end_layer);
            let min_frame = start_frame.min(end_frame);
            let max_frame = start_frame.max(end_frame);
            Some((min_layer, min_frame, max_layer, max_frame))
        } else {
            None
        }
    }

    /// 复制选区到系统剪贴板
    #[inline]
    fn copy_selection(&mut self, ctx: &egui::Context) {
        // 先清空剪贴板
        self.clipboard.clear();

        // 获取选区范围
        let range = self.get_selection_range();

        if let Some((min_layer, min_frame, max_layer, max_frame)) = range {
            if let Some(ts) = &self.timesheet {
                let mut text_rows = Vec::new();

                for layer in min_layer..=max_layer {
                    let mut row = Vec::new();
                    let mut text_cols = Vec::new();

                    for frame in min_frame..=max_frame {
                        let cell = ts.get_cell(layer, frame).copied();
                        row.push(cell);

                        // 转换为文本
                        let text = match cell {
                            Some(CellValue::Number(n)) => n.to_string(),
                            Some(CellValue::Same) => "-".to_string(),
                            None => "".to_string(),
                        };
                        text_cols.push(text);
                    }
                    self.clipboard.push(row);
                    text_rows.push(text_cols.join("\t"));
                }

                // 复制到系统剪贴板（TSV格式）
                if !self.clipboard.is_empty() {
                    let clipboard_text = text_rows.join("\n");
                    ctx.output_mut(|o| o.copied_text = clipboard_text);
                }
            }
        }
    }

    /// 剪切选区到剪贴板
    fn cut_selection(&mut self, ctx: &egui::Context) {
        self.copy_selection(ctx);

        // 清空选区内容并记录撤销
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            if let Some(ts) = &mut self.timesheet {
                // 保存旧值
                let mut old_values = Vec::new();
                for layer in min_layer..=max_layer {
                    let mut old_row = Vec::new();
                    for frame in min_frame..=max_frame {
                        old_row.push(ts.get_cell(layer, frame).copied());
                    }
                    old_values.push(old_row);
                }

                // 记录撤销
                self.undo_stack.push(UndoAction::SetRange {
                    min_layer,
                    min_frame,
                    old_values,
                });
                self.is_modified = true;

                // 清空
                for layer in min_layer..=max_layer {
                    for frame in min_frame..=max_frame {
                        ts.set_cell(layer, frame, None);
                    }
                }
            }
        }
    }

    /// 删除选区内容（不复制到剪贴板）
    fn delete_selection(&mut self) {
        // 清空选区内容并记录撤销
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            if let Some(ts) = &mut self.timesheet {
                // 保存旧值
                let mut old_values = Vec::new();
                for layer in min_layer..=max_layer {
                    let mut old_row = Vec::new();
                    for frame in min_frame..=max_frame {
                        old_row.push(ts.get_cell(layer, frame).copied());
                    }
                    old_values.push(old_row);
                }

                // 记录撤销
                self.undo_stack.push(UndoAction::SetRange {
                    min_layer,
                    min_frame,
                    old_values,
                });
                self.is_modified = true;

                // 清空
                for layer in min_layer..=max_layer {
                    for frame in min_frame..=max_frame {
                        ts.set_cell(layer, frame, None);
                    }
                }
            }
        } else if let Some((layer, frame)) = self.selected_cell {
            // 如果没有选区，删除当前选中的单元格
            let old_value = if let Some(ts) = &self.timesheet {
                ts.get_cell(layer, frame).copied()
            } else {
                None
            };

            // 记录撤销
            self.push_undo_set_cell(layer, frame, old_value, None);
            self.is_modified = true;

            // 清空单元格
            if let Some(ts) = &mut self.timesheet {
                ts.set_cell(layer, frame, None);
            }
        }
    }

    /// 从系统剪贴板粘贴
    fn paste_clipboard(&mut self) {
        if let Some((start_layer, start_frame)) = self.selected_cell {
            if let Some(ts) = &mut self.timesheet {
                // 使用内部剪贴板
                if !self.clipboard.is_empty() {
                    // 保存旧值用于撤销
                    let mut old_values = Vec::new();
                    for (layer_offset, row) in self.clipboard.iter().enumerate() {
                        let target_layer = start_layer + layer_offset;
                        let mut old_row = Vec::new();
                        for (frame_offset, _) in row.iter().enumerate() {
                            let target_frame = start_frame + frame_offset;
                            old_row.push(ts.get_cell(target_layer, target_frame).copied());
                        }
                        old_values.push(old_row);
                    }

                    // 记录撤销操作
                    self.undo_stack.push(UndoAction::SetRange {
                        min_layer: start_layer,
                        min_frame: start_frame,
                        old_values,
                    });
                    self.is_modified = true;

                    // 粘贴新值
                    for (layer_offset, row) in self.clipboard.iter().enumerate() {
                        let target_layer = start_layer + layer_offset;
                        for (frame_offset, cell) in row.iter().enumerate() {
                            let target_frame = start_frame + frame_offset;
                            ts.set_cell(target_layer, target_frame, *cell);
                        }
                    }
                }
            }
        }
    }

    /// 撤销操作
    fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            if let Some(ts) = &mut self.timesheet {
                match action {
                    UndoAction::SetCell { layer, frame, old_value, .. } => {
                        ts.set_cell(layer, frame, old_value);
                    }
                    UndoAction::SetRange { min_layer, min_frame, old_values } => {
                        for (layer_offset, row) in old_values.iter().enumerate() {
                            for (frame_offset, value) in row.iter().enumerate() {
                                ts.set_cell(
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
    }

    /// 记录单元格修改到撤销栈
    #[inline]
    fn push_undo_set_cell(&mut self, layer: usize, frame: usize, old_value: Option<CellValue>, new_value: Option<CellValue>) {
        // 限制撤销栈大小
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoAction::SetCell {
            layer,
            frame,
            old_value,
            new_value,
        });
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 设置窗口标题
        let title = if let Some(path) = &self.current_file_path {
            if let Some(ts) = &self.timesheet {
                if self.is_modified {
                    format!("{} - {}*", ts.name, path)
                } else {
                    format!("{} - {}", ts.name, path)
                }
            } else {
                "STS 3.0".to_string()
            }
        } else {
            "STS 3.0".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // 强制浅色
        ctx.set_visuals(egui::Visuals::light());

        // 快捷键
        let mut should_new = false;
        let mut should_open = false;
        let mut should_save = false;
        let mut should_copy = false;
        let mut should_cut = false;
        let mut should_paste = false;
        let mut should_undo = false;
        let mut should_delete = false;

        // 检测快捷键 - 检查事件队列
        ctx.input(|i| {
            // 文件操作快捷键
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                should_new = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                should_open = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                should_save = true;
            }

            // 剪贴板快捷键
            for event in &i.events {
                match event {
                    egui::Event::Copy => {
                        should_copy = true;
                    }
                    egui::Event::Cut => {
                        should_cut = true;
                    }
                    egui::Event::Paste(_) => {
                        should_paste = true;
                    }
                    _ => {}
                }
            }

            // Ctrl+Z 撤销
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                should_undo = true;
            }

            // Delete键删除选区
            if i.key_pressed(egui::Key::Delete) {
                should_delete = true;
            }
        });

        // 处理文件操作快捷键
        if should_new {
            self.show_new_dialog = true;
        }

        if should_open {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("STS Files", &["sts"])
                .pick_file()
            {
                match sts_rust::parse_sts_file(path.to_str().unwrap()) {
                    Ok(ts) => {
                        self.timesheet = Some(ts);
                        self.selected_cell = Some((0, 0));
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open: {}", e));
                    }
                }
            }
        }

        if should_save {
            if let Some(ts) = &self.timesheet {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("STS Files", &["sts"])
                    .save_file()
                {
                    match sts_rust::write_sts_file(ts, path.to_str().unwrap()) {
                        Ok(_) => self.error_message = None,
                        Err(e) => {
                            self.error_message = Some(format!("Failed to save: {}", e));
                        }
                    }
                }
            }
        }

        // 处理剪贴板和撤销快捷键
        if self.timesheet.is_some() {

            // 执行撤销
            if should_undo {
                self.undo();
                self.context_menu_pos = None; // 关闭菜单
            }

            // 只在非编辑模式下处理 CVX 和 Delete
            let is_editing = self.editing_cell.is_some() || self.editing_layer_name.is_some();

            // 执行删除操作
            if !is_editing && should_delete {
                self.delete_selection();
                self.context_menu_pos = None; // 关闭菜单
            }

            // 执行剪贴板操作
            if !is_editing && (should_copy || should_cut || should_paste) {
                // 关闭右键菜单
                self.context_menu_pos = None;

                if should_copy {
                    if self.selection_start.is_some() && self.selection_end.is_some() {
                        self.copy_selection(ctx);
                    } else if let Some((layer, frame)) = self.selected_cell {
                        self.selection_start = Some((layer, frame));
                        self.selection_end = Some((layer, frame));
                        self.copy_selection(ctx);
                    }
                } else if should_cut {
                    if self.selection_start.is_some() && self.selection_end.is_some() {
                        self.cut_selection(ctx);
                    } else if let Some((layer, frame)) = self.selected_cell {
                        self.selection_start = Some((layer, frame));
                        self.selection_end = Some((layer, frame));
                        self.cut_selection(ctx);
                        self.selection_start = None;
                        self.selection_end = None;
                    }
                } else if should_paste {
                    self.paste_clipboard();
                }
            }
        }

        // 新建对话框
        if self.show_new_dialog {
            // 绘制半透明的背景覆盖层（浅灰色）
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
                            200, // 半透明
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

                    // 秒 + 帧输入
                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.add(egui::DragValue::new(&mut self.new_seconds).range(0..=3600).suffix("s"));
                        ui.label("+");
                        ui.add(egui::DragValue::new(&mut self.new_frames).range(0..=self.new_framerate - 1).suffix("k"));
                    });

                    // 显示计算结果
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
                        self.create_new_timesheet();
                    }
                });
            return;
        }

        // 菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New (Ctrl+N)").clicked() {
                        self.show_new_dialog = true;
                        ui.close_menu();
                    }

                    if ui.button("Open... (Ctrl+O)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("STS Files", &["sts"])
                            .pick_file()
                        {
                            match sts_rust::parse_sts_file(path.to_str().unwrap()) {
                                Ok(ts) => {
                                    self.timesheet = Some(ts);
                                    self.selected_cell = Some((0, 0));
                                    self.error_message = None;
                                }
                                Err(e) => {
                                    self.error_message = Some(format!("Failed to open: {}", e));
                                }
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Save... (Ctrl+S)").clicked() {
                        if let Some(ts) = &self.timesheet {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("STS Files", &["sts"])
                                .save_file()
                            {
                                match sts_rust::write_sts_file(ts, path.to_str().unwrap()) {
                                    Ok(_) => self.error_message = None,
                                    Err(e) => {
                                        self.error_message = Some(format!("Failed to save: {}", e));
                                    }
                                }
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Import AE Keyframe...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text Files", &["txt"])
                            .pick_file()
                        {
                            match sts_rust::parse_ae_keyframe_file(path.to_str().unwrap()) {
                                Ok(_ts) => {
                                    self.error_message = Some("AE import not yet adapted to X-Sheet format".to_string());
                                }
                                Err(e) => {
                                    self.error_message = Some(format!("Failed to import: {}", e));
                                }
                            }
                        }
                        ui.close_menu();
                    }

                });
            });
        });

        // 主内容区域
        if self.timesheet.is_some() {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(msg) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, msg);
                }

                // 控制栏
                ui.horizontal(|ui| {
                    let ts = self.timesheet.as_ref().unwrap();
                    ui.label(&ts.name);

                    ui.separator();

                    // Total Frames
                    ui.label("Total Frames:");
                    let mut frames_buf = itoa::Buffer::new();
                    ui.label(frames_buf.format(ts.total_frames()));

                    ui.separator();
                });

                ui.separator();

                // 表格
                let row_height = 16.0;
                let col_width = 36.0;
                let page_col_width = 36.0;

                // 表头
                let layer_count = self.timesheet.as_ref().unwrap().layer_count;

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
                        let is_editing = self.editing_layer_name == Some(i);

                        // 背景
                        let bg_color = if is_editing {
                            egui::Color32::from_rgb(255, 255, 200)
                        } else {
                            egui::Color32::from_rgb(240, 240, 240)
                        };
                        ui.painter().rect_filled(rect, 0.0, bg_color);

                        // 边框
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::GRAY),
                        );

                        if is_editing {
                            let resp = ui.put(
                                rect,
                                egui::TextEdit::singleline(&mut self.editing_layer_text)
                                    .desired_width(col_width)
                                    .horizontal_align(egui::Align::Center)
                                    .frame(false),
                            );
                            resp.request_focus();

                            if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if let Some(ts) = &mut self.timesheet {
                                    ts.layer_names[i] = self.editing_layer_text.clone();
                                    self.is_modified = true;
                                }
                                self.editing_layer_name = None;
                            }

                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                self.editing_layer_name = None;
                            }
                        } else {
                            // 显示模式 - 居中显示
                            let resp = ui.interact(rect, id, egui::Sense::click());

                            let layer_name = &self.timesheet.as_ref().unwrap().layer_names[i];
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                layer_name,
                                egui::FontId::proportional(11.0),
                                egui::Color32::BLACK,
                            );

                            if resp.clicked() {
                                self.editing_layer_name = Some(i);
                                self.editing_layer_text = layer_name.clone();
                            }
                        }
                    }
                });

                ui.separator();

                // 数据区域
                let total_frames = {
                    let ts_mut = self.timesheet.as_mut().unwrap();
                    let total = ts_mut.total_frames().max(1);
                    ts_mut.ensure_frames(total);
                    total
                };

                ui.spacing_mut().item_spacing.y = 0.0;

                let mut page_buf = itoa::Buffer::new();
                let mut frame_buf = itoa::Buffer::new();
                
                let (pointer_pos, pointer_down) = ui.input(|i| {
                    (i.pointer.interact_pos(), i.pointer.primary_down())
                });

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show_rows(ui, row_height, total_frames, |ui, row_range| {
                        for frame_idx in row_range {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                let (page, frame_in_page) = self.timesheet.as_ref().unwrap().get_page_and_frame(frame_idx);
                                let page_str = page_buf.format(page);
                                let frame_str = frame_buf.format(frame_in_page);

                                let (_page_id, page_rect) = ui.allocate_space(egui::vec2(page_col_width, row_height));

                                // 边框
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

                                // 单元格
                                for layer_idx in 0..layer_count {
                                    let is_selected = self.selected_cell == Some((layer_idx, frame_idx));
                                    let is_editing = self.editing_cell == Some((layer_idx, frame_idx));

                                    let (cell_id, cell_rect) = ui.allocate_space(egui::vec2(col_width, row_height));
                                    let cell_response = ui.interact(
                                        cell_rect,
                                        cell_id,
                                        egui::Sense::click().union(egui::Sense::drag()),
                                    );

                                    if (is_selected || is_editing) && self.auto_scroll_to_selection {
                                        cell_response.scroll_to_me(None);
                                        self.auto_scroll_to_selection = false;
                                    }

                                    let is_in_selection = self.is_cell_in_selection(layer_idx, frame_idx);

                                    let bg_color = if is_editing { BG_EDITING }
                                        else if is_selected { BG_SELECTED }
                                        else if is_in_selection { BG_IN_SELECTION }
                                        else { BG_NORMAL };

                                    ui.painter().rect_filled(cell_rect, 0.0, bg_color);

                                    // 边框
                                    let border_color = if is_in_selection { BORDER_SELECTION } else { BORDER_NORMAL };
                                    ui.painter().rect_stroke(cell_rect, 0.0, egui::Stroke::new(1.0, border_color));

                                    // 内容
                                    if is_editing {
                                        // 编辑模式
                                        let text_response = ui.put(
                                            cell_rect,
                                            egui::TextEdit::singleline(&mut self.editing_text)
                                                .desired_width(col_width)
                                                .horizontal_align(egui::Align::Center)
                                                .frame(false),
                                        );

                                        text_response.request_focus();

                                        if text_response.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                                            self.finish_edit(false, true);
                                        }
                                    } else {
                                        let ts_ref = self.timesheet.as_ref().unwrap();
                                        if let Some(current_val) = ts_ref.get_cell(layer_idx, frame_idx) {
                                            let should_show_dash = frame_idx > 0 &&
                                                ts_ref.get_cell(layer_idx, frame_idx - 1)
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

                                    // 鼠标交互
                                    if cell_response.secondary_clicked() {
                                        // 右键菜单
                                        self.context_menu_pos = Some((layer_idx, frame_idx));

                                        // 保存当前选区状态
                                        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                                            self.context_menu_selection = Some((start, end));
                                        } else {
                                            self.context_menu_selection = None;
                                        }

                                        // 右键时不清除选区
                                        if !self.is_cell_in_selection(layer_idx, frame_idx) {
                                            self.selected_cell = Some((layer_idx, frame_idx));
                                        }

                                        // 保存右键点击时的屏幕位置
                                        if let Some(pos) = cell_response.interact_pointer_pos() {
                                            self.context_menu_screen_pos = pos;
                                        }
                                    } else {
                                        // 左键拖拽选择
                                        if let Some(pos) = pointer_pos {
                                            if pointer_down && cell_rect.contains(pos) {
                                                if !self.is_dragging {
                                                    // 初始化拖拽
                                                    self.is_dragging = true;
                                                    self.selection_start = Some((layer_idx, frame_idx));
                                                    self.selection_end = Some((layer_idx, frame_idx));
                                                    self.selected_cell = Some((layer_idx, frame_idx));
                                                    // 退出编辑模式
                                                    if self.editing_cell.is_some() {
                                                        self.editing_cell = None;
                                                        self.editing_text.clear();
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // 拖拽中：检查指针是否在当前格子内
                                    if self.is_dragging && pointer_down {
                                        if let Some(pos) = pointer_pos {
                                            if cell_rect.contains(pos) {
                                                if self.selection_end != Some((layer_idx, frame_idx)) {
                                                    self.selection_end = Some((layer_idx, frame_idx));
                                                    self.selected_cell = Some((layer_idx, frame_idx));
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    });
            });

            // 鼠标释放
            ctx.input(|i| {
                if !i.pointer.primary_down() && self.is_dragging {
                    self.is_dragging = false;
                }
            });

            // 右键菜单
            if let Some((_menu_layer, _menu_frame)) = self.context_menu_pos {
                // 固定在右键点击的位置
                let menu_result = egui::Area::new(egui::Id::new("context_menu"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(self.context_menu_screen_pos)
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

                // 处理按钮点击
                let any_button_clicked = copy_clicked || cut_clicked || paste_clicked || undo_clicked;

                if copy_clicked {
                    // 使用右键时保存的选区状态
                    if let Some((start, end)) = self.context_menu_selection {
                        // 有选区，复制选区
                        self.clipboard.clear();
                        if let Some(ts) = &self.timesheet {
                            let min_layer = start.0.min(end.0);
                            let max_layer = start.0.max(end.0);
                            let min_frame = start.1.min(end.1);
                            let max_frame = start.1.max(end.1);

                            let mut text_rows = Vec::new();
                            for layer in min_layer..=max_layer {
                                let mut row = Vec::new();
                                let mut text_cols = Vec::new();

                                for frame in min_frame..=max_frame {
                                    let cell = ts.get_cell(layer, frame).copied();
                                    row.push(cell);

                                    let text = match cell {
                                        Some(CellValue::Number(n)) => n.to_string(),
                                        Some(CellValue::Same) => "-".to_string(),
                                        None => "".to_string(),
                                    };
                                    text_cols.push(text);
                                }
                                self.clipboard.push(row);
                                text_rows.push(text_cols.join("\t"));
                            }

                            // 复制到系统剪贴板
                            let clipboard_text = text_rows.join("\n");
                            ctx.output_mut(|o| o.copied_text = clipboard_text);
                        }
                    } else {
                        // 没有选区，从右键位置复制单个单元格
                        if let Some((layer, frame)) = self.context_menu_pos {
                            self.clipboard.clear();
                            if let Some(ts) = &self.timesheet {
                                let cell = ts.get_cell(layer, frame).copied();
                                self.clipboard.push(vec![cell]);

                                // 复制到系统剪贴板
                                let text = match cell {
                                    Some(CellValue::Number(n)) => n.to_string(),
                                    Some(CellValue::Same) => "-".to_string(),
                                    None => "".to_string(),
                                };
                                ctx.output_mut(|o| o.copied_text = text);
                            }
                        }
                    }
                    self.context_menu_pos = None;
                } else if cut_clicked {
                    // 使用右键时保存的选区状态
                    if let Some((start, end)) = self.context_menu_selection {
                        // 临时设置选区用于剪切
                        self.selection_start = Some(start);
                        self.selection_end = Some(end);
                        self.cut_selection(ctx);
                        self.selection_start = None;
                        self.selection_end = None;
                    } else if let Some((layer, frame)) = self.context_menu_pos {
                        // 没有选区，剪切单个单元格
                        self.selection_start = Some((layer, frame));
                        self.selection_end = Some((layer, frame));
                        self.cut_selection(ctx);
                        self.selection_start = None;
                        self.selection_end = None;
                    }
                    self.context_menu_pos = None;
                } else if paste_clicked {
                    // 粘贴到右键点击的位置
                    if let Some((layer, frame)) = self.context_menu_pos {
                        self.selected_cell = Some((layer, frame));
                    }
                    self.paste_clipboard();
                    self.context_menu_pos = None;
                } else if undo_clicked {
                    self.undo();
                    self.context_menu_pos = None;
                }

                // 只在没有按钮被点击时才检查菜单外部点击
                if !any_button_clicked {
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
                        self.context_menu_pos = None;
                    }
                }

                // ESC 键关闭菜单
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.context_menu_pos = None;
                }
            }

            let layer_count = self.timesheet.as_ref().unwrap().layer_count;

            if let Some((layer, frame)) = self.editing_cell {
                let current_text = self.editing_text.clone();
                let has_input = !current_text.is_empty();

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Enter) {
                        self.finish_edit(true, true);
                        self.auto_scroll_to_selection = true;
                    } else if i.key_pressed(egui::Key::Escape) {
                        self.editing_cell = None;
                        self.editing_text.clear();
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
                                // 保存当前单元格并记录撤销
                                self.finish_edit(false, true);
                                // 移动到新单元格并开始编辑
                                self.start_edit(pos.0, pos.1);
                            } else {
                                self.editing_cell = None;
                                self.editing_text.clear();
                            }
                            self.selected_cell = Some(pos);
                            self.auto_scroll_to_selection = true;
                        }
                    }
                });
            } else if let Some((layer, frame)) = self.selected_cell {
                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Enter) {
                        // 先收集信息
                        let (old_value, new_value) = if let Some(ts) = &self.timesheet {
                            if frame > 0 {
                                let old = ts.get_cell(layer, frame).copied();
                                let new = ts.get_cell(layer, frame - 1).copied();
                                (old, new)
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };

                        // 记录撤销并设置值
                        if old_value != new_value && new_value.is_some() {
                            self.push_undo_set_cell(layer, frame, old_value, new_value);
                            self.is_modified = true;
                            if let Some(ts) = &mut self.timesheet {
                                ts.set_cell(layer, frame, new_value);
                            }
                        }

                        self.selected_cell = Some((layer, frame + 1));
                        self.auto_scroll_to_selection = true;
                    } else if i.key_pressed(egui::Key::Tab) && layer < layer_count - 1 {
                        self.selected_cell = Some((layer + 1, frame));
                        self.auto_scroll_to_selection = true;
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
                            self.selected_cell = Some(pos);
                            self.auto_scroll_to_selection = true;
                        } else {
                            // 检查文本输入
                            for event in &i.events {
                                if let egui::Event::Text(text) = event {
                                    if text.chars().all(|c| c.is_ascii_digit()) {
                                        self.start_edit(layer, frame);
                                        self.editing_text = text.clone();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("No timesheet open");
                    if ui.button("New Timesheet").clicked() {
                        self.show_new_dialog = true;
                    }
                });
            });
        }

        // 确认保存对话框 - 放在最后渲染，确保叠在其他UI之上
        if self.show_confirm_dialog {
            let mut should_close = false;
            let mut action_choice: Option<bool> = None;

            egui::Window::new("Save Changes?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Do you want to save before continuing?");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.add_sized([80.0, 25.0], egui::Button::new("Save")).clicked() {
                            action_choice = Some(true);
                            should_close = true;
                        }

                        if ui.add_sized(
                            [90.0, 25.0],
                            egui::Button::new(
                                egui::RichText::new("Don't Save").color(egui::Color32::RED)
                            )
                        ).clicked() {
                            action_choice = Some(false);
                            should_close = true;
                        }

                        if ui.add_sized([80.0, 25.0], egui::Button::new("Cancel")).clicked() {
                            should_close = true;
                        }
                    });
                });

            if should_close {
                self.show_confirm_dialog = false;
                match action_choice {
                    Some(true) => {
                        self.save_file();
                        if let Some(action) = self.pending_action {
                            match action {
                                PendingAction::New => self.show_new_dialog = true,
                                PendingAction::Open => self.open_file(),
                            }
                        }
                        self.pending_action = None;
                    }
                    Some(false) => {
                        if let Some(action) = self.pending_action {
                            match action {
                                PendingAction::New => self.show_new_dialog = true,
                                PendingAction::Open => self.open_file(),
                            }
                        }
                        self.pending_action = None;
                    }
                    None => {
                        self.pending_action = None;
                    }
                }
            }
        }
    }
}
