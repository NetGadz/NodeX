use egui::{vec2, Align2, Color32, Margin, Rounding, ScrollArea, Stroke, Window};
use crate::state::AppState;
use crate::theme::Theme;

pub fn render_mnemonic_modal(
    ctx: &egui::Context,
    state: &mut AppState,
    theme: &Theme,
) {
    if !state.modals.show_mnemonic_modal {
        return;
    }

    let mut is_open = true;
    let screen_center = ctx.screen_rect().center();

    Window::new("🔐 Резервная копия и Seed-фраза")
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(Align2::CENTER_CENTER)
        .default_size(vec2(540.0, 520.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(16.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .inner_margin(Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔐").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Секретная фраза восстановления")
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            egui::RichText::new("Используйте эти слова для восстановления доступа к аккаунту")
                                .size(11.5)
                                .color(theme.text_muted),
                        );
                    });
                });

                ui.add_space(14.0);

                // Security Recommendation Card (Password recommendation)
                egui::Frame::none()
                    .fill(Color32::from_rgba_premultiplied(59, 130, 246, 25))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(59, 130, 246)))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🛡️").size(22.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Рекомендация по безопасности: Установите пароль")
                                        .strong()
                                        .size(12.5)
                                        .color(Color32::from_rgb(147, 197, 253)),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Для защиты сессии и секретных ключей от посторонних лиц на этом устройстве \
                                        настоятельно рекомендуется включить мастер-пароль приложения. \
                                        Все локальные базы и ключи шифруются ChaCha20-Poly1305."
                                    )
                                    .size(11.0)
                                    .color(theme.text_primary),
                                );

                                ui.add_space(6.0);
                                if state.passcode_hash.is_some() {
                                    ui.label(
                                        egui::RichText::new("✓ Мастер-пароль активен и защищает сессию")
                                            .size(11.0)
                                            .color(theme.online_indicator)
                                            .strong(),
                                    );
                                } else {
                                    let set_pwd_btn = egui::Button::new(
                                        egui::RichText::new("🔒 Установить пароль приложения")
                                            .size(11.5)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(37, 99, 235))
                                    .rounding(Rounding::same(8.0));

                                    if ui.add(set_pwd_btn).clicked() {
                                        state.show_passcode_setup = true;
                                    }
                                }
                            });
                        });
                    });

                ui.add_space(14.0);

                // Words Length Selector (12 or 24)
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Длина фразы:").size(12.0).color(theme.text_muted));

                    let is_12 = state.mnemonic_length == 12;
                    let is_24 = state.mnemonic_length == 24;

                    if ui.selectable_label(is_12, "12 слов").clicked() {
                        state.mnemonic_length = 12;
                        if let Ok(m) = nodex_messenger::mnemonic::MnemonicManager::generate_12_words() {
                            state.mnemonic_seed = m;
                        }
                    }
                    if ui.selectable_label(is_24, "24 слова").clicked() {
                        state.mnemonic_length = 24;
                        if let Ok(m) = nodex_messenger::mnemonic::MnemonicManager::generate_24_words() {
                            state.mnemonic_seed = m;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("🔄 Сгенерировать новую").size(11.0).color(theme.accent)).clicked() {
                            let new_m = if state.mnemonic_length == 12 {
                                nodex_messenger::mnemonic::MnemonicManager::generate_12_words()
                            } else {
                                nodex_messenger::mnemonic::MnemonicManager::generate_24_words()
                            };
                            if let Ok(m) = new_m {
                                state.mnemonic_seed = m;
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // Word Chips Grid
                let words: Vec<&str> = state.mnemonic_seed.split_whitespace().collect();
                egui::Frame::none()
                    .fill(theme.bubble_other_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .max_height(200.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                egui::Grid::new("mnemonic_words_grid")
                                    .num_columns(3)
                                    .spacing([12.0, 8.0])
                                    .min_col_width(140.0)
                                    .show(ui, |ui| {
                                        for (i, word) in words.iter().enumerate() {
                                            egui::Frame::none()
                                                .fill(theme.card_bg)
                                                .stroke(Stroke::new(1.0_f32, theme.border_color))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(10.0, 6.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!("{:02}", i + 1))
                                                                .size(10.5)
                                                                .color(theme.text_muted),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(*word)
                                                                .size(12.5)
                                                                .strong()
                                                                .color(theme.text_primary),
                                                        );
                                                    });
                                                });

                                            if (i + 1) % 3 == 0 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                    });

                ui.add_space(16.0);

                // Bottom Controls
                ui.horizontal(|ui| {
                    let copy_btn = egui::Button::new(
                        egui::RichText::new("📋 Скопировать всю фразу")
                            .size(12.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(theme.accent)
                    .rounding(Rounding::same(8.0));

                    if ui.add_sized(vec2(200.0, 36.0), copy_btn).clicked() {
                        ui.output_mut(|o| o.copied_text = state.mnemonic_seed.clone());
                        state.audio_player.play_sent_sound();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn = egui::Button::new(
                            egui::RichText::new("Закрыть")
                                .size(12.0)
                                .color(theme.text_primary),
                        )
                        .fill(theme.bubble_other_bg)
                        .stroke(Stroke::new(1.0_f32, theme.border_color))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(vec2(100.0, 36.0), close_btn).clicked() {
                            state.modals.show_mnemonic_modal = false;
                        }
                    });
                });
            });
        });

    if !is_open {
        state.modals.show_mnemonic_modal = false;
    }
}
