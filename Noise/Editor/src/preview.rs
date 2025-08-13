use bevy_egui::egui;
use noise_engine::*;
use crate::EditorState;
use crate::ui_strings::UiStrings;

pub fn preview_ui(ui: &mut egui::Ui, state: &mut EditorState, ui_text: &UiStrings) {
    ui.heading(&ui_text.preview.title);

    ui.horizontal(|ui| {
        ui.label(&ui_text.preview.resolution);
        ui.add(egui::Slider::new(&mut state.preview_w, 32..=1024).text(&ui_text.preview.width_short));
        ui.add(egui::Slider::new(&mut state.preview_h, 32..=1024).text(&ui_text.preview.height_short));
    });

    ui.horizontal(|ui| {
        ui.label(&ui_text.preview.channel);
        egui::ComboBox::from_label("")
            .selected_text(match state.preview_channel {
                0 => ui_text.preview.r.clone(),
                1 => ui_text.preview.g.clone(),
                2 => ui_text.preview.b.clone(),
                _ => ui_text.preview.r.clone(),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.preview_channel, 0, &ui_text.preview.r);
                ui.selectable_value(&mut state.preview_channel, 1, &ui_text.preview.g);
                ui.selectable_value(&mut state.preview_channel, 2, &ui_text.preview.b);
            });
    });

    ui.separator();

    // Open popup window button
    if ui.button(&ui_text.preview.open_window).clicked() {
        state.show_preview_window = true;
    }

    if ui.button(&ui_text.preview.generate).clicked() {
        if let Some(engine) = &mut state.engine {
            let w = state.preview_w.max(16) as u32;
            let h = state.preview_h.max(16) as u32;
            let req = RegionRequest { origin: [0, 0, 0], size: [w, h, 1], lod: 0 };
            let spec = ChannelsSpec(vec![ChannelDesc { name: "height".into(), kind: ChannelKind::Height2D }]);
            if let Ok(res) = engine.sample_region(&req, &spec) {
                if let Some(ChannelData::Scalar2D { data, .. }) = res.channels.get(0) {
                    let mut img = egui::ColorImage::new([w as usize, h as usize], egui::Color32::BLACK);
                    for y in 0..h as usize {
                        for x in 0..w as usize {
                            let v = data[y * w as usize + x];
                            let v = ((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                            img.pixels[y * w as usize + x] = egui::Color32::from_gray(v);
                        }
                    }
                    let tex = ui.ctx().load_texture("preview", img, egui::TextureOptions::NEAREST);
                    let tex_size = tex.size_vec2();
                    let available = ui.available_size_before_wrap();
                    let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                    let draw_size = tex_size * scale;
                    ui.image(egui::load::SizedTexture::new(tex.id(), draw_size));
                }
            }
        }
    }

    // Show popup window with the same preview content if toggled
    if state.show_preview_window {
        let mut open = true;
        egui::Window::new(&ui_text.preview.window_title)
            .open(&mut open)
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(ui.ctx(), |ui| {
                if ui.button(&ui_text.preview.generate).clicked() {
                    if let Some(engine) = &mut state.engine {
                        let w = state.preview_w.max(16) as u32;
                        let h = state.preview_h.max(16) as u32;
                        let req = RegionRequest { origin: [0, 0, 0], size: [w, h, 1], lod: 0 };
                        let spec = ChannelsSpec(vec![ChannelDesc { name: "height".into(), kind: ChannelKind::Height2D }]);
                        if let Ok(res) = engine.sample_region(&req, &spec) {
                            if let Some(ChannelData::Scalar2D { data, .. }) = res.channels.get(0) {
                                let mut img = egui::ColorImage::new([w as usize, h as usize], egui::Color32::BLACK);
                                for y in 0..h as usize {
                                    for x in 0..w as usize {
                                        let v = data[y * w as usize + x];
                                        let v = ((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                                        img.pixels[y * w as usize + x] = egui::Color32::from_gray(v);
                                    }
                                }
                                let tex = ui.ctx().load_texture("preview_window", img, egui::TextureOptions::NEAREST);
                                let tex_size = tex.size_vec2();
                                let available = ui.available_size_before_wrap();
                                let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                                let draw_size = tex_size * scale;
                                ui.image(egui::load::SizedTexture::new(tex.id(), draw_size));
                            }
                        }
                    }
                }
            });
        if !open {
            state.show_preview_window = false;
        }
    }
}