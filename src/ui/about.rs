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
            .anchor(egui::Align2::LEFT_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("STS 3.0");
                    ui.add_space(10.0);
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(5.0);
                    ui.label("Animation Timesheet Editor");
                    ui.add_space(10.0);
                    ui.label("Written by Ma Chenxing © 2025");
                    ui.add_space(5.0);
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
