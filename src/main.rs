#![windows_subsystem = "windows"]
#![allow(dead_code)] 

mod document;
mod app;
mod ui;
pub mod settings;
mod i18n;
mod theme; 
mod video_utils;

// ... 下面的 font 设置和 main 函数保持不变 ...
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    
    // 尝试加载 Windows 系统中文字体
    // 💡小提示：如果您想完美支持日文显示，可以在这里顺便加一个日文字体路径
    // 比如 "C:\\Windows\\Fonts\\msgothic.ttc"
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",      // Microsoft YaHei
        "C:\\Windows\\Fonts\\simhei.ttf",    // SimHei
        "C:\\Windows\\Fonts\\simsun.ttc",    // SimSun
    ];

    let mut font_loaded = false;
    for font_path in &font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
            font_loaded = true;
            break;
        }
    }

    if font_loaded {
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("chinese".to_owned());

        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("chinese".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../assets/window_icon.ico");
    let icon_image = image::load_from_memory(icon_bytes).ok()?.into_rgba8();
    let (width, height) = icon_image.dimensions();
    Some(egui::IconData {
        rgba: icon_image.into_raw(),
        width,
        height,
    })
}

fn main() -> Result<(), eframe::Error> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 720.0])
        .with_title("STS 3.0 - MionaRira Edition");

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "STS 3.0 - MionaRira Edition",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            // 注册图片加载器，允许播放器读取本地序列帧
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(crate::app::StsApp::default()))
        }),
    )
}