use egui::{vec2, Align2, Color32, Margin, Stroke, Window};
use crate::theme::Theme;

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct MediaViewerModalState {
    pub is_open: bool,
    pub title: String,
    pub image_data: Option<String>,
    pub file_info: Option<(String, usize)>,
}

pub fn render_media_viewer(
    ctx: &egui::Context,
    state: &mut MediaViewerModalState,
    _theme: &Theme,
) {
    if !state.is_open {
        return;
    }

    let mut is_open = true;

    Window::new("Media Viewer")
        .open(&mut is_open)
        .title_bar(false)
        .resizable(true)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .fixed_size(vec2(720.0, 520.0))
        .frame(
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(10, 15, 22, 245))
                .rounding(12.0)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 30)))
                .inner_margin(Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            // Header bar
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&state.title)
                        .size(16.0)
                        .strong()
                        .color(Color32::WHITE),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("✕ Close").color(Color32::WHITE)).clicked() {
                        state.is_open = false;
                    }

                    if ui.button(egui::RichText::new("💾 Save File").color(Color32::from_rgb(59, 130, 246))).clicked() {
                        if let Some(dest) = rfd::FileDialog::new().set_file_name(&state.title).save_file() {
                            println!("[GUI] Saved media to {:?}", dest);
                        }
                    }
                });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(20.0);

            // Viewer content area
            ui.vertical_centered(|ui| {
                if let Some((name, size)) = &state.file_info {
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("📄").size(64.0));
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(name)
                            .size(18.0)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} MB", *size as f32 / (1024.0 * 1024.0)))
                            .size(13.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                } else {
                    ui.add_space(80.0);
                    ui.label(egui::RichText::new("🖼 Media Preview").size(24.0).color(Color32::WHITE));
                }
            });
        });

    if !is_open {
        state.is_open = false;
    }
}
