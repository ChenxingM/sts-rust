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
const EMPTY: &str = "";

struct StsApp {
    timesheet: Option<TimeSheet>,
    show_new_dialog: bool,
    new_name: String,
    new_framerate: u32,
    new_layer_count: usize,
    new_frames_per_page: u32,
    new_total_frames: usize,
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
}

impl Default for StsApp {
    fn default() -> Self {
        Self {
            timesheet: None,
            show_new_dialog: true,
            new_name: "sheet1".to_string(),
            new_framerate: 24,
            new_layer_count: 12,
            new_frames_per_page: 144,
            new_total_frames: 100,
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
        }
    }
}

impl StsApp {
    /// 创建新摄影表
    #[inline]
    fn create_new_timesheet(&mut self) {
        let mut ts = TimeSheet::new(
            self.new_name.clone(),
            self.new_framerate,
            self.new_layer_count,
            self.new_frames_per_page,
        );
        ts.ensure_frames(self.new_total_frames);
        self.timesheet = Some(ts);
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

    /// 完成编辑
    #[inline]
    fn finish_edit(&mut self, move_down: bool) {
        if let Some((layer, frame)) = self.editing_cell {
            if let Some(ts) = &mut self.timesheet {
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

                ts.set_cell(layer, frame, value);

                if move_down {
                    self.selected_cell = Some((layer, frame + 1));
                }
            }

            self.editing_cell = None;
            self.editing_text.clear();
        }
    }

    /// 检查单元格是否在选区内
    #[inline]
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

    /// 复制选区到剪贴板
    fn copy_selection(&mut self) {
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            if let Some(ts) = &self.timesheet {
                self.clipboard.clear();
                for layer in min_layer..=max_layer {
                    let mut row = Vec::new();
                    for frame in min_frame..=max_frame {
                        row.push(ts.get_cell(layer, frame).copied());
                    }
                    self.clipboard.push(row);
                }
            }
        }
    }

