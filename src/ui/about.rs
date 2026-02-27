//! About dialog component

use eframe::egui;

/// About dialog state
#[derive(Default)]
pub struct AboutDialog {
    pub open: bool,
}

impl AboutDialog {
    /// Render the about dialog. Returns true if dialog should close.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        // Dimmer background
        egui::Area::new(egui::Id::new("about_modal_dimmer"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
                );
            });

        let mut should_close = false;

        egui::Window::new("About STS")
            .collapsible(false)
            .resizable(false)
            // 弹窗依然在屏幕正中间
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    
                    // 👇 1. 加载顶部的 Miona 长方形 Banner 👇
                    // 注意：相对路径是从 src/ui/about.rs 指向根目录的 miona_banner.png
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/miona_banner.png"))
                            .max_width(320.0) // 限制图片最大宽度，适配弹窗大小
                            .rounding(6.0),   // 给图片加上精致的圆角
                    );
                    
                    ui.add_space(15.0);

                    // 👇 2. 软件名称与版本 👇
                    ui.heading("STS 3.0 (MionaRira Edition)");
                    ui.add_space(5.0);
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    
                    ui.label("Animation Timesheet Editor");
                    ui.add_space(15.0);
                    
                    // 👇 3. 极其规范的开源双署名 👇
                    ui.label("Original Core Written by Ma Chenxing © 2025");
                    ui.label("New Features by 银河猫抓板 © 2026");
                    
                    ui.add_space(15.0);
                });

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.vertical_centered(|ui| {
                    if ui.button("OK").clicked() || enter_pressed {
                        should_close = true;
                    }
                });
            });

        if should_close {
            self.open = false;
        }
    }
}