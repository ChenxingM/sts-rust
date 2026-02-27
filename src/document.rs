//! Document module - handles individual document state and operations

use eframe::egui;
use std::collections::{VecDeque, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use sts_rust::TimeSheet;
use sts_rust::models::timesheet::CellValue;

pub const MAX_UNDO_ACTIONS: usize = 100;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LayerType {
    Cel,      
    Pan,      
    Opacity,  
}

impl Default for LayerType {
    fn default() -> Self { Self::Cel }
}

#[derive(Clone)]
pub enum UndoAction {
    SetCell { layer: usize, frame: usize, old_value: Option<CellValue> },
    SetRange { min_layer: usize, min_frame: usize, old_values: Rc<Vec<Vec<Option<CellValue>>>> },
    InsertLayer { index: usize },
    DeleteLayer { 
        index: usize, 
        name: String, 
        cells: Vec<Option<CellValue>>,
        layer_type: Option<LayerType>,
        layer_folder: Option<PathBuf>,
    },
}

pub struct EditState {
    pub editing_cell: Option<(usize, usize)>,
    pub editing_layer_name: Option<usize>,
    pub editing_text: String,
    pub editing_layer_text: String,
    pub batch_edit_range: Option<(usize, usize, usize, usize)>,
    
    pub renaming_document: bool,
    pub rename_doc_buffer: String,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            editing_cell: None,
            editing_layer_name: None,
            editing_text: String::new(),
            editing_layer_text: String::new(),
            batch_edit_range: None,
            renaming_document: false,
            rename_doc_buffer: String::new(),
        }
    }
}

pub struct SelectionState {
    pub selected_cell: Option<(usize, usize)>,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub is_dragging: bool,
    pub auto_scroll_to_selection: bool,
    pub is_fill_dragging: bool,
    pub fill_source_range: Option<(usize, usize, usize, usize)>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            selected_cell: Some((0, 0)),
            selection_start: None,
            selection_end: None,
            is_dragging: false,
            auto_scroll_to_selection: false,
            is_fill_dragging: false,
            fill_source_range: None,
        }
    }
}

pub struct ContextMenuState {
    pub pos: Option<(usize, usize)>,
    pub screen_pos: egui::Pos2,
    pub selection: Option<((usize, usize), (usize, usize))>,
}

impl Default for ContextMenuState {
    fn default() -> Self { Self { pos: None, screen_pos: egui::Pos2::ZERO, selection: None } }
}

pub struct RepeatDialogState {
    pub open: bool,
    pub layer: usize,
    pub start_frame: usize,
    pub end_frame: usize,
    pub repeat_count_str: String,
    pub repeat_until_end: bool,
}

impl Default for RepeatDialogState {
    fn default() -> Self { Self { open: false, layer: 0, start_frame: 0, end_frame: 0, repeat_count_str: "1".to_string(), repeat_until_end: false } }
}

pub struct SequenceFillDialogState {
    pub open: bool,
    pub layer: usize,
    pub start_frame: usize,
    pub start_value_str: String,
    pub end_value_str: String,
    pub hold_frames_str: String,
}

impl Default for SequenceFillDialogState {
    fn default() -> Self { Self { open: false, layer: 0, start_frame: 0, start_value_str: "1".to_string(), end_value_str: "24".to_string(), hold_frames_str: "1".to_string() } }
}

pub struct MotionCurveDialogState {
    pub open: bool,
    pub layer: usize,
    pub start_frame: usize,
    pub end_frame: usize,
    pub start_val_str: String,
    pub total_drawings_str: String,
    pub p1: egui::Pos2,
    pub p2: egui::Pos2,
}

impl Default for MotionCurveDialogState {
    fn default() -> Self {
        Self {
            open: false,
            layer: 0,
            start_frame: 0,
            end_frame: 0,
            start_val_str: "1".to_string(),
            total_drawings_str: "5".to_string(),
            p1: egui::pos2(0.25, 0.25),
            p2: egui::pos2(0.75, 0.75),
        }
    }
}

