use egui::{vec2, Margin, Rounding, Stroke, Window};
use crate::components::avatar::draw_avatar;
use crate::components::qr_view::draw_qr_code;
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_profile_modal(ctx: &egui::Context, state: &mut AppState, theme: &Theme) {
    if !state.modals.show_profile {
        return;
    }

    let mut is_open = true;
    let mut should_close = false;
    let screen_center = ctx.screen_rect().center();

    Window::new("Мой профиль и QR-визитка")
        .open(&mut is_open)
        .resizable(true)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_size(vec2(400.0, 520.0))
        .min_size(vec2(340.0, 420.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                draw_avatar(ui, &state.my_node_id, &state.my_name, 64.0, true);
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(&state.my_name)
                        .size(17.0)
                        .strong()
                        .color(theme.text_primary),
                );
                ui.label(
                    egui::RichText::new("✓ Подключен к P2P Kademlia DHT")
                        .size(11.0)
                        .color(theme.online_indicator),
                );

                ui.add_space(14.0);

                // Render QR code of the user's Invite Link
                let invite_link = format!("nodex://invite/{}", state.my_node_id);
                draw_qr_code(ui, &invite_link, 170.0);

                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("Отсканируйте QR или отправьте Invite-ссылку другу:")
                        .size(11.0)
                        .color(theme.text_muted),
                );

                ui.add_space(8.0);

                // Node ID hex box with copy button
                egui::Frame::none()
                    .fill(theme.bubble_other_bg)
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let display_id = crate::theme::truncate_id(&state.my_node_id, 8, 8);
                            ui.monospace(
                                egui::RichText::new(display_id)
                                    .size(11.0)
                                    .color(theme.accent),
                            );

                            if ui.button(egui::RichText::new("[ Скопировать ID ]").size(10.5)).clicked() {
                                ui.output_mut(|o| o.copied_text = state.my_node_id.clone());
                                state.audio_player.play_sent_sound();
                            }
                        });
                    });

                ui.add_space(8.0);

                if ui.button(egui::RichText::new("📋 Скопировать Invite-ссылку").strong().color(theme.accent)).clicked() {
                    ui.output_mut(|o| o.copied_text = invite_link.clone());
                    state.audio_player.play_sent_sound();
                }

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("[ Новый аккаунт ]").size(11.0).color(theme.accent)).clicked() {
                        state.onboarding_step = 1;
                        state.is_onboarded = false;
                        state.my_name.clear();
                        state.my_bio.clear();
                        state.mnemonic_seed = nodex_messenger::mnemonic::MnemonicManager::generate_24_words().unwrap_or_default();
                        state.seed_confirmed_saved = false;
                        should_close = true;
                    }

                    if ui.button(egui::RichText::new("[ Восстановить ]").size(11.0).color(theme.text_primary)).clicked() {
                        state.onboarding_step = 3;
                        state.is_onboarded = false;
                        state.restore_input.clear();
                        state.restore_error = None;
                        should_close = true;
                    }

                    if ui.button(egui::RichText::new("[ Закрыть ]").color(theme.text_muted)).clicked() {
                        should_close = true;
                    }
                });
            });
        });

    if !is_open || should_close {
        state.modals.show_profile = false;
    }
}
