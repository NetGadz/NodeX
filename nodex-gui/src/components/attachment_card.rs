use egui::{Color32, Ui, Rounding, Stroke};
use crate::theme::{BG_CARD, TEXT_PRIMARY, TEXT_MUTED};

pub fn draw_attachment_card(ui: &mut Ui, file_name: &str, file_size: usize, is_outgoing: bool) {
    let bg_color = if is_outgoing {
        Color32::from_rgb(35, 68, 100)
    } else {
        BG_CARD
    };

    egui::Frame::none()
        .fill(bg_color)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 20)))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📁").size(24.0));

                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(file_name).strong().color(TEXT_PRIMARY).size(13.0));
                    
                    let size_str = if file_size >= 1024 * 1024 {
                        format!("{:.1} MB", file_size as f32 / (1024.0 * 1024.0))
                    } else {
                        format!("{} KB", file_size / 1024)
                    };
                    
                    ui.label(egui::RichText::new(size_str).color(TEXT_MUTED).size(11.0));
                });

                ui.add_space(8.0);

                if ui.button(egui::RichText::new("💾 Save").size(12.0)).clicked() {
                    if let Some(dest) = rfd::FileDialog::new().set_file_name(file_name).save_file() {
                        println!("[GUI] User selected save location: {:?}", dest);
                    }
                }
            });
        });
}
