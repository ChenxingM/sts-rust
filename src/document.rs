//! Document module - handles individual document state and operations

use eframe::egui;
use std::collections::VecDeque;
use std::rc::Rc;
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

// 撤销栈限制
pub const MAX_UNDO_ACTIONS: usize = 100;

// 撤销操作类型
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
        old_values: Rc<Vec<Vec<Option<CellValue>>>>,
    },
}

// 编辑状态
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

// Repeat 弹窗状态
pub struct RepeatDialogState {
    pub open: bool,
    pub layer: usize,
    pub start_frame: usize,
    pub end_frame: usize,
    pub repeat_count: u32,
    pub repeat_until_end: bool,
}

impl Default for RepeatDialogState {
    fn default() -> Self {
        Self {
            open: false,
            layer: 0,
            start_frame: 0,
            end_frame: 0,
            repeat_count: 1,
            repeat_until_end: false,
        }
    }
}

// 剪贴板数据
pub type ClipboardData = Rc<Vec<Vec<Option<CellValue>>>>;

// 文档结构
pub struct Document {
    pub id: usize,
    pub timesheet: Box<TimeSheet>,
    pub file_path: Option<Box<str>>,
    pub is_modified: bool,
    pub is_open: bool,
    pub edit_state: EditState,
    pub selection_state: SelectionState,
    pub context_menu: ContextMenuState,
    pub clipboard: Option<ClipboardData>,
    pub undo_stack: VecDeque<UndoAction>,
    pub repeat_dialog: RepeatDialogState,
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
            undo_stack: VecDeque::with_capacity(MAX_UNDO_ACTIONS),
            repeat_dialog: RepeatDialogState::default(),
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

    /// Auto-save if file path exists. Saves silently (no error returned).
    /// Sets is_modified to false after successful save.
    pub fn auto_save(&mut self) {
        if self.file_path.is_some() {
            let _ = self.save();
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
                let total_frames = self.timesheet.total_frames();
                if frame + 1 < total_frames {
                    self.selection_state.selected_cell = Some((layer, frame + 1));
                }
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

            self.undo_stack.push_back(UndoAction::SetRange {
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

            self.undo_stack.push_back(UndoAction::SetRange {
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

                self.undo_stack.push_back(UndoAction::SetRange {
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
        if let Some(action) = self.undo_stack.pop_back() {
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
    pub fn push_undo_set_cell(&mut self, layer: usize, frame: usize, old_value: Option<CellValue>) {
        // 限制撤销栈大小
        if self.undo_stack.len() >= MAX_UNDO_ACTIONS {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(UndoAction::SetCell {
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

    /// 检查选择是否为单列，返回 (layer, min_frame, max_frame) 或错误信息
    pub fn check_single_column_selection(&self) -> Result<(usize, usize, usize), &'static str> {
        if let Some((min_layer, min_frame, max_layer, max_frame)) = self.get_selection_range() {
            if min_layer != max_layer {
                return Err("Only single column selection is supported");
            }
            Ok((min_layer, min_frame, max_frame))
        } else {
            Err("No selection")
        }
    }

    /// 执行重复操作
    pub fn repeat_selection(&mut self, repeat_count: u32, repeat_until_end: bool) -> Result<(), &'static str> {
        let (layer, start_frame, end_frame) = self.check_single_column_selection()?;

        // 获取选择范围的值
        let selection_len = end_frame - start_frame + 1;
        let mut source_values: Vec<Option<CellValue>> = Vec::with_capacity(selection_len);
        for frame in start_frame..=end_frame {
            source_values.push(self.timesheet.get_cell(layer, frame).copied());
        }

        let total_frames = self.timesheet.total_frames();
        let insert_start = end_frame + 1;

        // 计算可用的帧数
        let available_frames = total_frames.saturating_sub(insert_start);
        if available_frames == 0 {
            return Err("No frames available to repeat into");
        }

        // 计算需要写入的总帧数
        let total_write_frames = if repeat_until_end {
            // 填满所有剩余帧（包括不完整的组）
            available_frames
        } else {
            // 尝试写入 repeat_count 组，但不超过可用帧数
            let requested_frames = selection_len * repeat_count as usize;
            requested_frames.min(available_frames)
        };

        let write_end = insert_start + total_write_frames;

        // 保存旧值用于撤销
        let mut old_values = Vec::new();
        let mut old_row = Vec::with_capacity(total_write_frames);
        for frame in insert_start..write_end {
            old_row.push(self.timesheet.get_cell(layer, frame).copied());
        }
        old_values.push(old_row);

        self.undo_stack.push_back(UndoAction::SetRange {
            min_layer: layer,
            min_frame: insert_start,
            old_values: Rc::new(old_values),
        });
        self.is_modified = true;

        // 写入重复的值（循环写入source_values直到填满）
        let mut write_frame = insert_start;
        while write_frame < write_end {
            for value in &source_values {
                if write_frame >= write_end {
                    break;
                }
                self.timesheet.set_cell(layer, write_frame, *value);
                write_frame += 1;
            }
        }

        Ok(())
    }

    /// 执行反向操作
    /// 反向时跳过与最后一帧相同值的所有帧，例如 111222333 -> 111222333222111
    pub fn reverse_selection(&mut self) -> Result<(), &'static str> {
        let (layer, start_frame, end_frame) = self.check_single_column_selection()?;

        let selection_len = end_frame - start_frame + 1;
        if selection_len < 2 {
            return Err("Selection must have at least 2 frames");
        }

        // 获取最后一帧的值
        let last_value = self.timesheet.get_cell(layer, end_frame).copied();

        // 从 end_frame 向前找到第一个不同值的帧
        let mut actual_end = end_frame;
        while actual_end > start_frame {
            let current_value = self.timesheet.get_cell(layer, actual_end - 1).copied();
            if current_value != last_value {
                break;
            }
            actual_end -= 1;
        }

        // 如果所有帧都是相同值，无法反向
        if actual_end <= start_frame {
            return Err("All frames have the same value, cannot reverse");
        }

        // 收集反向值（从 actual_end - 1 到 start_frame）
        let reverse_len = actual_end - start_frame;
        let mut reverse_values: Vec<Option<CellValue>> = Vec::with_capacity(reverse_len);
        for frame in (start_frame..actual_end).rev() {
            reverse_values.push(self.timesheet.get_cell(layer, frame).copied());
        }

        let total_frames = self.timesheet.total_frames();
        let insert_start = end_frame + 1;
        let write_end = insert_start + reverse_len;

        // 检查是否超出范围
        if write_end > total_frames {
            return Err("Not enough frames to reverse");
        }

        // 保存旧值用于撤销
        let mut old_values = Vec::new();
        let mut old_row = Vec::with_capacity(reverse_len);
        for frame in insert_start..write_end {
            old_row.push(self.timesheet.get_cell(layer, frame).copied());
        }
        old_values.push(old_row);

        self.undo_stack.push_back(UndoAction::SetRange {
            min_layer: layer,
            min_frame: insert_start,
            old_values: Rc::new(old_values),
        });
        self.is_modified = true;

        // 写入反向值
        for (i, value) in reverse_values.iter().enumerate() {
            self.timesheet.set_cell(layer, insert_start + i, *value);
        }

        Ok(())
    }
}
