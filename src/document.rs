//! Document module - handles individual document state and operations

use eframe::egui;
use std::rc::Rc;
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

// 撤销栈限制
pub const MAX_UNDO_ACTIONS: usize = 100;

// 撤销操作类型 - 使用 Rc 共享数据以减少内存使用
#[derive(Clone)]
pub enum UndoAction {
    SetCell {
        layer: usize,
        frame: usize,
        old_value: Option<CellValue>,
    },
    SetRange {
        min_layer: usize,
        min_frame: usize,
        // 使用 Rc 共享不变数据，避免深拷贝
        old_values: Rc<Vec<Vec<Option<CellValue>>>>,
    },
}

// 编辑状态 - 使用更紧凑的字符串存储
pub struct EditState {
    pub editing_cell: Option<(usize, usize)>,
    pub editing_layer_name: Option<usize>,
    // 使用 String 但初始容量更小
    pub editing_text: String,
    pub editing_layer_text: String,
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
pub struct SelectionState {
    pub selected_cell: Option<(usize, usize)>,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub is_dragging: bool,
    pub auto_scroll_to_selection: bool,
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
pub struct ContextMenuState {
    pub pos: Option<(usize, usize)>,
    pub screen_pos: egui::Pos2,
    pub selection: Option<((usize, usize), (usize, usize))>,
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

// 剪贴板数据 - 使用 Rc 共享以避免大量复制
pub type ClipboardData = Rc<Vec<Vec<Option<CellValue>>>>;

// 文档结构 - 每个打开的文件对应一个Document
// 使用 Box 优化大型字段的内存布局
pub struct Document {
    pub id: usize,
    // TimeSheet 可能很大，使用 Box 减少栈上大小
    pub timesheet: Box<TimeSheet>,
    pub file_path: Option<Box<str>>,
    pub is_modified: bool,
    pub is_open: bool,
    pub edit_state: EditState,
    pub selection_state: SelectionState,
    pub context_menu: ContextMenuState,
    pub clipboard: Option<ClipboardData>,
    // 使用 Box 减少 Vec 的栈大小
    pub undo_stack: Box<Vec<UndoAction>>,
}

impl Document {
    pub fn new(id: usize, timesheet: TimeSheet, file_path: Option<String>) -> Self {
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

    pub fn title(&self) -> String {
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

    pub fn save(&mut self) -> Result<(), String> {
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

    pub fn save_as(&mut self, path: String) -> Result<(), String> {
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
    pub fn start_edit(&mut self, layer: usize, frame: usize) {
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
    pub fn finish_edit(&mut self, move_down: bool, record_undo: bool) {
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
    pub fn is_cell_in_selection(&self, layer: usize, frame: usize) -> bool {
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
    pub fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
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
    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        let range = self.get_selection_range();

        if let Some((min_layer, min_frame, max_layer, max_frame)) = range {
            let row_count = max_layer - min_layer + 1;
            let col_count = max_frame - min_frame + 1;

            // 预分配容量以减少内存重新分配
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
                // 使用 Rc 共享数据，避免克隆
                self.clipboard = Some(Rc::new(clipboard_data));
                ctx.output_mut(|o| o.copied_text = clipboard_text);
            }
        }
    }

    pub fn cut_selection(&mut self, ctx: &egui::Context) {
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

    pub fn delete_selection(&mut self) {
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

    pub fn paste_clipboard(&mut self) {
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

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action {
                UndoAction::SetCell { layer, frame, old_value } => {
                    self.timesheet.set_cell(layer, frame, old_value);
                }
                UndoAction::SetRange { min_layer, min_frame, old_values } => {
                    // Rc 解引用不会复制数据
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
    pub fn push_undo_set_cell(&mut self, layer: usize, frame: usize, old_value: Option<CellValue>) {
        // 限制撤销栈大小
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
    pub fn estimate_undo_memory(&self) -> usize {
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
