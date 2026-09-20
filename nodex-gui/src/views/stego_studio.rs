use egui::{Color32, FontId, RichText, Rounding, ScrollArea, Stroke, Ui, Vec2};
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_stego_studio(
    ui: &mut Ui,
    state: &mut AppState,
    theme: &Theme,
    messenger: &std::sync::Arc<nodex_messenger::KadMessenger>,
) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(16.0);

            // Studio Header
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Stego-Carrier Studio")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Embed cryptographic identities into photos and memes. Serverless & Untraceable.")
                        .font(FontId::proportional(12.5))
                        .color(theme.text_muted),
                );
            });

            ui.add_space(20.0);

            // Notification / Status banner if any
            if let Some(ref msg) = state.stego_status_msg {
                let banner_color = if msg.starts_with("✅") || msg.starts_with("🎉") {
                    theme.online_indicator
                } else {
                    Color32::from_rgb(255, 107, 107)
                };

                ui.vertical_centered(|ui| {
                    egui::Frame::none()
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, banner_color))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(msg).color(banner_color).strong());
                        });
                });
                ui.add_space(14.0);
            }

            ui.horizontal(|ui| {
                let half_width = (ui.available_width() - 20.0) / 2.0;

                // --- CARD 1: IMPORT CONTACT FROM IMAGE ---
                ui.allocate_ui(Vec2::new(half_width, ui.available_height()), |ui| {
                    egui::Frame::none()
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Import Stego-Image")
                                    .font(FontId::proportional(15.0))
                                    .strong()
                                    .color(theme.accent),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Extract encrypted P2P peer coordinates from an image.")
                                    .font(FontId::proportional(11.5))
                                    .color(theme.text_muted),
                            );
                            ui.add_space(14.0);

                            ui.label(RichText::new("Passphrase (optional):").size(11.5).color(theme.text_primary));
                            ui.add_space(4.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut state.stego_passphrase)
                                    .password(true)
                                    .hint_text("Leave blank if none")
                                    .desired_width(f32::INFINITY),
                            );

                            ui.add_space(16.0);

                            // Upload Button
                            let btn_resp = ui.add_sized(
                                [ui.available_width(), 36.0],
                                egui::Button::new(RichText::new("[ Browse & Decode Image ]").font(FontId::proportional(13.0)).strong())
                                    .fill(theme.bubble_outgoing_bg)
                                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                                    .rounding(Rounding::same(8.0)),
                            );

                            if btn_resp.clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                                    .pick_file()
                                {
                                    if let Ok(bytes) = std::fs::read(&path) {
                                        let pass_opt = if state.stego_passphrase.trim().is_empty() {
                                            None
                                        } else {
                                            Some(state.stego_passphrase.trim())
                                        };

                                        if let Ok(rt) = tokio::runtime::Runtime::new() {
                                            let m_clone = messenger.clone();
                                            let pass_str = pass_opt.map(|s| s.to_string());

                                            match rt.block_on(async move {
                                                m_clone.import_stego_avatar(&bytes, pass_str.as_deref()).await
                                            }) {
                                                Ok(contact) => {
                                                    state.audio_player.play_sent_sound();
                                                    state.stego_status_msg = Some(format!(
                                                        "🎉 Added contact '{}' ({}) via Stego-Carrier!",
                                                        contact.name,
                                                        &contact.user_id_hex[..8]
                                                    ));
                                                }
                                                Err(e) => {
                                                    state.stego_status_msg = Some(format!("❌ Import failed: {}", e));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                });

                ui.add_space(8.0);

                // --- CARD 2: EXPORT STEGO AVATAR / MEME ---
                ui.allocate_ui(Vec2::new(half_width, ui.available_height()), |ui| {
                    egui::Frame::none()
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Create Stego-Carrier Image")
                                    .font(FontId::proportional(15.0))
                                    .strong()
                                    .color(Color32::from_rgb(168, 85, 247)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Embed your signed network identity into an image.")
                                    .font(FontId::proportional(11.5))
                                    .color(theme.text_muted),
                            );
                            ui.add_space(14.0);

                            if let Some(ref path) = state.stego_export_image_path {
                                ui.label(RichText::new(format!("Source: {}", path)).size(11.0).color(theme.accent));
                            } else {
                                ui.label(RichText::new("No image chosen yet.").size(11.0).color(theme.text_muted));
                            }

                            ui.add_space(8.0);

                            if ui.button("[ Select Source Photo ]").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                                    .pick_file()
                                {
                                    state.stego_export_image_path = Some(path.to_string_lossy().to_string());
                                }
                            }

                            ui.add_space(14.0);

                            let export_ready = state.stego_export_image_path.is_some();
                            let export_btn = ui.add_enabled(
                                export_ready,
                                egui::Button::new(RichText::new("[ Generate Stego-PNG ]").font(FontId::proportional(13.0)).strong())
                                    .min_size(Vec2::new(ui.available_width(), 36.0))
                                    .fill(Color32::from_rgb(107, 33, 168))
                                    .rounding(Rounding::same(8.0)),
                            );

                            if export_btn.clicked() {
                                if let Some(ref path_str) = state.stego_export_image_path {
                                    if let Ok(src_bytes) = std::fs::read(path_str) {
                                        let pass_opt = if state.stego_passphrase.trim().is_empty() {
                                            None
                                        } else {
                                            Some(state.stego_passphrase.trim())
                                        };

                                        if let Ok(rt) = tokio::runtime::Runtime::new() {
                                            let m_clone = messenger.clone();
                                            let pass_str = pass_opt.map(|s| s.to_string());

                                            match rt.block_on(async move {
                                                m_clone.export_stego_avatar(&src_bytes, pass_str.as_deref()).await
                                            }) {
                                                Ok(png_bytes) => {
                                                    if let Some(save_path) = rfd::FileDialog::new()
                                                        .set_file_name("nodex_stego_avatar.png")
                                                        .add_filter("PNG Image", &["png"])
                                                        .save_file()
                                                    {
                                                        if std::fs::write(&save_path, &png_bytes).is_ok() {
                                                            state.audio_player.play_sent_sound();
                                                            state.stego_status_msg = Some(format!(
                                                                "✅ Stego image saved to {}!",
                                                                save_path.file_name().unwrap_or_default().to_string_lossy()
                                                            ));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    state.stego_status_msg = Some(format!("❌ Embedding failed: {}", e));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                });
            });

            ui.add_space(30.0);
        });
}
