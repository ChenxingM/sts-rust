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
    InsertLayer {
        index: usize,
    },
    DeleteLayer {
        index: usize,
        name: String,
        cells: Vec<Option<CellValue>>,
    },
}

// 编辑状态
pub struct EditState {
    pub editing_cell: Option<(usize, usize)>,
    pub editing_layer_name: Option<usize>,
    // 使用 String 但初始容量更小
    pub editing_text: String,
    pub editing_layer_text: String,
    // 批量编辑时保存的选区范围 (min_layer, min_frame, max_layer, max_frame)
    pub batch_edit_range: Option<(usize, usize, usize, usize)>,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            editing_cell: None,
            editing_layer_name: None,
            // 初始不分配内存
            editing_text: String::new(),
            editing_layer_text: String::new(),
            batch_edit_range: None,
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

// 序列填充弹窗状态
pub struct SequenceFillDialogState {
    pub open: bool,
    pub layer: usize,
    pub start_frame: usize,
    pub start_value: u32,
    pub end_value: u32,
    pub hold_frames: u32,  // 拍数（每个数字重复多少帧）
}

impl Default for SequenceFillDialogState {
    fn default() -> Self {
        Self {
            open: false,
            layer: 0,
            start_frame: 0,
            start_value: 1,
            end_value: 24,
            hold_frames: 1,
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
    pub sequence_fill_dialog: SequenceFillDialogState,
    pub jump_step: usize,  // Enter key jump step (adjustable with / and *)
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
            sequence_fill_dialog: SequenceFillDialogState::default(),
            jump_step: 1,
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
        self.edit_state.batch_edit_range = None;

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

    /// 开始批量编辑 - 保存当前选区范围，完成编辑时会填充所有选中的单元格
    #[inline]
    pub fn start_batch_edit(&mut self, layer: usize, frame: usize) {
        // 保存当前选区范围
        self.edit_state.batch_edit_range = self.get_selection_range();

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
            // 解析输入值
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

            // 检查是否有批量编辑范围
            if let Some((min_layer, min_frame, max_layer, max_frame)) = self.edit_state.batch_edit_range {
                // 批量填充所有选中的单元格
                if record_undo {
                    // 保存旧值用于撤销
                    let mut old_values = Vec::new();
                    for l in min_layer..=max_layer {
                        let mut old_row = Vec::new();
                        for f in min_frame..=max_frame {
                            old_row.push(self.timesheet.get_cell(l, f).copied());
                        }
                        old_values.push(old_row);
                    }
                    self.undo_stack.push_back(UndoAction::SetRange {
                        min_layer,
                        min_frame,
                        old_values: Rc::new(old_values),
                    });
                    self.is_modified = true;
                }

                // 填充所有选中的单元格
                for l in min_layer..=max_layer {
                    for f in min_frame..=max_frame {
                        self.timesheet.set_cell(l, f, value);
                    }
                }

                // 清除选区
                self.selection_state.selection_start = None;
                self.selection_state.selection_end = None;
            } else {
                // 单个单元格编辑（原有逻辑）
                let old_value = self.timesheet.get_cell(layer, frame).copied();

                if record_undo && old_value != value {
                    self.push_undo_set_cell(layer, frame, old_value);
                    self.is_modified = true;
                }

                self.timesheet.set_cell(layer, frame, value);

                if move_down {
                    let total_frames = self.timesheet.total_frames();
                    let new_frame = frame + self.jump_step;

                    // Fill skipped cells with Same marker (continuing the value)
                    if self.jump_step > 1 && value.is_some() {
                        for skip_frame in (frame + 1)..new_frame.min(total_frames) {
                            let old_skip_value = self.timesheet.get_cell(layer, skip_frame).copied();
                            if record_undo && old_skip_value != Some(CellValue::Same) {
                                self.push_undo_set_cell(layer, skip_frame, old_skip_value);
                            }
                            self.timesheet.set_cell(layer, skip_frame, Some(CellValue::Same));
                        }
                    }

                    if new_frame < total_frames {
                        self.selection_state.selected_cell = Some((layer, new_frame));
                    } else if total_frames > 0 {
                        self.selection_state.selected_cell = Some((layer, total_frames - 1));
                    }
                }
            }

            self.edit_state.editing_cell = None;
            self.edit_state.editing_text.clear();
            self.edit_state.batch_edit_range = None;
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

    /// 从文本解析剪贴板数据（tab分隔格式）
    pub fn parse_clipboard_text(text: &str) -> Option<ClipboardData> {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let mut data = Vec::new();
        for line in lines {
            let row: Vec<Option<CellValue>> = line
                .split('\t')
                .map(|s| {
                    let s = s.trim();
                    if s.is_empty() {
                        None
                    } else if s == "-" {
                        Some(CellValue::Same)
                    } else {
                        s.parse::<u32>().ok().map(CellValue::Number)
                    }
                })
                .collect();
            data.push(row);
        }
        Some(Rc::new(data))
    }

    /// 从系统剪贴板文本粘贴，返回是否成功
    pub fn paste_from_text(&mut self, text: &str) -> bool {
        if let Some(clipboard) = Self::parse_clipboard_text(text) {
            self.clipboard = Some(clipboard);
            self.paste_clipboard();
            true
        } else {
            false
        }
    }

    /// 在指定位置插入一列
    pub fn insert_layer(&mut self, index: usize) {
        self.timesheet.insert_layer(index);
        // 限制撤销栈大小
        if self.undo_stack.len() >= MAX_UNDO_ACTIONS {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(UndoAction::InsertLayer { index });
        self.is_modified = true;

        // 调整可能受列插入影响的状态索引
        self.adjust_selection_for_insert(index);
        self.adjust_editing_for_insert(index);
        self.adjust_context_menu_for_insert(index);
    }

    /// 调整选择状态的索引（列插入后）
    fn adjust_selection_for_insert(&mut self, inserted_index: usize) {
        // 调整选中的单元格索引
        if let Some((layer, frame)) = self.selection_state.selected_cell {
            if layer >= inserted_index {
                self.selection_state.selected_cell = Some((layer + 1, frame));
            }
        }

        // 调整选择范围索引
        if let Some((start_layer, start_frame)) = self.selection_state.selection_start {
            if start_layer >= inserted_index {
                self.selection_state.selection_start = Some((start_layer + 1, start_frame));
            }
        }
        if let Some((end_layer, end_frame)) = self.selection_state.selection_end {
            if end_layer >= inserted_index {
                self.selection_state.selection_end = Some((end_layer + 1, end_frame));
            }
        }
    }

    /// 调整编辑状态的索引（列插入后）
    fn adjust_editing_for_insert(&mut self, inserted_index: usize) {
        // 调整单元格编辑状态索引
        if let Some((layer, frame)) = self.edit_state.editing_cell {
            if layer >= inserted_index {
                self.edit_state.editing_cell = Some((layer + 1, frame));
            }
        }

        // 调整列名编辑状态索引
        if let Some(layer) = self.edit_state.editing_layer_name {
            if layer >= inserted_index {
                self.edit_state.editing_layer_name = Some(layer + 1);
            }
        }
    }

    /// 调整上下文菜单状态的索引（列插入后）
    fn adjust_context_menu_for_insert(&mut self, inserted_index: usize) {
        if let Some((layer, frame)) = self.context_menu.pos {
            if layer >= inserted_index {
                self.context_menu.pos = Some((layer + 1, frame));
            }
        }

        // 调整上下文菜单的选择范围
        if let Some(((start_layer, start_frame), (end_layer, end_frame))) = self.context_menu.selection {
            let new_start_layer = if start_layer >= inserted_index { start_layer + 1 } else { start_layer };
            let new_end_layer = if end_layer >= inserted_index { end_layer + 1 } else { end_layer };
            self.context_menu.selection = Some(((new_start_layer, start_frame), (new_end_layer, end_frame)));
        }
    }

    /// 删除指定位置的列
    pub fn delete_layer(&mut self, index: usize) {
        if let Some((name, cells)) = self.timesheet.delete_layer(index) {
            // 限制撤销栈大小
            if self.undo_stack.len() >= MAX_UNDO_ACTIONS {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(UndoAction::DeleteLayer { index, name, cells });
            self.is_modified = true;

            // 清理可能指向被删除列的状态
            self.clear_selection_if_layer_affected(index);
            self.clear_editing_if_layer_affected(index);
            self.clear_context_menu_if_layer_affected(index);
        }
    }

    /// 清理选择状态（如果受列删除影响）
    fn clear_selection_if_layer_affected(&mut self, deleted_index: usize) {
        // 如果选中的单元格在被删除的列或之后，清除选择
        if let Some((layer, _)) = self.selection_state.selected_cell {
            if layer >= deleted_index {
                self.selection_state.selected_cell = None;
            }
        }

        // 清除选择范围（如果涉及被删除的列）
        let should_clear_range = if let Some((start_layer, _)) = self.selection_state.selection_start {
            start_layer >= deleted_index
        } else {
            false
        } || if let Some((end_layer, _)) = self.selection_state.selection_end {
            end_layer >= deleted_index
        } else {
            false
        };

        if should_clear_range {
            self.selection_state.selection_start = None;
            self.selection_state.selection_end = None;
        }
    }

    /// 清理编辑状态（如果受列删除影响）
    fn clear_editing_if_layer_affected(&mut self, deleted_index: usize) {
        // 清除单元格编辑状态
        if let Some((layer, _)) = self.edit_state.editing_cell {
            if layer >= deleted_index {
                self.edit_state.editing_cell = None;
                self.edit_state.editing_text.clear();
            }
        }

        // 清除列名编辑状态
        if let Some(layer) = self.edit_state.editing_layer_name {
            if layer >= deleted_index {
                self.edit_state.editing_layer_name = None;
                self.edit_state.editing_layer_text.clear();
            }
        }
    }

    /// 清理上下文菜单状态（如果受列删除影响）
    fn clear_context_menu_if_layer_affected(&mut self, deleted_index: usize) {
        if let Some((layer, _)) = self.context_menu.pos {
            if layer >= deleted_index {
                self.context_menu.pos = None;
                self.context_menu.selection = None;
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
                UndoAction::InsertLayer { index } => {
                    // 撤销插入 = 删除该列（不记录撤销）
                    let _ = self.timesheet.delete_layer(index);
                }
                UndoAction::DeleteLayer { index, name, cells } => {
                    // 撤销删除 = 恢复该列
                    self.timesheet.cells.insert(index, cells);
                    self.timesheet.layer_names.insert(index, name);
                    self.timesheet.layer_count += 1;
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
                UndoAction::InsertLayer { .. } => std::mem::size_of::<UndoAction>(),
                UndoAction::DeleteLayer { cells, name, .. } => {
                    std::mem::size_of::<UndoAction>() +
                    cells.len() * std::mem::size_of::<Option<CellValue>>() +
                    name.len()
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

    /// 执行序列填充操作
    /// 从 start_value 到 end_value，每个数字重复 hold_frames 帧
    /// 例如：start=1, end=5, hold=2 -> 1122334455
    pub fn sequence_fill(&mut self, layer: usize, start_frame: usize, start_value: u32, end_value: u32, hold_frames: u32) -> Result<(), &'static str> {
        if hold_frames == 0 {
            return Err("Hold frames must be at least 1");
        }

        let total_frames = self.timesheet.total_frames();
        if start_frame >= total_frames {
            return Err("Start frame is out of range");
        }

        // 计算需要填充的帧数
        let value_count = if end_value >= start_value {
            end_value - start_value + 1
        } else {
            start_value - end_value + 1
        };
        let total_fill_frames = (value_count * hold_frames) as usize;

        // 限制不超出总帧数
        let write_end = (start_frame + total_fill_frames).min(total_frames);
        let actual_fill_frames = write_end - start_frame;

        if actual_fill_frames == 0 {
            return Err("No frames available to fill");
        }

        // 保存旧值用于撤销
        let mut old_values = Vec::new();
        let mut old_row = Vec::with_capacity(actual_fill_frames);
        for frame in start_frame..write_end {
            old_row.push(self.timesheet.get_cell(layer, frame).copied());
        }
        old_values.push(old_row);

        self.undo_stack.push_back(UndoAction::SetRange {
            min_layer: layer,
            min_frame: start_frame,
            old_values: Rc::new(old_values),
        });
        self.is_modified = true;

        // 填充序列值
        let mut write_frame = start_frame;
        let step: i32 = if end_value >= start_value { 1 } else { -1 };
        let mut current_value = start_value as i32;
        let end_value_i32 = end_value as i32;

        'outer: loop {
            for _ in 0..hold_frames {
                if write_frame >= write_end {
                    break 'outer;
                }
                self.timesheet.set_cell(layer, write_frame, Some(CellValue::Number(current_value as u32)));
                write_frame += 1;
            }

            if current_value == end_value_i32 {
                break;
            }
            current_value += step;
        }

        Ok(())
    }

    /// Generate AE Time Remap keyframe data for entire column and copy to clipboard
    /// version: AE keyframe version string like "6.0", "7.0", "8.0", "9.0"
    pub fn copy_ae_keyframes(&self, ctx: &egui::Context, layer: usize, version: &str) -> Result<(), &'static str> {
        if layer >= self.timesheet.layer_count {
            return Err("Invalid layer");
        }

        let framerate = self.timesheet.framerate as f64;
        let frame_count = self.timesheet.total_frames();
        let mut keyframe_text = String::with_capacity(1024);

        // AE keyframe header (use \r\n for Windows clipboard compatibility)
        keyframe_text.push_str("Adobe After Effects ");
        keyframe_text.push_str(version);
        keyframe_text.push_str(" Keyframe Data\r\n\r\n");
        keyframe_text.push_str("\tUnits Per Second\t");
        keyframe_text.push_str(&(framerate as u32).to_string());
        keyframe_text.push_str("\r\n\tSource Width\t1000\r\n\tSource Height\t1000\r\n");
        keyframe_text.push_str("\tSource Pixel Aspect Ratio\t1\r\n\tComp Pixel Aspect Ratio\t1\r\n\r\n");

        // Time Remap effect
        keyframe_text.push_str("Layer\r\n");
        keyframe_text.push_str("Time Remap\r\n");
        keyframe_text.push_str("\tFrame\tseconds\t\r\n");

        // Collect keyframes (only when value changes)
        let mut prev_value: Option<u32> = None;
        let mut last_frame = 0usize;

        for frame in 0..frame_count {
            let current_value = self.timesheet.get_actual_value(layer, frame);

            // Output keyframe when value changes
            if current_value != prev_value {
                // Frame number in timeline
                keyframe_text.push('\t');
                keyframe_text.push_str(&frame.to_string());
                keyframe_text.push('\t');

                if let Some(value) = current_value {
                    // Time Remap value: convert cell value to seconds
                    // Cell value 1 = frame 0 in source = 0 seconds
                    let time_seconds = (value.saturating_sub(1)) as f64 / framerate;
                    // Format with limited precision (AE uses ~7 decimal places)
                    if time_seconds == 0.0 {
                        keyframe_text.push_str("0");
                    } else {
                        // Remove trailing zeros from formatted number
                        let formatted = format!("{:.7}", time_seconds);
                        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
                        keyframe_text.push_str(trimmed);
                    }
                    last_frame = frame;
                } else {
                    // Empty cell - output 0
                    keyframe_text.push_str("0");
                }
                keyframe_text.push_str("\t\r\n");
                prev_value = current_value;
            }
        }

        // Add Effects section with Blinds (using match names for language independence)
        keyframe_text.push_str("\r\nEffects\tADBE Blinds\tADBE Blinds-0001\r\n");
        keyframe_text.push_str("\tFrame\tpercent\t\r\n");
        keyframe_text.push_str("\t0\t0\t\r\n");
        keyframe_text.push('\t');
        keyframe_text.push_str(&last_frame.to_string());
        keyframe_text.push_str("\t100\t\r\n");

        keyframe_text.push_str("\r\nEnd of Keyframe Data\r\n");

        // Copy to system clipboard
        ctx.output_mut(|o| o.copied_text = keyframe_text);

        Ok(())
    }
}
