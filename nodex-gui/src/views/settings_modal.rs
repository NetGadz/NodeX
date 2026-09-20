use egui::{vec2, Align2, Color32, Margin, Rounding, ScrollArea, Stroke, Vec2, Window};
use crate::components::avatar::draw_avatar;
use crate::components::qr_view::draw_qr_code;
use crate::state::{AppState, ThemeMode};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsCategory {
    Account,
    Sessions,
    Notifications,
    Privacy,
    Chat,
    Folders,
    Network,
}

pub fn render_settings_modal(
    ctx: &egui::Context,
    state: &mut AppState,
    theme: &Theme,
    messenger: &std::sync::Arc<nodex_messenger::KadMessenger>,
) {
    if !state.modals.show_settings {
        return;
    }

    let mut is_open = true;
    let mut should_close = false;
    let mut current_tab = SettingsCategory::Account;

    ctx.data_mut(|d| {
        if let Some(tab) = d.get_temp::<SettingsCategory>(egui::Id::new("settings_active_category")) {
            current_tab = tab;
        }
    });

    let screen_center = ctx.screen_rect().center();

    Window::new("Настройки NodeX")
        .open(&mut is_open)
        .resizable(true)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(Align2::CENTER_CENTER)
        .default_size(vec2(600.0, 460.0))
        .min_size(vec2(480.0, 360.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .inner_margin(Margin::same(12.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // LEFT SIDEBAR: CATEGORIES
                ui.allocate_ui(vec2(165.0, ui.available_height()), |ui| {
                    ui.vertical(|ui| {
                        let categories = [
                            (SettingsCategory::Account, "👤  Мой аккаунт"),
                            (SettingsCategory::Sessions, "📱  Устройства и сессии"),
                            (SettingsCategory::Notifications, "🔔  Уведомления и звуки"),
                            (SettingsCategory::Privacy, "🔒  Конфиденциальность"),
                            (SettingsCategory::Chat, "💬  Настройки чатов"),
                            (SettingsCategory::Folders, "📁  Папки с чатами"),
                            (SettingsCategory::Network, "🌐  Сеть и DHT"),
                        ];

                        for (cat, label) in categories {
                            let is_active = current_tab == cat;
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(11.5)
                                    .strong()
                                    .color(if is_active { Color32::WHITE } else { theme.text_primary }),
                            )
                            .fill(if is_active { theme.accent } else { Color32::TRANSPARENT })
                            .rounding(Rounding::same(6.0))
                            .stroke(if is_active { Stroke::NONE } else { Stroke::new(1.0_f32, theme.border_color) });

                            if ui.add_sized(vec2(ui.available_width(), 28.0), btn).clicked() {
                                current_tab = cat;
                                ctx.data_mut(|d| d.insert_temp(egui::Id::new("settings_active_category"), cat));
                            }
                            ui.add_space(2.0);
                        }
                    });
                });

                ui.separator();
                ui.add_space(4.0);

                // RIGHT CONTENT AREA - VERTICAL COLUMN
                ui.vertical(|ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                match current_tab {
                                    SettingsCategory::Account => {
                                        render_account_tab(ui, state, theme, messenger);
                                    }
                                    SettingsCategory::Sessions => {
                                        render_sessions_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Notifications => {
                                        render_notifications_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Privacy => {
                                        render_privacy_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Chat => {
                                        render_chat_tab(ui, state, theme, ctx);
                                    }
                                    SettingsCategory::Folders => {
                                        render_folders_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Network => {
                                        render_network_tab(ui, state, theme);
                                    }
                                }

                                ui.add_space(10.0);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let done_btn = egui::Button::new(
                                        egui::RichText::new("Закрыть").size(11.5).strong().color(Color32::WHITE),
                                    )
                                    .fill(theme.accent)
                                    .rounding(Rounding::same(6.0));

                                    if ui.add_sized(Vec2::new(75.0, 24.0), done_btn).clicked() {
                                        should_close = true;
                                    }
                                });
                            });
                        });
                });
            });
        });

    if !is_open || should_close {
        state.modals.show_settings = false;
    }
}

