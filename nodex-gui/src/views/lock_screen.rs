use egui::{vec2, Color32, Margin, Rounding, Stroke, Ui};
use sha2::{Digest, Sha256};
use crate::components::avatar::draw_avatar;
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn hash_passcode(passcode: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nodex_app_passcode_salt_");
    hasher.update(passcode.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_passcode(passcode: &str, expected_hash: &str) -> bool {
    hash_passcode(passcode) == expected_hash
}

pub fn render_lock_screen(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);

        egui::Frame::none()
            .fill(theme.card_bg)
            .rounding(Rounding::ZERO)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .inner_margin(Margin::same(32.0))
            .show(ui, |ui| {
                ui.set_width(380.0);

                ui.label(
                    egui::RichText::new("LOCKED")
                        .size(20.0)
                        .strong()
                        .color(theme.text_primary),
                );

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Enter passcode to unlock NodeX session")
                        .size(11.5)
                        .color(theme.text_muted),
                );

                ui.add_space(16.0);

                // User avatar & display name
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 140.0).max(0.0) / 2.0);
                    draw_avatar(ui, &state.my_node_id, &state.my_name, 36.0, true);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&state.my_name).strong().size(13.5).color(theme.text_primary));
                        ui.label(egui::RichText::new("Session Active").size(10.5).color(theme.online_indicator));
                    });
                });

                ui.add_space(18.0);

                // Passcode text edit
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.passcode_input)
                        .password(true)
                        .hint_text("Passcode")
                        .desired_width(f32::INFINITY),
                );

                let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if let Some(err) = &state.passcode_error {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(err).size(11.0).color(Color32::from_rgb(231, 76, 60)));
                }

                ui.add_space(14.0);

                let unlock_btn = egui::Button::new(
                    egui::RichText::new("[ Unlock ]")
                        .size(13.0)
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(theme.accent)
                .rounding(Rounding::ZERO);

                let width = ui.available_width();
                let clicked = ui.add_sized(vec2(width, 36.0), unlock_btn).clicked();

                if (clicked || enter_pressed) && !state.passcode_input.is_empty() {
                    let entered_hash = hash_passcode(&state.passcode_input);
                    if let Some(expected_hash) = &state.passcode_hash {
                        if &entered_hash == expected_hash {
                            state.is_app_locked = false;
                            state.passcode_input.clear();
                            state.passcode_error = None;
                            state.last_active = std::time::Instant::now();
                        } else {
                            state.passcode_error = Some("⚠ Incorrect passcode. Please try again.".to_string());
                            state.passcode_input.clear();
                        }
                    } else {
                        // No passcode configured -> unlock
                        state.is_app_locked = false;
                        state.passcode_input.clear();
                        state.passcode_error = None;
                        state.last_active = std::time::Instant::now();
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(10.0);

                // Forgot passcode button -> allows entering 12/24 word seed to unlock
                if ui.button(egui::RichText::new("🔑 Forgot Passcode? Restore with Seed Phrase").size(11.5).color(theme.text_muted)).clicked() {
                    state.onboarding_step = 3; // Go to restore step
                    state.is_onboarded = false;
                    state.is_app_locked = false;
                    state.passcode_input.clear();
                    state.passcode_error = None;
                }
            });
    });
}
