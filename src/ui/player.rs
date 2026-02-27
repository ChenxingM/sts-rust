// src/ui/player.rs

use eframe::egui;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::document::Document;
use crate::i18n::Translation; 

fn find_image_for_value(dir: &Path, val: u32) -> Option<PathBuf> {
    let patterns = [
        format!("{}.png", val), format!("{}.jpg", val), format!("{}.tga", val), format!("{}.tif", val),
        format!("{:04}.png", val), format!("{:04}.jpg", val), format!("{:04}.tga", val), format!("{:04}.tif", val),
        format!("{:03}.png", val), format!("{:03}.jpg", val), format!("{:03}.tga", val), format!("{:03}.tif", val),
    ];
    for p in &patterns {
        let path = dir.join(p);
        if path.exists() { return Some(path); }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let numbers: String = stem.chars().filter(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = numbers.parse::<u32>() { if n == val { return Some(path); } }
                }
            }
        }
    }
    None
}

pub struct SequencePlayer {
    pub playing: bool, pub current_frame: usize, pub loop_playback: bool, pub preview_mode: i32, 
    pub show_frame_number: bool, last_time: f64, accumulator: f64, pub cached_dir_counts: HashMap<PathBuf, usize>,
}

impl Default for SequencePlayer {
    fn default() -> Self {
        Self { playing: false, current_frame: 0, loop_playback: true, preview_mode: -1, show_frame_number: true, last_time: 0.0, accumulator: 0.0, cached_dir_counts: HashMap::new() }
    }
}

impl SequencePlayer {
    pub fn new() -> Self { Self::default() }