fn render_account_tab(
    ui: &mut egui::Ui,
    state: &mut AppState,
    theme: &Theme,
    messenger: &std::sync::Arc<nodex_messenger::KadMessenger>,
) {
    // Compact Profile Header
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                draw_avatar(ui, &state.my_node_id, &state.my_name, 32.0, true);
                ui.add_space(6.0);
                crate::components::avatar::draw_crypto_identicon(ui, &state.my_node_id, 32.0);
                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&state.my_name)
                            .size(13.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let display_id = crate::theme::truncate_id(&state.my_node_id, 6, 6);
                    ui.monospace(egui::RichText::new(format!("ID: {}", display_id)).size(10.0).color(theme.accent));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Скопировать ID").size(10.5)).clicked() {
                        ui.output_mut(|o| o.copied_text = state.my_node_id.clone());
                        state.audio_player.play_sent_sound();
                    }
                });
            });
        });

    ui.add_space(6.0);

    // Compact Form Inputs
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Имя:").size(11.0).color(theme.text_muted));
                ui.add(egui::TextEdit::singleline(&mut state.my_name).desired_width(120.0));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Bio:").size(11.0).color(theme.text_muted));
                ui.add(egui::TextEdit::singleline(&mut state.my_bio).desired_width(f32::INFINITY));
            });
        });

    ui.add_space(6.0);

    // Stego-Avatar & Invite Link Section
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let invite_link = format!("nodex://invite/{}", state.my_node_id);
                if ui.button(egui::RichText::new("📋 Скопировать Invite").size(10.5).strong().color(theme.accent)).clicked() {
                    ui.output_mut(|o| o.copied_text = invite_link.clone());
                    state.audio_player.play_sent_sound();
                }

                if ui.button(egui::RichText::new("🖼 Экспорт Стего-PNG").size(10.5)).clicked() {
                    if let Some(src_path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                        .pick_file()
                    {
                        if let Ok(src_bytes) = std::fs::read(&src_path) {
                            if let Some(save_path) = rfd::FileDialog::new()
                                .set_file_name("my_stego_avatar.png")
                                .add_filter("PNG Image", &["png"])
                                .save_file()
                            {
                                let msgr = messenger.clone();
                                std::thread::spawn(move || {
                                    if let Ok(rt) = tokio::runtime::Runtime::new() {
                                        if let Ok(png_bytes) = rt.block_on(async move {
                                            msgr.export_stego_avatar(&src_bytes, None).await
                                        }) {
                                            let _ = std::fs::write(&save_path, &png_bytes);
                                        }
                                    }
                                });
                                state.audio_player.play_sent_sound();
                            }
                        }
                    }
                }
            });
        });

    ui.add_space(6.0);

    // Seed phrase reveal
    if !state.reveal_seed_in_settings {
        if ui.button(egui::RichText::new("🔑 Показать Seed-фразу (24 слова)").size(10.5)).clicked() {
            state.reveal_seed_in_settings = true;
        }
    } else {
        egui::Frame::none()
            .fill(theme.bubble_other_bg)
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.0_f32, theme.border_color))
            .inner_margin(Margin::same(6.0))
            .show(ui, |ui| {
                let words: Vec<&str> = state.mnemonic_seed.split_whitespace().collect();
                egui::Grid::new("seed_grid_settings").num_columns(4).spacing(vec2(4.0, 2.0)).show(ui, |ui| {
                    for (i, w) in words.iter().enumerate() {
                        ui.label(egui::RichText::new(format!("{:02}. {}", i + 1, w)).size(9.5).color(theme.accent));
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });
            });
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new("[ Копировать ]").size(10.0)).clicked() {
                ui.output_mut(|o| o.copied_text = state.mnemonic_seed.clone());
            }
            if ui.button(egui::RichText::new("[ Скрыть ]").size(10.0)).clicked() {
                state.reveal_seed_in_settings = false;
            }
        });
    }
}