pub type ClipboardData = Rc<Vec<Vec<Option<CellValue>>>>;

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
    pub motion_curve_dialog: MotionCurveDialogState,
    
    pub reference_image_dir: Option<std::path::PathBuf>,

    pub show_player: bool,
    pub player_selected_layer: usize,
    
    pub layer_folders: HashMap<usize, PathBuf>,
    pub layer_types: HashMap<usize, LayerType>,
    
    pub jump_step: usize,
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
            motion_curve_dialog: MotionCurveDialogState::default(),
            
            show_player: false,
            player_selected_layer: 0,
            
            layer_folders: HashMap::new(),
            layer_types: HashMap::new(), 
            
            jump_step: 1,
            reference_image_dir: None,
        }
    }

    pub fn title(&self) -> String {
        let base = if let Some(path) = &self.file_path { format!("{} - {}", self.timesheet.name, path) } else { self.timesheet.name.clone() };
        if self.is_modified { format!("{}*", base) } else { base }
    }

    pub fn save(&mut self) -> Result<(), String> {
        if let Some(path) = &self.file_path {
            match sts_rust::write_sts_file(&self.timesheet, path) {
                Ok(_) => { self.is_modified = false; Ok(()) }
                Err(e) => Err(format!("Failed to save: {}", e)),
            }
        } else { Err("No file path".to_string()) }
    }

    pub fn save_as(&mut self, path: String) -> Result<(), String> {
        match sts_rust::write_sts_file(&self.timesheet, &path) {
            Ok(_) => { self.file_path = Some(path.into_boxed_str()); self.is_modified = false; Ok(()) }
            Err(e) => Err(format!("Failed to save: {}", e)),
        }
    }

    pub fn auto_save(&mut self) { if self.file_path.is_some() { let _ = self.save(); } }

    // ==========================================
    // 👇 全新视觉滤镜引擎：自动翻译 A-Z 和数字 👇
    // ==========================================

    pub fn format_cell_value(&self, layer: usize, val: u32) -> String {
        let layer_type = self.layer_types.get(&layer).copied().unwrap_or(LayerType::Cel);
        match layer_type {
            LayerType::Pan => {
                if val >= 1 && val <= 26 {
                    ((b'A' + (val as u8) - 1) as char).to_string()
                } else {
                    val.to_string()
                }
            },
            LayerType::Opacity => val.to_string(), // 透明度直接显示数字
            LayerType::Cel => val.to_string(),
        }
    }

    pub fn get_cell_display_string(&self, layer: usize, frame: usize) -> String {
        match self.timesheet.get_cell(layer, frame) {
            Some(CellValue::Number(n)) => self.format_cell_value(layer, *n),
            Some(CellValue::Same) => "-".to_string(),
            None => "".to_string(),
        }
    }

    pub fn parse_cell_input(text: &str, layer_type: LayerType) -> Option<u32> {
        let t = text.trim();
        if t.is_empty() { return None; }
        
        if layer_type == LayerType::Pan && t.len() == 1 {
            let c = t.chars().next().unwrap().to_ascii_uppercase();
            if c >= 'A' && c <= 'Z' {
                return Some((c as u32) - 'A' as u32 + 1);
            }
        }
        t.parse::<u32>().ok()
    }

    pub fn parse_math_input(&self, layer_type: LayerType, input: &str, base_value: i32) -> Option<u32> {
        let text = input.trim(); 
        if text.is_empty() { return None; }
        
        let first = text.chars().next()?;
        match first {
            '+' | '-' | '*' | '/' => {
                if let Ok(op) = text[1..].trim().parse::<f64>() {
                    let base = base_value as f64;
                    let res = match first { '+' => base + op, '-' => (base - op).max(0.0), '*' => base * op, '/' => if op!=0.0 { base/op } else { base }, _ => base };
                    Some(res.round() as u32)
                } else { None }
            }, 
            _ => Self::parse_cell_input(text, layer_type)
        }
    }

    pub fn clear_all_cells(&mut self) {
        let total_frames = self.timesheet.total_frames();
        let layer_count = self.timesheet.layer_count;
        let mut old_values = Vec::new();
        for l in 0..layer_count {
            let mut row = Vec::new();
            for f in 0..total_frames { row.push(self.timesheet.get_cell(l, f).copied()); }
            old_values.push(row);
        }
        self.undo_stack.push_back(UndoAction::SetRange { min_layer: 0, min_frame: 0, old_values: Rc::new(old_values) });
        for l in 0..layer_count { for f in 0..total_frames { self.timesheet.set_cell(l, f, None); } }
        self.is_modified = true;
    }

    fn resolve_actual_value(&self, layer: usize, frame: usize) -> Option<u32> {
        for f in (0..=frame).rev() {
            match self.timesheet.get_cell(layer, f) {
                Some(CellValue::Number(n)) => return Some(*n),
                Some(CellValue::Same) => continue, 
                None => return None, 
            }
        }
        None
    }

    fn scan_upwards_for_numbers(&self, layer: usize, start_search_frame: usize) -> Vec<u32> {
        let mut samples = Vec::new();
        if start_search_frame == 0 { return samples; }

        let mut f = start_search_frame - 1;
        loop {
            if let Some(v) = self.resolve_actual_value(layer, f) {
                samples.push(v);
            } else {
                break;
            }
            if f == 0 { break; }
            f -= 1;
        }
        samples.reverse();
        samples
    }

    fn generate_smart_sequence(samples: &[u32], count_to_fill: usize) -> Vec<Option<CellValue>> {
        let sample_len = samples.len();
        let mut result = Vec::with_capacity(count_to_fill);
        
        if sample_len == 0 {
            for _ in 0..count_to_fill { result.push(None); }
            return result;
        }

        if sample_len == 1 {
            let val = samples[0];
            for _ in 0..count_to_fill { result.push(Some(CellValue::Number(val))); }
            return result;
        }

        let step1 = samples[1] as i32 - samples[0] as i32;
        let mut is_arithmetic = true;
        for i in 2..sample_len {
            if (samples[i] as i32 - samples[i-1] as i32) != step1 {
                is_arithmetic = false; break;
            }
        }

        if is_arithmetic {
            let mut current = samples.last().unwrap().clone() as i32;
            for _ in 0..count_to_fill {
                current += step1;
                result.push(Some(CellValue::Number(current.max(0) as u32)));
            }
            return result;
        }

        let mut groups: Vec<(u32, usize)> = Vec::new();
        let mut cur_val = samples[0];
        let mut cur_cnt = 1;
        for &val in samples.iter().skip(1) {
            if val == cur_val { cur_cnt += 1; }
            else {
                groups.push((cur_val, cur_cnt));
                cur_val = val;
                cur_cnt = 1;
            }
        }
        groups.push((cur_val, cur_cnt));

        if groups.len() >= 2 {
            let step = groups[1].0 as i32 - groups[0].0 as i32;
            let hold = groups[0].1;
            
            let consistent_step = groups.windows(2).all(|w| (w[1].0 as i32 - w[0].0 as i32) == step);
            let consistent_hold = groups.iter().take(groups.len()-1).all(|g| g.1 == hold);

            if consistent_step && consistent_hold {
                let mut current = groups.last().unwrap().0;
                let mut current_filled = groups.last().unwrap().1;
                
                for _ in 0..count_to_fill {
                    if current_filled < hold {
                        result.push(Some(CellValue::Number(current)));
                        current_filled += 1;
                    } else {
                        current = (current as i32 + step).max(0) as u32;
                        result.push(Some(CellValue::Number(current)));
                        current_filled = 1;
                    }
                }
                return result;
            }
        }

        for i in 0..count_to_fill {
            result.push(Some(CellValue::Number(samples[i % sample_len])));
        }
        result
    }

    #[inline]
    pub fn start_edit(&mut self, layer: usize, frame: usize) {
        self.edit_state.editing_cell = Some((layer, frame));
        self.edit_state.editing_text.clear();
        
        if self.edit_state.batch_edit_range.is_none() {
            if let Some((min_l, min_f, max_l, max_f)) = self.get_selection_range() {
                if layer >= min_l && layer <= max_l && frame >= min_f && frame <= max_f {
                     self.edit_state.batch_edit_range = Some((min_l, min_f, max_l, max_f));
                }
            }
        }
        if let Some(val) = self.resolve_actual_value(layer, frame) {
            // [修改] 开始编辑时，自动还原成 A-Z 字母供用户修改
            self.edit_state.editing_text.push_str(&self.format_cell_value(layer, val));
        }
    }

    #[inline] pub fn start_batch_edit(&mut self, l: usize, f: usize) { self.edit_state.batch_edit_range = self.get_selection_range(); self.start_edit(l, f); }

    #[inline]
    pub fn finish_edit(&mut self, move_down: bool, record_undo: bool) {
        if let Some((layer, frame)) = self.edit_state.editing_cell {
            let text = self.edit_state.editing_text.trim();
            let mut operation = None; 
            let mut direct_value = None;
            let mut inherit_above = false;
            let layer_type = self.layer_types.get(&layer).copied().unwrap_or(LayerType::Cel);

            if text.is_empty() {
                inherit_above = true;
            } else {
                let first_char = text.chars().next().unwrap();
                if ['+', '-', '*', '/'].contains(&first_char) {
                    if let Ok(operand) = text[1..].trim().parse::<f64>() {
                        operation = Some((first_char, operand));
                    }
                } else {
                    let mut math_split = None;
                    for (i, c) in text.chars().enumerate() {
                        if i > 0 && ['+', '-', '*', '/'].contains(&c) {
                            math_split = Some((i, c)); break;
                        }
                    }
                    if let Some((idx, op)) = math_split {
                        let left = text[0..idx].trim().parse::<f64>();
                        let right = text[idx+1..].trim().parse::<f64>();
                        if let (Ok(l), Ok(r)) = (left, right) {
                            let res = match op { '+' => l+r, '-' => (l-r).max(0.0), '*' => l*r, '/' => if r!=0.0{l/r}else{l}, _ => l };
                            direct_value = Some(CellValue::Number(res.round() as u32));
                        }
                    } 
                    if direct_value.is_none() { 
                        // [修改] 支持输入 A-Z
                        if let Some(n) = Self::parse_cell_input(text, layer_type) { 
                            direct_value = Some(CellValue::Number(n)); 
                        } 
                    }
                }
            }

            if let Some((min_layer, min_frame, max_layer, max_frame)) = self.edit_state.batch_edit_range {
                if record_undo {
                    let mut old = Vec::new();
                    for l in min_layer..=max_layer { let mut r = Vec::new(); for f in min_frame..=max_frame { r.push(self.timesheet.get_cell(l, f).copied()); } old.push(r); }
                    self.undo_stack.push_back(UndoAction::SetRange { min_layer, min_frame, old_values: Rc::new(old) });
                    self.is_modified = true;
                }
                for l in min_layer..=max_layer {
                    for f in min_frame..=max_frame {
                        if inherit_above {
                            if f > 0 { let v = self.timesheet.get_cell(l, f - 1).copied(); self.timesheet.set_cell(l, f, v); } 
                            else { self.timesheet.set_cell(l, f, None); }
                        } else if let Some((op, operand)) = operation {
                            let cur = self.resolve_actual_value(l, f).unwrap_or(0) as f64;
                            let res = match op { '+' => cur+operand, '-' => (cur-operand).max(0.0), '*' => cur*operand, '/' => if operand!=0.0{cur/operand}else{cur}, _ => cur };
                            self.timesheet.set_cell(l, f, Some(CellValue::Number(res.round() as u32)));
                        } else if let Some(val) = direct_value { self.timesheet.set_cell(l, f, Some(val)); } 
                        else { self.timesheet.set_cell(l, f, None); }
                    }
                }
                self.selection_state.selection_start = None; self.selection_state.selection_end = None;
            } else {
                let new_cell = if inherit_above { if frame > 0 { Some(CellValue::Same) } else { None } } 
                else if let Some((op, operand)) = operation {
                    let cur = self.resolve_actual_value(layer, frame).unwrap_or(0) as f64;
                    let res = match op { '+' => cur+operand, '-' => (cur-operand).max(0.0), '*' => cur*operand, '/' => if operand!=0.0{cur/operand}else{cur}, _ => cur };
                    Some(CellValue::Number(res.round() as u32))
                } else if let Some(val) = direct_value { Some(val) } else { self.timesheet.get_cell(layer, frame).copied() };

                let old = self.timesheet.get_cell(layer, frame).copied();
                if record_undo && old != new_cell { self.push_undo_set_cell(layer, frame, old); self.is_modified = true; }
                self.timesheet.set_cell(layer, frame, new_cell);

                if move_down {
                    let total = self.timesheet.total_frames(); let new_f = frame + self.jump_step;
                    if self.jump_step > 1 && new_cell.is_some() {
                        let limit = new_f.min(total);
                        for sf in (frame + 1)..limit {
                            let old_s = self.timesheet.get_cell(layer, sf).copied();
                            if record_undo && old_s != Some(CellValue::Same) { self.push_undo_set_cell(layer, sf, old_s); }
                            self.timesheet.set_cell(layer, sf, Some(CellValue::Same));
                        }
                    }
                    if new_f < total { self.selection_state.selected_cell = Some((layer, new_f)); self.selection_state.auto_scroll_to_selection = true; }
                }
            }
            self.edit_state.editing_cell = None; self.edit_state.editing_text.clear(); self.edit_state.batch_edit_range = None;
        }
    }

    pub fn apply_smart_fill(&mut self) {
        if !self.selection_state.is_fill_dragging { return; }
        if let (Some((src_min_l, src_min_f, src_max_l, src_max_f)), 
                Some((curr_min_l, _curr_min_f, curr_max_l, curr_max_f))) = 
                (self.selection_state.fill_source_range, self.get_selection_range()) 
        {
            if curr_max_f > src_max_f && src_min_l == curr_min_l && src_max_l == curr_max_l {
                let fill_start = src_max_f + 1;
                let fill_end = curr_max_f;
                let mut old = Vec::new();
                for l in src_min_l..=src_max_l {
                    let mut r = Vec::new();
                    for f in fill_start..=fill_end { r.push(self.timesheet.get_cell(l, f).copied()); }
                    old.push(r);
                }
                self.undo_stack.push_back(UndoAction::SetRange { min_layer: src_min_l, min_frame: fill_start, old_values: Rc::new(old) });
                self.is_modified = true;

                for layer in src_min_l..=src_max_l {
                    let mut samples = Vec::new();
                    for f in src_min_f..=src_max_f { 
                        if let Some(v) = self.resolve_actual_value(layer, f) {
                            samples.push(v);
                        }
                    }
                    
                    let total_fill = fill_end - fill_start + 1;
                    
                    let generated = Self::generate_smart_sequence(&samples, total_fill);

                    for (i, val) in generated.into_iter().enumerate() {
                        self.timesheet.set_cell(layer, fill_start + i, val);
                    }
                }
            }
        }
        self.selection_state.is_fill_dragging = false;
        self.selection_state.fill_source_range = None;
    }

    pub fn smart_fill_auto(&mut self) -> Result<(), &'static str> {
        let (layer, start_frame, end_frame) = self.check_single_column_selection()?;
        
        let mut keyframes = Vec::new();
        let mut has_trailing_empty = false;

        // 👇 核心意图判断：如果选区的最后一帧是空的，说明用户是在“往下拖拽拓展”，强制走序列推演！
        if self.timesheet.get_cell(layer, end_frame).is_none() {
            has_trailing_empty = true;
        }

        // 收集所有明确填写的数字
        for f in start_frame..=end_frame {
            if let Some(CellValue::Number(n)) = self.timesheet.get_cell(layer, f) {
                keyframes.push((f, *n));
            }
        }

        // 🚀 [引擎 A] 线性补间模式：必须不是拖拽拓展，且至少有两个关键帧
        if !has_trailing_empty && keyframes.len() >= 2 {
            let mut old_rows = Vec::new();
            let mut row = Vec::new();
            for f in start_frame..=end_frame { row.push(self.timesheet.get_cell(layer, f).copied()); }
            old_rows.push(row);
            self.undo_stack.push_back(UndoAction::SetRange { min_layer: layer, min_frame: start_frame, old_values: Rc::new(old_rows) });
            self.is_modified = true;

            for window in keyframes.windows(2) {
                let (f1, v1) = window[0];
                let (f2, v2) = window[1];
                let duration = (f2 - f1) as f64;
                
                let mut prev_val = v1;

                for f in (f1 + 1)..f2 {
                    // 👇 核心尊重机制：看看这个格子是不是空的
                    let is_empty = self.timesheet.get_cell(layer, f).is_none();

                    if is_empty {
                        // 只有格子是空的，才进行插值计算并填入！
                        let progress = (f - f1) as f64 / duration;
                        let current_val = (v1 as f64 + (v2 as f64 - v1 as f64) * progress).round() as u32;

                        if current_val != prev_val {
                            self.timesheet.set_cell(layer, f, Some(CellValue::Number(current_val)));
                            prev_val = current_val;
                        } else {
                            self.timesheet.set_cell(layer, f, Some(CellValue::Same));
                        }
                    } else {
                        // 如果格子不为空（比如用户打了 "-" 或者其他数字），我们绝对不覆盖它！
                        // 但为了后续计算准确，我们需要更新 prev_val 为它实际代表的数字
                        if let Some(actual_val) = self.resolve_actual_value(layer, f) {
                            prev_val = actual_val;
                        }
                    }
                }
            }
            return Ok(());
        }

        // 🛡️ [引擎 B] 传统推演模式 (处理 1,1,2,2,3,3 往下拖的情况)
        let mut samples = Vec::new();
        let mut selection_has_data = false;

        for f in start_frame..=end_frame {
            if let Some(v) = self.resolve_actual_value(layer, f) {
                samples.push(v);
                selection_has_data = true;
            }
        }

        if !selection_has_data {
            samples = self.scan_upwards_for_numbers(layer, start_frame);
        }

        if samples.is_empty() {
            return Err("No pattern found above or in selection");
        }

        let fill_start_frame = if !selection_has_data {
            start_frame
        } else {
            let mut last_val_idx = start_frame;
            for f in start_frame..=end_frame {
                if self.timesheet.get_cell(layer, f).is_some() {
                    last_val_idx = f;
                }
            }
            last_val_idx + 1
        };

        if fill_start_frame > end_frame {
            return Err("Selection is already full");
        }

        let count_to_fill = end_frame - fill_start_frame + 1;
        let generated = Self::generate_smart_sequence(&samples, count_to_fill);

        let mut old = Vec::new();
        let mut row = Vec::new();
        for f in fill_start_frame..=end_frame { row.push(self.timesheet.get_cell(layer, f).copied()); }
        old.push(row);
        self.undo_stack.push_back(UndoAction::SetRange { min_layer: layer, min_frame: fill_start_frame, old_values: Rc::new(old) });
        self.is_modified = true;

        for (i, val) in generated.into_iter().enumerate() {
            self.timesheet.set_cell(layer, fill_start_frame + i, val);
        }

        Ok(())
    }

    #[inline(always)] pub fn is_cell_in_selection(&self, l: usize, f: usize) -> bool {
        if let (Some((sl,sf)), Some((el,ef))) = (self.selection_state.selection_start, self.selection_state.selection_end) {
            l >= sl.min(el) && l <= sl.max(el) && f >= sf.min(ef) && f <= sf.max(ef)
        } else { false }
    }
    #[inline] pub fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        if let (Some((sl,sf)), Some((el,ef))) = (self.selection_state.selection_start, self.selection_state.selection_end) {
            Some((sl.min(el), sf.min(ef), sl.max(el), sf.max(ef)))
        } else { None }
    }
    pub fn delete_selection(&mut self) {
        if let Some((ml, mf, xl, xf)) = self.get_selection_range() {
            let mut old = Vec::new(); for l in ml..=xl { let mut r = Vec::new(); for f in mf..=xf { r.push(self.timesheet.get_cell(l, f).copied()); } old.push(r); }
            self.undo_stack.push_back(UndoAction::SetRange{min_layer:ml, min_frame:mf, old_values:Rc::new(old)}); self.is_modified=true;
            for l in ml..=xl { for f in mf..=xf { self.timesheet.set_cell(l, f, None); }}
        } else if let Some((l,f)) = self.selection_state.selected_cell {
            let o = self.timesheet.get_cell(l,f).copied(); self.push_undo_set_cell(l,f,o); self.is_modified=true; self.timesheet.set_cell(l,f,None);
        }
    }
    
    pub fn copy_selection(&mut self, ctx: &egui::Context) { 
        if let Some((ml, mf, xl, xf)) = self.get_selection_range() {
            let mut txt = String::new();
            let mut data = Vec::new();
            for l in ml..=xl {
                let mut row = Vec::new();
                for f in mf..=xf {
                    let c = self.timesheet.get_cell(l,f).copied();
                    row.push(c);
                    if f > mf { txt.push('\t'); }
                    match c { 
                        // [修改] 拷贝时带上翻译好的字母，方便在外部编辑器查看
                        Some(CellValue::Number(n)) => txt.push_str(&self.format_cell_value(l, n)), 
                        Some(CellValue::Same) => txt.push('-'), 
                        _=>{} 
                    }
                }
                data.push(row);
                if l < xl { txt.push('\n'); }
            }
            self.clipboard = Some(Rc::new(data));
            ctx.output_mut(|o| o.copied_text = txt);
        }
    }
    pub fn cut_selection(&mut self, ctx: &egui::Context) { self.copy_selection(ctx); self.delete_selection(); }
    pub fn paste_clipboard(&mut self) { 
        if let Some((sl, sf)) = self.selection_state.selected_cell {
            if let Some(cb) = &self.clipboard {
                let mut old = Vec::new();
                for (li, row) in cb.iter().enumerate() {
                    let mut orow = Vec::new();
                    for (fi, _) in row.iter().enumerate() { orow.push(self.timesheet.get_cell(sl+li, sf+fi).copied()); }
                    old.push(orow);
                }
                self.undo_stack.push_back(UndoAction::SetRange{min_layer:sl, min_frame:sf, old_values:Rc::new(old)});
                self.is_modified = true;
                for (li, row) in cb.iter().enumerate() { for (fi, val) in row.iter().enumerate() { self.timesheet.set_cell(sl+li, sf+fi, *val); } }
            }
        }
    }
    pub fn paste_from_text(&mut self, text: &str) -> bool { if let Some(cb) = Self::parse_clipboard_text(text) { self.clipboard=Some(cb); self.paste_clipboard(); true } else { false } }
    
    pub fn parse_clipboard_text(text: &str) -> Option<ClipboardData> {
        let lines: Vec<&str> = text.lines().collect(); if lines.is_empty() { return None; }
        let mut d = Vec::new();
        for l in lines { 
            d.push(l.split('\t').map(|s| {
                let s = s.trim();
                if s.is_empty() { None }
                else if s == "-" { Some(CellValue::Same) }
                else {
                    // [修改] 全局剪贴板支持自动将 A-Z 翻译成 1-26
                    if let Ok(n) = s.parse::<u32>() {
                        Some(CellValue::Number(n))
                    } else if s.len() == 1 {
                        let c = s.chars().next().unwrap().to_ascii_uppercase();
                        if c >= 'A' && c <= 'Z' {
                            Some(CellValue::Number((c as u32) - 'A' as u32 + 1))
                        } else { None }
                    } else { None }
                }
            }).collect()); 
        }
        Some(Rc::new(d))
    }
    
    pub fn insert_layer(&mut self, index: usize) { 
        self.timesheet.insert_layer(index); 
        self.undo_stack.push_back(UndoAction::InsertLayer{index}); 
        self.is_modified=true; 
        self.adjust_selection_for_insert(index); 
        self.adjust_editing_for_insert(index); 
        self.adjust_context_menu_for_insert(index); 
        
        let mut new_types = HashMap::new();
        for (k, v) in self.layer_types.drain() {
            if k >= index { new_types.insert(k + 1, v); } else { new_types.insert(k, v); }
        }
        self.layer_types = new_types;

        let mut new_folders = HashMap::new();
        for (k, v) in self.layer_folders.drain() {
            if k >= index { new_folders.insert(k + 1, v); } else { new_folders.insert(k, v); }
        }
        self.layer_folders = new_folders;
    }
    
    pub fn delete_layer(&mut self, index: usize) { 
        if let Some((n,c)) = self.timesheet.delete_layer(index) { 
            let layer_type = self.layer_types.remove(&index);
            let layer_folder = self.layer_folders.remove(&index);
            
            self.undo_stack.push_back(UndoAction::DeleteLayer{index, name:n, cells:c, layer_type, layer_folder}); 
            self.is_modified=true; 
            self.clear_selection_if_layer_affected(index); 
            self.clear_editing_if_layer_affected(index); 
            self.clear_context_menu_if_layer_affected(index); 

            let mut new_types = HashMap::new();
            for (k, v) in self.layer_types.drain() {
                if k > index { new_types.insert(k - 1, v); } else { new_types.insert(k, v); }
            }
            self.layer_types = new_types;

            let mut new_folders = HashMap::new();
            for (k, v) in self.layer_folders.drain() {
                if k > index { new_folders.insert(k - 1, v); } else { new_folders.insert(k, v); }
            }
            self.layer_folders = new_folders;
        } 
    }
    
    fn adjust_selection_for_insert(&mut self, i:usize) { if let Some((l,f))=self.selection_state.selected_cell{if l>=i{self.selection_state.selected_cell=Some((l+1,f));}} }
    fn adjust_editing_for_insert(&mut self, i:usize) { if let Some((l,f))=self.edit_state.editing_cell{if l>=i{self.edit_state.editing_cell=Some((l+1,f));}} }
    fn adjust_context_menu_for_insert(&mut self, _i:usize) {}
    fn clear_selection_if_layer_affected(&mut self, i:usize) { if self.selection_state.selected_cell.map_or(false, |(l,_)| l>=i) { self.selection_state.selected_cell=None; } }
    fn clear_editing_if_layer_affected(&mut self, i:usize) { if self.edit_state.editing_cell.map_or(false, |(l,_)| l>=i) { self.edit_state.editing_cell=None; } }
    fn clear_context_menu_if_layer_affected(&mut self, _i:usize) {}
    
    pub fn undo(&mut self) { 
        if let Some(a) = self.undo_stack.pop_back() {
            match a {
                UndoAction::SetCell{layer,frame,old_value} => self.timesheet.set_cell(layer,frame,old_value),
                UndoAction::SetRange{min_layer,min_frame,old_values} => { for (l,r) in old_values.iter().enumerate() { for (f,v) in r.iter().enumerate() { self.timesheet.set_cell(min_layer+l, min_frame+f, *v); } } },
                UndoAction::InsertLayer{index} => { 
                    self.timesheet.delete_layer(index); 
                    
                    let mut new_types = HashMap::new();
                    for (k, v) in self.layer_types.drain() {
                        if k > index { new_types.insert(k - 1, v); } else if k < index { new_types.insert(k, v); }
                    }
                    self.layer_types = new_types;

                    let mut new_folders = HashMap::new();
                    for (k, v) in self.layer_folders.drain() {
                        if k > index { new_folders.insert(k - 1, v); } else if k < index { new_folders.insert(k, v); }
                    }
                    self.layer_folders = new_folders;
                },
                UndoAction::DeleteLayer{index, name, cells, layer_type, layer_folder} => { 
                    self.timesheet.cells.insert(index, cells); 
                    self.timesheet.layer_names.insert(index, name); 
                    self.timesheet.layer_count+=1; 
                    
                    let mut new_types = HashMap::new();
                    for (k, v) in self.layer_types.drain() {
                        if k >= index { new_types.insert(k + 1, v); } else { new_types.insert(k, v); }
                    }
                    if let Some(lt) = layer_type { new_types.insert(index, lt); }
                    self.layer_types = new_types;

                    let mut new_folders = HashMap::new();
                    for (k, v) in self.layer_folders.drain() {
                        if k >= index { new_folders.insert(k + 1, v); } else { new_folders.insert(k, v); }
                    }
                    if let Some(lf) = layer_folder { new_folders.insert(index, lf); }
                    self.layer_folders = new_folders;
                }
            }
            self.is_modified=true;
        }
    }
    
    pub fn push_undo_set_cell(&mut self, l:usize, f:usize, v:Option<CellValue>) { if self.undo_stack.len()>=MAX_UNDO_ACTIONS{self.undo_stack.pop_front();} self.undo_stack.push_back(UndoAction::SetCell{layer:l,frame:f,old_value:v}); }
    pub fn estimate_undo_memory(&self) -> usize { 0 }
    pub fn check_single_column_selection(&self) -> Result<(usize, usize, usize), &'static str> { if let Some((ml,mf,xl,xf))=self.get_selection_range(){ if ml!=xl{Err("Single col only")}else{Ok((ml,mf,xf))} } else { Err("No sel") } }
    
    pub fn repeat_selection(&mut self, count: u32, until_end: bool) -> Result<(), &'static str> { 
        let (l, sf, ef) = self.check_single_column_selection()?;
        let mut src = Vec::new(); for f in sf..=ef { src.push(self.timesheet.get_cell(l,f).copied()); }
        let start = ef + 1;
        let total = self.timesheet.total_frames();
        let avail = total.saturating_sub(start);
        if avail == 0 { return Err("No frames"); }
        let write_len = if until_end { avail } else { (src.len() * count as usize).min(avail) };
        let mut old = Vec::new(); for f in start..(start+write_len) { old.push(self.timesheet.get_cell(l,f).copied()); }
        self.undo_stack.push_back(UndoAction::SetRange{min_layer:l, min_frame:start, old_values:Rc::new(vec![old])});
        self.is_modified = true;
        let mut w = start;
        while w < start + write_len { for v in &src { if w >= start+write_len {break;} self.timesheet.set_cell(l, w, *v); w+=1; } }
        Ok(())
    }
    
    pub fn sequence_fill(&mut self, l: usize, sf: usize, sv: u32, ev: u32, h: u32) -> Result<(), &'static str> {
        let total = self.timesheet.total_frames();
        let count = if ev>=sv { ev-sv+1 } else { sv-ev+1 };
        let frames = (count * h) as usize;
        let end = (sf + frames).min(total);
        if end <= sf { return Err("No frames"); }
        let mut old = Vec::new(); for f in sf..end { old.push(self.timesheet.get_cell(l,f).copied()); }
        self.undo_stack.push_back(UndoAction::SetRange{min_layer:l, min_frame:sf, old_values:Rc::new(vec![old])});
        self.is_modified = true;
        let mut w = sf; let step = if ev>=sv {1} else {-1}; let mut cur = sv as i32;
        'o: loop { for _ in 0..h { if w>=end {break 'o;} self.timesheet.set_cell(l,w,Some(CellValue::Number(cur as u32))); w+=1; } if cur == ev as i32 {break;} cur += step; }
        Ok(())
    }
    
    // ==========================================
    // 👇 AE 导出升级：完美适配透明度层 👇
    // ==========================================
    pub fn copy_ae_keyframes(&self, ctx: &egui::Context, l: usize, v: &str) -> Result<(), &'static str> { 
        let fps = self.timesheet.framerate as f64;
        let layer_type = self.layer_types.get(&l).copied().unwrap_or(LayerType::Cel);

        let mut txt = format!("Adobe After Effects {} Keyframe Data\r\n\r\n\tUnits Per Second\t{}\r\n\tSource Width\t1000\r\n\tSource Height\t1000\r\n\tSource Pixel Aspect Ratio\t1\r\n\tComp Pixel Aspect Ratio\t1\r\n\r\n", v, fps as u32);
        
        match layer_type {
            LayerType::Opacity => {
                txt.push_str("Transform\tOpacity\r\n\tFrame\tpercent\t\r\n");
                let mut prev = None;
                for f in 0..self.timesheet.total_frames() {
                    let cur = self.timesheet.get_actual_value(l, f);
                    if cur.is_some() && cur != prev {
                        txt.push_str(&format!("\t{}\t{}\t\r\n", f, cur.unwrap()));
                        prev = cur;
                    }
                }
            }
            LayerType::Pan => {
                // 👇 输出我们自定义格式的干净文本，专门喂给我们的 AE 脚本 👇
                txt.push_str(&format!("STS_MARKER_DATA\r\nFPS:{}\r\n", fps));
                let mut prev = None;
                for f in 0..self.timesheet.total_frames() {
                    let cur = self.timesheet.get_actual_value(l, f);
                    if cur.is_some() && cur != prev {
                        let val_str = self.format_cell_value(l, cur.unwrap());
                        txt.push_str(&format!("{}\t{}\r\n", f, val_str));
                        prev = cur;
                    }
                }
            }
            _ => { 
                txt.push_str("Time Remap\r\n\tFrame\tseconds\t\r\n");
                let mut prev = None;
                for f in 0..self.timesheet.total_frames() {
                    let cur = self.timesheet.get_actual_value(l, f);
                    if cur != prev {
                        let t = if let Some(val) = cur { format!("{:.7}", (val.saturating_sub(1)) as f64 / fps) } else { "0".to_string() };
                        txt.push_str(&format!("\t{}\t{}\t\r\n", f, t.trim_end_matches('0').trim_end_matches('.')));
                        prev = cur;
                    }
                }
            }
        }
        
        txt.push_str("\r\nEnd of Keyframe Data\r\n");
        ctx.output_mut(|o| o.copied_text = txt);
        Ok(())
    }

    pub fn reverse_selection(&mut self) -> Result<(), &'static str> {
        let (layer, start_frame, end_frame) = self.check_single_column_selection()?;
        let mut src_values = Vec::new();
        let mut selection_has_data = false;
        for f in start_frame..=end_frame {
            if self.timesheet.get_cell(layer, f).is_some() { selection_has_data = true; break; }
        }
        if selection_has_data {
            let mut split_idx = None;
            for f in start_frame..=end_frame { if self.timesheet.get_cell(layer, f).is_none() { split_idx = Some(f); break; } }
            if let Some(empty_start) = split_idx {
                let src_end = empty_start - 1;
                for f in start_frame..=src_end { src_values.push(self.timesheet.get_cell(layer, f).copied()); }
                src_values.reverse(); 
                let write_count = (end_frame - empty_start + 1).min(src_values.len());
                let mut old_rows = Vec::new();
                for i in 0..write_count { old_rows.push(self.timesheet.get_cell(layer, empty_start + i).copied()); }
                self.undo_stack.push_back(UndoAction::SetRange { min_layer: layer, min_frame: empty_start, old_values: Rc::new(vec![old_rows]) });
                self.is_modified = true;
                for i in 0..write_count { self.timesheet.set_cell(layer, empty_start + i, src_values[i]); }
                return Ok(());
            } else {
                for f in start_frame..=end_frame { src_values.push(self.timesheet.get_cell(layer, f).copied()); }
                src_values.reverse();
            }
        } else {
            let upwards = self.scan_upwards_for_numbers(layer, start_frame);
            if upwards.is_empty() { return Err("Nothing above to mirror"); }
            for &val in upwards.iter().rev() { src_values.push(Some(CellValue::Number(val))); }
        }
        if src_values.is_empty() { return Ok(()); }
        let count_to_write = (end_frame - start_frame + 1).min(src_values.len());
        let mut changes_needed = false;
        for i in 0..count_to_write {
            if self.timesheet.get_cell(layer, start_frame + i) != src_values[i].as_ref() { changes_needed = true; break; }
        }
        if changes_needed {
            let mut old_rows = Vec::new();
            for i in 0..count_to_write { old_rows.push(self.timesheet.get_cell(layer, start_frame + i).copied()); }
            self.undo_stack.push_back(UndoAction::SetRange { min_layer: layer, min_frame: start_frame, old_values: Rc::new(vec![old_rows]) });
            self.is_modified = true;
            for i in 0..count_to_write { self.timesheet.set_cell(layer, start_frame + i, src_values[i]); }
        }
        Ok(())
    }

    fn solve_cubic_bezier_y_for_x(target_x: f64, p1: egui::Pos2, p2: egui::Pos2) -> f64 {
        let p1x = p1.x as f64;
        let p2x = p2.x as f64;
        
        let mut t = target_x;
        for _ in 0..8 {
            let one_minus_t = 1.0 - t;
            let current_x = 3.0 * one_minus_t.powi(2) * t * p1x 
                          + 3.0 * one_minus_t * t.powi(2) * p2x 
                          + t.powi(3);
            
            let dx_dt = 3.0 * one_minus_t.powi(2) * p1x 
                      + 6.0 * one_minus_t * t * (p2x - p1x) 
                      + 3.0 * t.powi(2) * (1.0 - p2x);
            
            if dx_dt.abs() < 1e-6 { break; }
            t -= (current_x - target_x) / dx_dt;
        }
        t = t.clamp(0.0, 1.0);

        let p1y = p1.y as f64; 
        let p2y = p2.y as f64; 

        let one_minus_t = 1.0 - t;
        
        3.0 * one_minus_t.powi(2) * t * p1y 
        + 3.0 * one_minus_t * t.powi(2) * p2y 
        + t.powi(3)
    }

    pub fn set_keyframe_curve(&mut self, layer: usize, frame_start: usize, p1: egui::Pos2, p2: egui::Pos2, start_val: u32, num_drawings: u32, duration: u32) {
        
        let total_frames = duration as usize;
        if total_frames < 1 { return; }

        let mut old_rows = Vec::new();
        for i in 0..total_frames {
            old_rows.push(self.timesheet.get_cell(layer, frame_start + i).copied());
        }
        self.undo_stack.push_back(UndoAction::SetRange { 
            min_layer: layer, 
            min_frame: frame_start, 
            old_values: Rc::new(vec![old_rows]) 
        });
        self.is_modified = true;

        let mut prev_written_val: Option<u32> = None;

        for i in 0..total_frames {
            let progress_x = if total_frames > 1 {
                i as f64 / (total_frames - 1) as f64
            } else { 
                1.0 
            };

            let progress_y = Self::solve_cubic_bezier_y_for_x(progress_x, p1, p2);
            
            let drawing_offset = (progress_y * (num_drawings.max(1) - 1) as f64).round() as u32;
            let final_val = start_val + drawing_offset;

            let should_write_number = if i == 0 {
                true 
            } else {
                Some(final_val) != prev_written_val
            };

            if should_write_number {
                self.timesheet.set_cell(layer, frame_start + i, Some(CellValue::Number(final_val)));
                prev_written_val = Some(final_val);
            } else {
                self.timesheet.set_cell(layer, frame_start + i, Some(CellValue::Same));
            }
        }
    }
}