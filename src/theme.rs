use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

// 主题配置结构体，支持被转为 JSON 和从 JSON 读取
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,         
    pub is_dark: bool,        

    pub bg_normal: Color32,
    pub bg_header: Color32,
    pub bg_header_active: Color32,
    pub bg_header_hover: Color32,
    pub bg_header_editing: Color32,
    
    pub bg_selected: Color32,
    pub bg_in_selection: Color32,
    pub bg_editing: Color32,
    
    pub border_normal: Color32,
    pub border_selection: Color32,
    
    pub text_normal: Color32,
    pub text_header: Color32,
    pub text_frame: Color32,
    pub text_timecode: Color32, 
}

impl ThemeConfig {
    ///  MionaRira 
    pub fn miona_rira() -> Self {
        Self {
            name: "MionaRira (Default)".to_string(),
            is_dark: false,
            // 极致纯净的白底，彻底去除灰脏感
            bg_normal: Color32::WHITE, 
            // 表头用非常非常浅、极其通透的冰蓝色
            bg_header: Color32::from_rgb(242, 250, 255),
            // 激活的表头呈现明亮通透的青蓝
            bg_header_active: Color32::from_rgb(180, 235, 255),
            bg_header_hover: Color32::from_rgb(215, 245, 255),
            bg_header_editing: Color32::from_rgb(255, 245, 200),
            
            // 选中的格子，干净的亮青底色
            bg_selected: Color32::from_rgb(150, 220, 255),
            bg_in_selection: Color32::from_rgb(220, 245, 255),
            bg_editing: Color32::from_rgb(255, 250, 210),
            
            // 边框：平时是清爽的浅蓝灰，选中时是角色最核心的高亮青色
            border_normal: Color32::from_rgb(210, 230, 245),
            border_selection: Color32::from_rgb(0, 174, 239), // 核心 Cyan
            
            // 文字：干净利落的深海军蓝，极其清晰
            text_normal: Color32::from_rgb(20, 35, 55),
            text_header: Color32::from_rgb(15, 25, 45),
            text_frame: Color32::from_rgb(100, 130, 160),
            text_timecode: Color32::from_rgb(0, 160, 240),
        }
    }
    ///  Classic Light 
    pub fn light() -> Self { 
        Self {
            name: "Classic Light".to_string(),
            is_dark: false,
            bg_normal: Color32::WHITE,
            bg_header: Color32::from_gray(240),
            bg_header_active: Color32::from_gray(210),
            bg_header_hover: Color32::from_gray(225),
            bg_header_editing: Color32::from_rgb(255, 255, 200),
            bg_selected: Color32::from_rgb(200, 220, 255),
            bg_in_selection: Color32::from_rgb(225, 235, 255),
            bg_editing: Color32::from_rgb(255, 255, 200),
            border_normal: Color32::from_gray(200),
            border_selection: Color32::from_rgb(100, 150, 255), 
            text_normal: Color32::BLACK,
            text_header: Color32::BLACK,
            text_frame: Color32::DARK_GRAY,
            text_timecode: Color32::from_rgb(0, 100, 200), 
        }
    }
    ///  Classic Dark 
    pub fn dark() -> Self { 
        Self {
            name: "Classic Dark".to_string(),
            is_dark: true,
            bg_normal: Color32::from_rgb(30, 30, 30), 
            bg_header: Color32::from_rgb(45, 45, 45),
            bg_header_active: Color32::from_rgb(70, 70, 70),
            bg_header_hover: Color32::from_rgb(55, 55, 55),
            bg_header_editing: Color32::from_rgb(100, 100, 70),
            bg_selected: Color32::from_rgb(85, 85, 85),
            bg_in_selection: Color32::from_rgb(50, 50, 50),
            bg_editing: Color32::from_rgb(90, 90, 60),
            border_normal: Color32::from_rgb(70, 70, 70),
            border_selection: Color32::from_rgb(160, 160, 160), // 中性白边框高亮
            text_normal: Color32::from_rgb(220, 220, 220),
            text_header: Color32::from_rgb(240, 240, 240),
            text_frame: Color32::from_rgb(150, 150, 150),
            text_timecode: Color32::from_rgb(200, 200, 200),
        }
    }