fn render_sessions_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Устройства и сессии").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    // Current Session
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.online_indicator))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💻").size(16.0));
                ui.label(egui::RichText::new("Windows Desktop (Текущее)").strong().size(11.5).color(theme.text_primary));
                ui.label(egui::RichText::new(format!("UDP {}", state.port)).size(10.0).color(theme.text_muted));
            });
        });

    ui.add_space(6.0);

    // Link by QR code
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("🔗  Спаривание по QR").strong().size(11.5).color(theme.accent));

            if state.session_pair_token.is_empty() {
                state.session_pair_token = format!("nodex://pair/{}/{}", state.my_node_id, rand::random::<u32>());
            }

            ui.vertical_centered(|ui| {
                draw_qr_code(ui, &state.session_pair_token, 110.0);
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("[ Новый токен ]").size(10.0)).clicked() {
                    state.session_pair_token = format!("nodex://pair/{}/{}", state.my_node_id, rand::random::<u32>());
                }
                if ui.button(egui::RichText::new("[ Копировать ]").size(10.0)).clicked() {
                    ui.output_mut(|o| o.copied_text = state.session_pair_token.clone());
                }
            });
        });
}

fn render_notifications_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Звуки и оповещения").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.checkbox(&mut state.notifications_enabled, "Звуки сообщений");
            ui.checkbox(&mut state.ringtone_enabled, "Звук входящего вызова");
            ui.checkbox(&mut state.zen_mode, "Режим «Не беспокоить»");

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            let vol_pct = (state.sound_volume * 100.0) as u32;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("🔊 Громкость: {}%", vol_pct)).size(11.0).color(theme.text_primary));
                if ui.button(egui::RichText::new(" - ").size(10.0)).clicked() {
                    state.sound_volume = (state.sound_volume - 0.1).clamp(0.0, 1.0);
                }
                if ui.button(egui::RichText::new(" + ").size(10.0)).clicked() {
                    state.sound_volume = (state.sound_volume + 0.1).clamp(0.0, 1.0);
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("▶ Тест звука").size(10.0)).clicked() {
                    state.audio_player.play_sent_sound();
                }
                if ui.button(egui::RichText::new("📞 Рингтон").size(10.0)).clicked() {
                    state.audio_player.play_ringtone_loop();
                }
                if ui.button(egui::RichText::new("⏹ Стоп").size(10.0)).clicked() {
                    state.audio_player.stop();
                }
            });
        });
}

fn render_privacy_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Конфиденциальность").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.checkbox(&mut state.onion_routing_enabled, "Blind 2-Hop Onion маршрутизация (сокрытие IP)");

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(egui::RichText::new("🔐 Локальный PIN-код").size(11.5).strong().color(theme.text_primary));
            if state.passcode_hash.is_some() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("✓ PIN активен").size(10.5).color(theme.online_indicator).strong());
                    if ui.button(egui::RichText::new("[ Заблокировать ]").size(10.0)).clicked() {
                        state.is_app_locked = true;
                        state.modals.show_settings = false;
                    }
                    if ui.button(egui::RichText::new("[ Снять PIN ]").size(10.0).color(Color32::from_rgb(231, 76, 60))).clicked() {
                        state.passcode_hash = None;
                    }
                });
            } else {
                if ui.button(egui::RichText::new("[ Установить PIN (1234) ]").size(10.0)).clicked() {
                    state.passcode_hash = Some(crate::views::lock_screen::hash_passcode("1234"));
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Автоблокировка:").size(11.0).color(theme.text_muted));
                for (mins, label) in [(0, "Выкл"), (1, "1м"), (5, "5м"), (15, "15м")] {
                    let is_active = state.auto_lock_mins == mins;
                    let btn = egui::Button::new(
                        egui::RichText::new(label).size(10.0).color(if is_active { Color32::WHITE } else { theme.text_muted }),
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(4.0));

                    if ui.add(btn).clicked() {
                        state.auto_lock_mins = mins;
                    }
                }
            });
        });
}

