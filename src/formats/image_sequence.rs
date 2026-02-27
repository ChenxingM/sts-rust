// src/formats/image_sequence.rs
use std::path::{Path, PathBuf};

/// 核心算法：无视前缀，提取文件名中的数字进行比对
pub fn find_image_by_index(folder: &Path, target_index: u32) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }

            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                // 提取文件名中所有的数字，例如 "Layer_A_005" -> "005"
                let numeric_str: String = file_stem.chars().filter(|c| c.is_ascii_digit()).collect();
                
                if let Ok(num) = numeric_str.parse::<u32>() {
                    if num == target_index {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}