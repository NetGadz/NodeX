use egui::{vec2, Align2, Color32, Margin, Rounding, Stroke, Window};
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_add_contact_modal(
    ctx: &egui::Context,
    state: &mut AppState,
    theme: &Theme,
    new_contact_out: &mut Option<(String, String)>,
    _messenger: &std::sync::Arc<nodex_messenger::KadMessenger>,
) {
    if !state.modals.show_add_contact {
        return;
    }

    let mut is_open = state.modals.show_add_contact;
    let mut add_contact_requested = false;
    let screen_center = ctx.screen_rect().center();

    Window::new("Добавить контакт (P2P DHT Поиск)")
        .open(&mut is_open)
        .resizable(true)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(Align2::CENTER_CENTER)
        .default_size(vec2(480.0, 350.0))
        .min_size(vec2(380.0, 280.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::same(18.0)),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Поиск и подключение узла через Kademlia DHT")
                    .size(13.5)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Введите публичный ID узла, вставьте Invite-ссылку или выберите Стего-аватарку:")
                    .size(11.5)
                    .color(theme.text_muted),
            );

            ui.add_space(10.0);

            ui.label(egui::RichText::new("Имя контакта (псевдоним):").size(11.5).color(theme.text_primary));
            ui.add(
                egui::TextEdit::singleline(&mut state.new_contact_alias)
                    .hint_text("Например: Алиса")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);

            ui.label(egui::RichText::new("Node ID / Invite-ссылка / Токен спаривания:").size(11.5).color(theme.text_primary));
            ui.add(
                egui::TextEdit::singleline(&mut state.new_contact_id)
                    .hint_text("40/64 hex ID или nodex://invite/...")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);

            // Stego & Quick Connect Shortcuts
            ui.horizontal(|ui| {
                let stego_btn = egui::Button::new(
                    egui::RichText::new("🖼 Загрузить Стего-Аватарку").size(11.0).color(Color32::from_rgb(168, 85, 247))
                )
                .fill(theme.bubble_other_bg)
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .rounding(Rounding::same(8.0));

                if ui.add(stego_btn).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_file()
                    {
                        if let Ok(bytes) = std::fs::read(&path) {
                            if let Ok(card) = nodex_messenger::stego::StegoCarrier::extract_contact_from_image(&bytes, None) {
                                state.new_contact_id = card.user_id_hex;
                                if state.new_contact_alias.is_empty() {
                                    state.new_contact_alias = card.display_name;
                                }
                                state.audio_player.play_sent_sound();
                            }
                        }
                    }
                }

                let share_btn = egui::Button::new(
                    egui::RichText::new("📋 Моя Invite-ссылка").size(11.0).color(theme.accent)
                )
                .fill(theme.bubble_other_bg)
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .rounding(Rounding::same(8.0));

                if ui.add(share_btn).clicked() {
                    ui.output_mut(|o| o.copied_text = format!("nodex://invite/{}", state.my_node_id));
                    state.audio_player.play_sent_sound();
                }
            });

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("[ Отмена ]").color(theme.text_muted)).clicked() {
                    state.modals.show_add_contact = false;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_add = !state.new_contact_id.trim().is_empty() && !state.new_contact_alias.trim().is_empty();
                    let btn = egui::Button::new(
                        egui::RichText::new("Добавить контакт")
                            .color(if can_add { Color32::WHITE } else { theme.text_muted })
                            .strong(),
                    )
                    .fill(if can_add { theme.accent } else { theme.bubble_other_bg })
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .rounding(Rounding::same(8.0));

                    if ui.add_enabled(can_add, btn).clicked() {
                        add_contact_requested = true;
                    }
                });
            });
        });

    if add_contact_requested {
        let mut raw_id = state.new_contact_id.trim().to_string();
        if let Some(stripped) = raw_id.strip_prefix("nodex://invite/") {
            raw_id = stripped.split('?').next().unwrap_or(stripped).to_string();
        }
        if let Some(stripped) = raw_id.strip_prefix("nodex://pair/") {
            raw_id = stripped.split('/').next().unwrap_or(stripped).to_string();
        }
        let alias = state.new_contact_alias.trim().to_string();
        if !raw_id.is_empty() && !alias.is_empty() {
            *new_contact_out = Some((raw_id, alias));
            state.new_contact_id.clear();
            state.new_contact_alias.clear();
            state.modals.show_add_contact = false;
        }
    }

    state.modals.show_add_contact = is_open;
}
