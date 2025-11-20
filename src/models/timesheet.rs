use serde::{Deserialize, Serialize};

/// 摄影表格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSheet {
    /// 名称
    pub name: String,
    
    /// 帧率
    pub framerate: u32,
    
    /// 每页帧数
    pub frames_per_page: u32,
    
    /// 图层数量
    pub layer_count: usize,
    
    /// 图层名称
    pub layer_names: Vec<String>,
    
    /// 单元格数据 [层][帧]
    /// None = 空单元格
    /// Some(CellValue::Number(n)) = 数字
    /// Some(CellValue::Same) = "-" (和上一格相同)
    pub cells: Vec<Vec<Option<CellValue>>>,
    
    /// 源文件宽度
    pub source_width: u32,
    
    /// 源文件高度
    pub source_height: u32,
    
    /// 源像素纵横比
    pub source_pixel_aspect_ratio: f64,
    
    /// 合成像素纵横比
    pub comp_pixel_aspect_ratio: f64,
}

/// 单元格值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellValue {
    /// 数字
    Number(u32),
    /// 和上一格相同 (显示为 "-")
    Same,
}

impl TimeSheet {
    /// 创建新的摄影表
    pub fn new(name: String, framerate: u32, layer_count: usize, frames_per_page: u32) -> Self {
        let layer_names = (0..layer_count)
            .map(|i| Self::column_name(i))
            .collect();

        // 初始创建空表格，通过 ensure_frames 设置实际帧数
        let cells = vec![vec![]; layer_count];

        Self {
            name,
            framerate,
            frames_per_page,
            layer_count,
            layer_names,
            cells,
            source_width: 640,
            source_height: 480,
            source_pixel_aspect_ratio: 1.0,
            comp_pixel_aspect_ratio: 1.0,
        }
    }

    /// 生成列名
    #[inline]
    pub fn column_name(index: usize) -> String {
        let mut result = String::new();
        let mut n = index;
        
        loop {
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        
        result
    }

    /// 获取单元格值
    #[inline(always)]
    pub fn get_cell(&self, layer: usize, frame: usize) -> Option<&CellValue> {
        self.cells.get(layer)?.get(frame)?.as_ref()
    }

    /// 设置单元格值
    #[inline]
    pub fn set_cell(&mut self, layer: usize, frame: usize, value: Option<CellValue>) {
        if let Some(layer_cells) = self.cells.get_mut(layer) {
            if frame >= layer_cells.len() {
                // 扩展帧数
                layer_cells.resize(frame + 1, None);
            }
            layer_cells[frame] = value;
        }
    }

    /// 获取单元格的实际值
    #[inline]
    pub fn get_actual_value(&self, layer: usize, frame: usize) -> Option<u32> {
        let cell = self.get_cell(layer, frame)?;
        
        match cell {
            CellValue::Number(n) => Some(*n),
            CellValue::Same => {
                // 向上查找最近的数字
                for prev_frame in (0..frame).rev() {
                    if let Some(CellValue::Number(n)) = self.get_cell(layer, prev_frame) {
                        return Some(*n);
                    }
                }
                None
            }
        }
    }

    /// 获取页号和页内帧号 (1-indexed)
    #[inline(always)]
    pub fn get_page_and_frame(&self, frame_index: usize) -> (u32, u32) {
        let frame_num = frame_index as u32 + 1; // 1-indexed
        let page = (frame_num - 1) / self.frames_per_page + 1;
        let frame_in_page = (frame_num - 1) % self.frames_per_page + 1;
        (page, frame_in_page)
    }

    /// 获取总帧数
    #[inline]
    pub fn total_frames(&self) -> usize {
        self.cells.get(0).map_or(0, |v| v.len())
    }

    /// 扩展到指定帧数
    pub fn ensure_frames(&mut self, frame_count: usize) {
        for layer_cells in &mut self.cells {
            if layer_cells.len() < frame_count {
                layer_cells.resize(frame_count, None);
            }
        }
    }
}

impl Default for TimeSheet {
    fn default() -> Self {
        Self::new("sheet1".to_string(), 24, 12, 144)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_name() {
        assert_eq!(TimeSheet::column_name(0), "A");
        assert_eq!(TimeSheet::column_name(1), "B");
        assert_eq!(TimeSheet::column_name(25), "Z");
        assert_eq!(TimeSheet::column_name(26), "AA");
        assert_eq!(TimeSheet::column_name(27), "AB");
    }

    #[test]
    fn test_page_and_frame() {
        let ts = TimeSheet::new("test".to_string(), 24, 12, 144);
        
        // 第1帧 = 第1页第1帧
        assert_eq!(ts.get_page_and_frame(0), (1, 1));
        
        // 第143帧 = 第1页第143帧
        assert_eq!(ts.get_page_and_frame(142), (1, 143));
        
        // 第144帧 = 第1页第144帧
        assert_eq!(ts.get_page_and_frame(143), (1, 144));
        
        // 第145帧 = 第2页第1帧
        assert_eq!(ts.get_page_and_frame(144), (2, 1));
    }

    #[test]
    fn test_actual_value() {
        let mut ts = TimeSheet::new("test".to_string(), 24, 2, 144);
        
        ts.set_cell(0, 0, Some(CellValue::Number(1)));
        ts.set_cell(0, 1, Some(CellValue::Same));
        ts.set_cell(0, 2, Some(CellValue::Number(2)));
        ts.set_cell(0, 3, Some(CellValue::Same));
        
        assert_eq!(ts.get_actual_value(0, 0), Some(1));
        assert_eq!(ts.get_actual_value(0, 1), Some(1)); // "-" = 1
        assert_eq!(ts.get_actual_value(0, 2), Some(2));
        assert_eq!(ts.get_actual_value(0, 3), Some(2)); // "-" = 2
    }
}