    fn get_dir_image_count(&mut self, dir: &Path) -> usize {
        if let Some(&c) = self.cached_dir_counts.get(dir) { return c; }
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "png" || ext_lower == "jpg" || ext_lower == "jpeg" || ext_lower == "tga" || ext_lower == "tif" || ext_lower == "tiff" { count += 1; }
                    }
                }
            }
        }
        self.cached_dir_counts.insert(dir.to_path_buf(), count);
        count
    }

    fn is_frame_playable(&self, doc: &Document, frame: usize) -> bool {
        if self.preview_mode == -1 {
            if let Some(dir) = &doc.reference_image_dir {
                let file_name = format!("frame_{:04}.png", frame + 1);
                return dir.join(&file_name).exists();
            }
            false
        } else {
            let l_idx = self.preview_mode as usize;
            if let Some(dir) = doc.layer_folders.get(&l_idx) {
                if let Some(val) = doc.timesheet.get_actual_value(l_idx, frame) {
                    return find_image_for_value(dir, val).is_some();
                }
            }
            false
        }
    }

    fn get_first_playable_frame(&self, doc: &Document, total_frames: usize) -> Option<usize> {
        for f in 0..total_frames { if self.is_frame_playable(doc, f) { return Some(f); } }
        None
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document, text: &Translation) -> Option<Result<String, String>> {
        let mut is_open = doc.show_player;
        if !is_open { return None; } 

        let mut bake_result = None; // 一个变量，用来装烘焙的消息

        let total_frames = doc.timesheet.total_frames().max(1);
        let fps = doc.timesheet.framerate as f64;
        let frame_time = 1.0 / fps.max(1.0);

        if !self.playing {
            if let Some((_, f)) = doc.selection_state.selected_cell { self.current_frame = f; }
        }

        if self.playing && !self.is_frame_playable(doc, self.current_frame) {
            if self.loop_playback {
                if let Some(first_valid) = self.get_first_playable_frame(doc, total_frames) { self.current_frame = first_valid; } 
                else { self.playing = false; }
            } else { self.playing = false; }
        }

        if self.playing {
            let now = ctx.input(|i| i.time);
            if self.last_time == 0.0 { self.last_time = now; }
            let dt = now - self.last_time;
            self.last_time = now;
            self.accumulator += dt;

            while self.accumulator >= frame_time {
                let next_frame = self.current_frame + 1;
                let hit_end = next_frame >= total_frames || !self.is_frame_playable(doc, next_frame);

                if hit_end {
                    if self.loop_playback {
                        if let Some(first_valid) = self.get_first_playable_frame(doc, total_frames) {
                            if first_valid == self.current_frame { self.accumulator -= frame_time; continue; }
                            self.current_frame = first_valid;
                        } else { self.playing = false; break; }
                    } else { self.playing = false; break; }
                } else { self.current_frame = next_frame; }
                self.accumulator -= frame_time;
            }

            let current_layer = doc.selection_state.selected_cell.map(|(l, _)| l).unwrap_or(0);
            doc.selection_state.selected_cell = Some((current_layer, self.current_frame));
            doc.selection_state.auto_scroll_to_selection = true; 
            ctx.request_repaint(); 
        } else {
            self.last_time = 0.0; self.accumulator = 0.0;
        }

        if self.current_frame >= total_frames { self.current_frame = total_frames - 1; }
        
        let doc_ptr = doc as *const Document as usize;
        egui::Window::new(text.player_title)
            .id(egui::Id::new("preview_player_window").with(doc_ptr))
            .open(&mut is_open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                
                ui.horizontal(|ui| {
                    ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

                    let play_btn_text = if self.playing { text.player_pause } else { text.player_play };
                    if ui.selectable_label(self.playing, play_btn_text).clicked() { self.playing = !self.playing; }
                    
                    if ui.selectable_label(false, text.player_stop).clicked() {
                        self.playing = false;
                        self.current_frame = 0;
                        let current_layer = doc.selection_state.selected_cell.map(|(l, _)| l).unwrap_or(0);
                        doc.selection_state.selected_cell = Some((current_layer, 0));
                    }
                    ui.checkbox(&mut self.loop_playback, text.player_loop);
                    ui.checkbox(&mut self.show_frame_number, "No."); 
                    
                    ui.separator();
                    ui.label(format!("Frame: {} / {}", self.current_frame + 1, total_frames));
                    ui.label(format!("FPS: {}", fps));
                });

                ui.horizontal(|ui| {
                    ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

                    ui.label(text.player_source);
                    let selected_text = if self.preview_mode == -1 {
                        text.player_ref_video.to_string()
                    } else {
                        let l_idx = self.preview_mode as usize;
                        doc.timesheet.layer_names.get(l_idx).cloned().unwrap_or_else(|| format!("Layer {}", l_idx + 1))
                    };

                    egui::ComboBox::from_id_salt("preview_source_combo")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.preview_mode, -1, text.player_ref_video);
                            ui.separator();
                            for i in 0..doc.timesheet.layer_count {
                                let name = doc.timesheet.layer_names.get(i).cloned().unwrap_or_else(|| format!("Layer {}", i+1));
                                ui.selectable_value(&mut self.preview_mode, i as i32, format!("📄 {}", name));
                            }
                        });

                    if self.preview_mode >= 0 {
                        let l_idx = self.preview_mode as usize;
                        if ui.selectable_label(false, text.player_bind_folder).clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                doc.layer_folders.insert(l_idx, folder.clone());
                                self.cached_dir_counts.remove(&folder); 
                            }
                        }
                        
                        // 新增：Bake 按钮 
                        if ui.selectable_label(false, text.player_bake).on_hover_text("Bake this layer to a sequence folder").clicked() {
                            if let Some(src_dir) = doc.layer_folders.get(&l_idx).cloned() {
                                if let Some(out_dir) = rfd::FileDialog::new().pick_folder() {
                                    let total = doc.timesheet.total_frames();
                                    let mut success_count = 0;
                                    
                                    for f in 0..total {
                                        if let Some(val) = doc.timesheet.get_actual_value(l_idx, f) {
                                            if let Some(src_path) = find_image_for_value(&src_dir, val) {
                                                let ext = src_path.extension().and_then(|e| e.to_str()).unwrap_or("png");
                                                let dst_path = out_dir.join(format!("frame_{:04}.{}", f + 1, ext));
                                                
                                                if std::fs::hard_link(&src_path, &dst_path).is_err() {
                                                    let _ = std::fs::copy(&src_path, &dst_path);
                                                }
                                                success_count += 1;
                                            }
                                        }
                                    }
                                    bake_result = Some(Ok(format!("Baked {} frames successfully!", success_count)));
                                }
                            } else {
                                bake_result = Some(Err("Please bind a folder first! (请先绑定文件夹)".to_string()));
                            }
                        }
                    }

                    ui.add_space(8.0);
                    if self.preview_mode == -1 {
                        if let Some(dir) = &doc.reference_image_dir {
                            let count = self.get_dir_image_count(dir);
                            ui.label(egui::RichText::new(format!("📦 {} f", count)).color(egui::Color32::GRAY));
                        }
                    } else {
                        let l_idx = self.preview_mode as usize;
                        if let Some(dir) = doc.layer_folders.get(&l_idx) {
                            let count = self.get_dir_image_count(dir);
                            ui.label(egui::RichText::new(format!("📦 {}", count)).color(egui::Color32::GRAY));
                            if ui.selectable_label(false, "🔄").clicked() {
                                self.cached_dir_counts.remove(dir);
                            }
                        }
                    }
                });

                let mut slider_frame = self.current_frame as u32 + 1;
                if ui.add(egui::Slider::new(&mut slider_frame, 1..=total_frames as u32).text(text.player_timeline)).changed() {
                    self.current_frame = (slider_frame - 1) as usize;
                    self.playing = false; 
                    let current_layer = doc.selection_state.selected_cell.map(|(l, _)| l).unwrap_or(0);
                    doc.selection_state.selected_cell = Some((current_layer, self.current_frame));
                }

                ui.separator();

                let mut loaded_path = None;
                let mut error_msg = None;
                let mut current_drawing_val = 0; 

                if self.preview_mode == -1 {
                    if let Some(dir) = &doc.reference_image_dir {
                        let file_name = format!("frame_{:04}.png", self.current_frame + 1);
                        let path = dir.join(&file_name);
                        if path.exists() {
                            loaded_path = Some(path);
                            current_drawing_val = self.current_frame as u32 + 1;
                        } else {
                            error_msg = Some(format!("Image not found: {}", file_name));
                        }
                    } else {
                        error_msg = Some("No Video Imported.".to_string());
                    }
                } else {
                    let l_idx = self.preview_mode as usize;
                    if let Some(dir) = doc.layer_folders.get(&l_idx) {
                        if let Some(val) = doc.timesheet.get_actual_value(l_idx, self.current_frame) {
                            if let Some(path) = find_image_for_value(dir, val) {
                                loaded_path = Some(path);
                                current_drawing_val = val; 
                            } else {
                                error_msg = Some(format!("Missing drawing {} in folder", val));
                            }
                        } else {
                            error_msg = Some("Blank frame".to_string());
                        }
                    } else {
                        error_msg = Some("No image folder set for this layer.".to_string());
                    }
                }

                if let Some(path) = loaded_path {
                    let uri = format!("file://{}", path.display());
                    let available_size = ui.available_size();
                    let target_size = available_size.max(egui::vec2(200.0, 150.0));
                    
                    let response = ui.add(
                        egui::Image::new(&uri)
                            .max_width(target_size.x)
                            .max_height(target_size.y)
                            .maintain_aspect_ratio(true)
                    );

                    if self.show_frame_number {
                        let display_text = format!("{}", current_drawing_val);
                        let painter = ui.painter();
                        let font_id = egui::FontId::proportional(28.0);
                        let text_color = egui::Color32::from_rgb(50, 255, 50);

                        let text_rect = painter.layout_no_wrap(
                            display_text.clone(),
                            font_id.clone(),
                            text_color,
                        );

                        let top_right = response.rect.right_top();
                        let padding = egui::vec2(-10.0, 10.0);
                        let text_pos = top_right + padding - egui::vec2(text_rect.rect.width(), 0.0);

                        let bg_rect = egui::Rect::from_min_size(
                            text_pos - egui::vec2(6.0, 2.0),
                            text_rect.rect.size() + egui::vec2(12.0, 4.0),
                        );
                        painter.rect_filled(bg_rect, 4.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 160));
                        painter.text(text_pos, egui::Align2::LEFT_TOP, display_text, font_id, text_color);
                    }

                } else if let Some(msg) = error_msg {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new(msg).color(egui::Color32::GRAY));
                    });
                }
            });

        doc.show_player = is_open;
        
        bake_result
    }
}