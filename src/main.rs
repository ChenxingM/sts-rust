#![windows_subsystem = "windows"]
#![allow(dead_code)] // Allow unused helper functions for future use

mod document;
mod app;
mod ui;
pub mod settings;

use app::StsApp;

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 尝试加载 Windows 系统中文字体
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
        // 将中文字体添加到所有字体族中（在默认字体之后）
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
    let icon_bytes = include_bytes!("../icon.ico");
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
        .with_inner_size([1200.0, 800.0])
        .with_title("STS 3.0");

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "STS 3.0",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(StsApp::default()))
        }),
    )
}