fn render_chat_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme, ctx: &egui::Context) {
    ui.label(egui::RichText::new("Вид и Настройки чатов").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            // Theme selection with crystal-clear high-contrast buttons
            ui.label(egui::RichText::new("Тема оформления:").size(11.0).strong().color(theme.text_primary));
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let modes = [
                    (ThemeMode::Dark, "🌙 Dark"),
                    (ThemeMode::Midnight, "⬛ Midnight"),
                    (ThemeMode::Day, "☀️ Day Light"),
                ];
                for (mode, label) in modes {
                    let is_active = state.theme_mode == mode;
                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .size(10.5)
                            .strong()
                            .color(if is_active { Color32::WHITE } else { theme.text_muted }),
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .stroke(if is_active { Stroke::NONE } else { Stroke::new(1.0_f32, theme.border_color) })
                    .rounding(Rounding::same(5.0));

                    if ui.add(btn).clicked() {
                        state.theme_mode = mode;
                        let new_theme = Theme::from_mode(mode);
                        new_theme.apply_to_ctx(ctx);
                    }
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(egui::RichText::new("Размер шрифта:").size(11.0).strong().color(theme.text_primary));
            ui.horizontal(|ui| {
                for (size, label) in [(12.0_f32, "12px"), (13.5_f32, "13.5px"), (15.5_f32, "15.5px")] {
                    let is_active = (state.chat_font_size - size).abs() < 0.1;
                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .size(10.0)
                            .color(if is_active { Color32::WHITE } else { theme.text_muted }),
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(4.0));

                    if ui.add(btn).clicked() {
                        state.chat_font_size = size;
                    }
                }
            });

            ui.add_space(6.0);
            ui.checkbox(&mut state.send_on_enter, "Отправка по Enter (Shift+Enter - перенос)");
            ui.checkbox(&mut state.compact_bubbles, "Компактные пузыри сообщений");
            ui.checkbox(&mut state.auto_download_media, "Автозагрузка медиа");
        });
}

fn render_folders_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Папки с чатами").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    let folders = [
        ("📁 Все чаты", state.folder_filter == crate::state::FolderFilter::All),
        ("👤 Личные", state.folder_filter == crate::state::FolderFilter::Direct),
        ("👥 Группы", state.folder_filter == crate::state::FolderFilter::Groups),
        ("📬 Непрочитанные", state.folder_filter == crate::state::FolderFilter::Unread),
    ];

    for (title, is_selected) in folders {
        egui::Frame::none()
            .fill(if is_selected { theme.bubble_other_bg } else { theme.card_bg })
            .rounding(Rounding::same(5.0))
            .stroke(Stroke::new(1.0_f32, if is_selected { theme.accent } else { theme.border_color }))
            .inner_margin(Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(title).size(11.0).strong().color(theme.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_selected {
                            ui.label(egui::RichText::new("✓").color(theme.online_indicator).strong());
                        } else {
                            let f = if title.contains("Личные") {
                                crate::state::FolderFilter::Direct
                            } else if title.contains("Группы") {
                                crate::state::FolderFilter::Groups
                            } else if title.contains("Непрочитанные") {
                                crate::state::FolderFilter::Unread
                            } else {
                                crate::state::FolderFilter::All
                            };
                            if ui.button(egui::RichText::new("Выбрать").size(9.5)).clicked() {
                                state.folder_filter = f;
                            }
                        }
                    });
                });
            });
        ui.add_space(2.0);
    }
}

fn render_network_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Сеть и P2P DHT").size(13.0).strong().color(theme.text_primary));
    ui.add_space(4.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Relay Node:").size(11.0).color(theme.text_primary));
                let relay_text = if state.network_stats.relay_active { "[ Вкл ]" } else { "[ Выкл ]" };
                let relay_color = if state.network_stats.relay_active { theme.accent } else { theme.text_muted };
                if ui.button(egui::RichText::new(relay_text).monospace().strong().color(relay_color)).clicked() {
                    state.network_stats.relay_active = !state.network_stats.relay_active;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("UPnP Port Mapping:").size(11.0).color(theme.text_primary));
                let upnp_text = if state.network_stats.upnp_active { "[ Вкл ]" } else { "[ Выкл ]" };
                let upnp_color = if state.network_stats.upnp_active { theme.accent } else { theme.text_muted };
                if ui.button(egui::RichText::new(upnp_text).monospace().strong().color(upnp_color)).clicked() {
                    state.network_stats.upnp_active = !state.network_stats.upnp_active;
                }
            });

            ui.add_space(6.0);
            ui.monospace(egui::RichText::new(format!("DHT Nodes: {} • NAT: {}", state.network_stats.known_nodes, state.network_stats.nat_type)).size(10.0).color(theme.text_muted));
        });
}