    ///  STS Pro 
    pub fn sts_pro() -> Self {
        Self {
            name: "STS Pro".to_string(),
            is_dark: true,
            bg_normal: Color32::from_rgb(25, 27, 31),
            bg_header: Color32::from_rgb(35, 38, 45),
            bg_header_active: Color32::from_rgb(50, 80, 120),
            bg_header_hover: Color32::from_rgb(45, 50, 60),
            bg_header_editing: Color32::from_rgb(180, 130, 40),
            bg_selected: Color32::from_rgb(60, 90, 140),
            bg_in_selection: Color32::from_rgb(45, 65, 95),
            bg_editing: Color32::from_rgb(180, 130, 40),
            border_normal: Color32::from_rgb(60, 65, 75),
            border_selection: Color32::from_rgb(100, 180, 255), 
            text_normal: Color32::from_rgb(230, 235, 240),
            text_header: Color32::from_rgb(180, 190, 200),
            text_frame: Color32::from_rgb(140, 150, 160),
            text_timecode: Color32::from_rgb(100, 200, 255),
        }
    }

    ///  After Effects 
    pub fn ae_classic() -> Self {
        Self {
            name: "AE Classic".to_string(),
            is_dark: true,
            bg_normal: Color32::from_rgb(40, 40, 40),
            bg_header: Color32::from_rgb(50, 50, 50),
            bg_header_active: Color32::from_rgb(35, 35, 35),
            bg_header_hover: Color32::from_rgb(60, 60, 60),
            bg_header_editing: Color32::from_rgb(200, 150, 50),
            bg_selected: Color32::from_rgb(55, 70, 100),
            bg_in_selection: Color32::from_rgb(45, 55, 75),
            bg_editing: Color32::from_rgb(80, 70, 40),
            border_normal: Color32::from_rgb(25, 25, 25),
            border_selection: Color32::from_rgb(60, 140, 255), 
            text_normal: Color32::from_rgb(210, 210, 210),
            text_header: Color32::from_rgb(230, 230, 230),
            text_frame: Color32::from_rgb(150, 150, 150),
            text_timecode: Color32::from_rgb(80, 160, 255),
        }
    }

    ///  赛博朋克
    pub fn cyberpunk() -> Self {
        Self {
            name: "Neon Cyberpunk".to_string(),
            is_dark: true,
            bg_normal: Color32::from_rgb(15, 15, 20),
            bg_header: Color32::from_rgb(25, 25, 35),
            bg_header_active: Color32::from_rgb(255, 0, 100), 
            bg_header_hover: Color32::from_rgb(60, 20, 50),
            bg_header_editing: Color32::from_rgb(255, 255, 0),
            bg_selected: Color32::from_rgb(80, 10, 50),
            bg_in_selection: Color32::from_rgb(40, 15, 35),
            bg_editing: Color32::from_rgb(40, 40, 10),
            border_normal: Color32::from_rgb(40, 45, 60),
            border_selection: Color32::from_rgb(0, 255, 255), 
            text_normal: Color32::from_rgb(220, 220, 230),
            text_header: Color32::from_rgb(255, 255, 255),
            text_frame: Color32::from_rgb(0, 255, 255),
            text_timecode: Color32::from_rgb(255, 255, 0),
        }
    }

    ///  护眼
    pub fn eye_care() -> Self {
        Self {
            name: "Eye Care".to_string(),
            is_dark: false,
            bg_normal: Color32::from_rgb(199, 237, 204), 
            bg_header: Color32::from_rgb(170, 215, 175),
            bg_header_active: Color32::from_rgb(140, 190, 145),
            bg_header_hover: Color32::from_rgb(180, 225, 185),
            bg_header_editing: Color32::from_rgb(220, 210, 150),
            bg_selected: Color32::from_rgb(160, 205, 165),
            bg_in_selection: Color32::from_rgb(180, 220, 185),
            bg_editing: Color32::from_rgb(220, 220, 180),
            border_normal: Color32::from_rgb(150, 190, 155),
            border_selection: Color32::from_rgb(80, 120, 85),
            text_normal: Color32::from_rgb(50, 60, 50),
            text_header: Color32::from_rgb(40, 50, 40),
            text_frame: Color32::from_rgb(60, 70, 60),
            text_timecode: Color32::from_rgb(40, 100, 40),
        }
    }


    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json_str = serde_json::to_string_pretty(self).map_err(|e| format!("Serialization error: {}", e))?;
        fs::write(path, json_str).map_err(|e| format!("File write error: {}", e))?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let json_str = fs::read_to_string(path).map_err(|e| format!("File read error: {}", e))?;
        let theme: ThemeConfig = serde_json::from_str(&json_str).map_err(|e| format!("Deserialization error: {}", e))?;
        Ok(theme)
    }

    pub fn load_all_custom_themes(dir: &Path) -> Vec<ThemeConfig> {
        let mut themes = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(theme) = Self::load_from_file(&path) { themes.push(theme); }
                }
            }
        }
        themes
    }
}