    /// 剪切选区到剪贴板
    fn cut_selection(&mut self) {
        self.copy_selection();
        // 清空选区内容
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            if let Some(ts) = &mut self.timesheet {
                for layer in min_layer..=max_layer {
                    for frame in min_frame..=max_frame {
                        ts.set_cell(layer, frame, None);
                    }
                }
            }
        }
    }

    /// 从当前选中单元格粘贴
    fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }

        if let Some((start_layer, start_frame)) = self.selected_cell {
            if let Some(ts) = &mut self.timesheet {
                for (layer_offset, row) in self.clipboard.iter().enumerate() {
                    let target_layer = start_layer + layer_offset;
                    for (frame_offset, cell) in row.iter().enumerate() {
                        let target_frame = start_frame + frame_offset;
                        ts.set_cell(target_layer, target_frame, cell.clone());
                    }
                }
            }
        }
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 强制浅色
        ctx.set_visuals(egui::Visuals::light());

        // 快捷键
        if self.timesheet.is_some() {
            let mut should_copy = false;
            let mut should_cut = false;
            let mut should_paste = false;

            // 剪贴板事件检测
            ctx.input(|i| {
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
            });

            // 执行操作
            if should_copy || should_cut || should_paste {

                // 退出编辑
                if self.editing_cell.is_some() {
                    self.editing_cell = None;
                    self.editing_text.clear();
                }
                if self.editing_layer_name.is_some() {
                    self.editing_layer_name = None;
                }

                if should_copy {
                    if self.selection_start.is_some() && self.selection_end.is_some() {
                        self.copy_selection();
                    } else if let Some((layer, frame)) = self.selected_cell {
                        self.selection_start = Some((layer, frame));
                        self.selection_end = Some((layer, frame));
                        self.copy_selection();
                    }
                } else if should_cut {
                    if self.selection_start.is_some() && self.selection_end.is_some() {
                        self.cut_selection();
                    } else if let Some((layer, frame)) = self.selected_cell {
                        self.selection_start = Some((layer, frame));
                        self.selection_end = Some((layer, frame));
                        self.cut_selection();
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
                    ui.horizontal(|ui| {
                        ui.label("Total Frames:");
                        ui.add(egui::DragValue::new(&mut self.new_total_frames).range(1..=10000));
                    });
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
                    if ui.button("New").clicked() {
                        self.show_new_dialog = true;
                        ui.close_menu();
                    }

                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("STS Files", &["sts", "json"])
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

                    if ui.button("Save...").clicked() {
                        if let Some(ts) = &self.timesheet {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("STS Files", &["sts", "json"])
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
                    ui.label(format!("FPS: {}", ts.framerate));

                    ui.separator();

                    ui.label("Total Frames:");
                    let mut temp_frames = ts.total_frames();
                    if ui.add(egui::DragValue::new(&mut temp_frames).range(1..=10000)).changed() {
                        self.timesheet.as_mut().unwrap().ensure_frames(temp_frames);
                    }
                });

                ui.separator();

                // 表格
                let row_height = 16.0;
                let col_width = 32.0;
                let page_col_width = 32.0;

                // 表头
                let (layer_count, layer_names) = {
                    let ts = self.timesheet.as_ref().unwrap();
                    (ts.layer_count, ts.layer_names.clone())
                };

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
                                }
                                self.editing_layer_name = None;
                            }

                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                self.editing_layer_name = None;
                            }
                        } else {
                            // 显示模式 - 居中显示
                            let resp = ui.interact(rect, id, egui::Sense::click());

                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &layer_names[i],
                                egui::FontId::proportional(11.0),
                                egui::Color32::BLACK,
                            );

                            if resp.clicked() {
                                self.editing_layer_name = Some(i);
                                self.editing_layer_text = layer_names[i].clone();
                            }
                        }
                    }
                });

                ui.separator();

                // 数据区域
                let total_frames = {
                    let ts_mut = self.timesheet.as_mut().unwrap();
                    let total = ts_mut.total_frames().max(100);
                    ts_mut.ensure_frames(total);
                    total
                };

                ui.spacing_mut().item_spacing.y = -0.1;

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
                                const BG_EDITING: egui::Color32 = egui::Color32::from_rgb(255, 255, 200);
                                const BG_SELECTED: egui::Color32 = egui::Color32::from_rgb(200, 220, 255);
                                const BG_IN_SELECTION: egui::Color32 = egui::Color32::from_rgb(220, 235, 255);
                                const BG_NORMAL: egui::Color32 = egui::Color32::WHITE;
                                const BORDER_SELECTION: egui::Color32 = egui::Color32::from_rgb(100, 150, 255);
                                const BORDER_NORMAL: egui::Color32 = egui::Color32::GRAY;

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
                                            self.finish_edit(false);
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
                                    // 检测鼠标按下
                                    if cell_response.is_pointer_button_down_on() {
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

            let layer_count = self.timesheet.as_ref().unwrap().layer_count;

            if let Some((layer, frame)) = self.editing_cell {
                let current_text = self.editing_text.clone();
                let has_input = !current_text.is_empty();

                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Enter) {
                        self.finish_edit(true);
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
                                self.editing_cell = Some(pos);
                            } else {
                                self.editing_cell = None;
                                self.editing_text.clear();
                            }
                            self.selected_cell = Some(pos);
                            self.auto_scroll_to_selection = true;
                        }
                    }
                });

                if has_input && self.editing_cell.is_some() && self.editing_cell != Some((layer, frame)) {
                    self.editing_text = current_text;
                }
            } else if let Some((layer, frame)) = self.selected_cell {
                ctx.input(|i| {
                    if i.key_pressed(egui::Key::Enter) {
                        if let Some(ts) = &mut self.timesheet {
                            if frame > 0 {
                                if let Some(prev_val) = ts.get_cell(layer, frame - 1).copied() {
                                    ts.set_cell(layer, frame, Some(prev_val));
                                }
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
    }
}
