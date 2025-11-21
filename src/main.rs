#![windows_subsystem = "windows"]
#![allow(dead_code)] // Allow unused helper functions for future use

mod document;
mod app;
mod ui;

use app::StsApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("STS 3.0"),
        ..Default::default()
    };

    eframe::run_native(
        "STS 3.0",
        options,
        Box::new(|_cc| Ok(Box::new(StsApp::default()))),
    )
}
