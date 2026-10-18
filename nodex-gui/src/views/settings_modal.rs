use egui::{vec2, Align2, Color32, Margin, Rounding, ScrollArea, Sense, Stroke, Vec2, Window};
use crate::components::avatar::draw_avatar;
use crate::components::qr_view::draw_qr_code;
use crate::state::{AppState, ThemeMode};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsCategory {
    Account,
    Sessions,
    Appearance,
    Notifications,
    Privacy,
    Chat,
    Folders,
    Network,
    Advanced,
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
        .default_size(vec2(660.0, 520.0))
        .min_size(vec2(520.0, 400.0))
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
                ui.allocate_ui(vec2(175.0, ui.available_height()), |ui| {
                    ui.vertical(|ui| {
                        let categories = [
                            (SettingsCategory::Account, "👤  Мой аккаунт"),
                            (SettingsCategory::Sessions, "📱  Устройства и сессии"),
                            (SettingsCategory::Appearance, "🎨  Темы и оформление"),
                            (SettingsCategory::Notifications, "🔔  Уведомления и звуки"),
                            (SettingsCategory::Privacy, "🔒  Конфиденциальность"),
                            (SettingsCategory::Chat, "💬  Настройки диалогов"),
                            (SettingsCategory::Folders, "📁  Папки с чатами"),
                            (SettingsCategory::Network, "🌐  Сеть DHT и P2P"),
                            (SettingsCategory::Advanced, "🎛  Продвинутые настройки"),
                        ];

                        for (cat, label) in categories {
                            let is_active = current_tab == cat;
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(11.0)
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
                                    SettingsCategory::Appearance => {
                                        render_appearance_tab(ui, state, theme, ctx);
                                    }
                                    SettingsCategory::Notifications => {
                                        render_notifications_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Privacy => {
                                        render_privacy_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Chat => {
                                        render_chat_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Folders => {
                                        render_folders_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Network => {
                                        render_network_tab(ui, state, theme);
                                    }
                                    SettingsCategory::Advanced => {
                                        render_advanced_tab(ui, state, theme);
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
                let invite_link = if !state.my_invite_link.is_empty() {
                    state.my_invite_link.clone()
                } else {
                    format!("nodex://invite/{}", state.my_node_id)
                };
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

    // Seed phrase reveal & Security modal
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔐").size(18.0));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Секретная Seed-фраза и защита").strong().size(12.0).color(theme.text_primary));
                    ui.label(egui::RichText::new("12 или 24 слова для восстановления и рекомендация по установке пароля").size(10.5).color(theme.text_muted));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let open_btn = egui::Button::new(
                        egui::RichText::new("Открыть").size(11.0).strong().color(Color32::WHITE)
                    )
                    .fill(theme.accent)
                    .rounding(Rounding::same(6.0));

                    if ui.add(open_btn).clicked() {
                        state.modals.show_mnemonic_modal = true;
                    }
                });
            });
        });
}

fn render_sessions_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📱 Устройства и сессии").size(14.0).strong().color(theme.text_primary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(egui::RichText::new("🔑 Seed-фраза").size(10.5).color(theme.accent)).clicked() {
                state.modals.show_mnemonic_modal = true;
            }
        });
    });
    ui.label(egui::RichText::new("Управляйте устройствами, подключенными к вашей учетной записи. Смахните сессию влево для ручного удаления/завершения.").size(10.5).color(theme.text_muted));
    ui.add_space(8.0);

    // Current Session Section
    ui.label(egui::RichText::new("ЭТО УСТРОЙСТВО (ТЕКУЩИЙ СЕАНС)").size(10.0).strong().color(theme.accent));
    ui.add_space(3.0);

    let current_session = state.device_sessions.iter().find(|s| s.is_current).cloned();
    egui::Frame::none()
        .fill(Color32::from_rgb(22, 30, 43))
        .rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.2_f32, theme.online_indicator))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("💻").size(24.0));
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let name = current_session.as_ref().map(|s| s.device_name.as_str()).unwrap_or("Windows Desktop");
                        ui.label(egui::RichText::new(name).strong().size(12.5).color(Color32::WHITE));
                        ui.label(egui::RichText::new("• Онлайн").size(10.5).color(theme.online_indicator).strong());
                    });
                    let ver = current_session.as_ref().map(|s| s.app_version.as_str()).unwrap_or("NodeX v0.1.0");
                    ui.label(egui::RichText::new(ver).size(10.5).color(theme.text_muted));
                    let ip = current_session.as_ref().map(|s| s.location_ip.as_str()).unwrap_or("127.0.0.1 (Текущий)");
                    ui.label(egui::RichText::new(ip).size(10.0).color(theme.text_muted));
                });
            });
        });

    ui.add_space(10.0);

    // Other Active Sessions Section
    let other_count = state.device_sessions.iter().filter(|s| !s.is_current).count();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("АКТИВНЫЕ СЕАНСЫ ({})", other_count)).size(10.0).strong().color(theme.text_muted));

        if other_count > 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let terminate_all_btn = egui::Button::new(
                    egui::RichText::new("Завершить все другие сеансы").size(10.0).color(Color32::from_rgb(239, 68, 68))
                )
                .fill(Color32::TRANSPARENT);

                if ui.add(terminate_all_btn).clicked() {
                    state.modals.show_delete_session_confirm = Some("all_other_sessions".to_string());
                }
            });
        }
    });
    ui.add_space(4.0);

    if other_count == 0 {
        egui::Frame::none()
            .fill(theme.bubble_other_bg)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0_f32, theme.border_color))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Нет других активных сессий").size(11.5).color(theme.text_muted));
                    ui.label(egui::RichText::new("Используйте QR-код ниже для входа на телефоне или ноутбуке").size(10.0).color(theme.text_muted));
                });
            });
    } else {
        // Render each device session with swipe-left gesture
        let other_sessions: Vec<crate::state::AccountDeviceSession> = state.device_sessions.iter().filter(|s| !s.is_current).cloned().collect();

        for sess in other_sessions {
            let item_w = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_w, 58.0), Sense::click_and_drag());

            let swipe_key = format!("device_{}", sess.id);
            let current_offset = state.swipe_offsets.entry(swipe_key.clone()).or_insert(0.0);

            if resp.dragged() {
                let delta = resp.drag_delta().x;
                *current_offset = (*current_offset + delta).clamp(-90.0, 0.0);
            } else {
                if *current_offset < -35.0 {
                    *current_offset = -75.0;
                } else {
                    *current_offset = 0.0;
                }
            }

            let is_swiped_open = *current_offset <= -40.0;

            // Background red delete button revealed on left swipe
            if *current_offset < -2.0 {
                let reveal_w = (-*current_offset).min(rect.width());
                let delete_box = egui::Rect::from_min_max(
                    egui::pos2(rect.max.x - reveal_w, rect.min.y),
                    rect.max,
                );
                ui.painter().rect_filled(delete_box, Rounding::same(10.0), Color32::from_rgb(220, 38, 38));

                if reveal_w >= 45.0 {
                    ui.painter().text(
                        delete_box.center() + egui::vec2(0.0, -6.0),
                        Align2::CENTER_CENTER,
                        "🗑",
                        egui::FontId::proportional(17.0),
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        delete_box.center() + egui::vec2(0.0, 10.0),
                        Align2::CENTER_CENTER,
                        "Удалить",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                }
            }

            if resp.clicked() {
                if is_swiped_open {
                    let click_pos = resp.interact_pointer_pos().unwrap_or(rect.min);
                    if click_pos.x >= rect.max.x - 80.0 {
                        state.modals.show_delete_session_confirm = Some(sess.id.clone());
                    }
                    *current_offset = 0.0;
                }
            }

            // Foreground card shifted by current_offset
            let content_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + *current_offset, rect.min.y),
                rect.size(),
            );

            let is_hover = resp.hovered();
            let bg_color = if is_hover { Color32::from_rgb(26, 35, 48) } else { theme.bubble_other_bg };

            ui.painter().rect_filled(content_rect, Rounding::same(10.0), bg_color);
            ui.painter().rect_stroke(content_rect, Rounding::same(10.0), Stroke::new(1.0_f32, if is_hover { theme.accent } else { theme.border_color }));

            let inner_rect = content_rect.shrink2(egui::vec2(10.0, 5.0));
            let mut child_ui = ui.child_ui(
                inner_rect,
                egui::Layout::left_to_right(egui::Align::Center),
                None,
            );

            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&sess.platform_icon).size(20.0));
                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 40.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&sess.device_name).strong().size(12.0).color(theme.text_primary));
                    });
                    ui.label(egui::RichText::new(&sess.app_version).size(10.0).color(theme.text_muted));
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&sess.location_ip).size(9.5).color(theme.text_muted));
                        ui.label(egui::RichText::new(format!("• {}", sess.last_active_str)).size(9.5).color(theme.text_muted));
                    });
                });

                // Small delete button on the right edge of the card for direct mouse clicks too
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("✕").size(11.0).color(Color32::from_rgb(239, 68, 68))).clicked() {
                        state.modals.show_delete_session_confirm = Some(sess.id.clone());
                    }
                });
            });

            ui.add_space(4.0);
        }
    }

    ui.add_space(10.0);

    // Link by QR code / Seed phrase
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("🔗  Подключение нового устройства").strong().size(12.0).color(theme.accent));
            ui.label(egui::RichText::new("Отсканируйте QR-код вторым устройством или авторизуйтесь через 12-24 слова Seed-фразы.").size(10.5).color(theme.text_muted));

            if state.session_pair_token.is_empty() {
                state.session_pair_token = format!("nodex://pair/{}/{}", state.my_node_id, rand::random::<u32>());
            }

            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                draw_qr_code(ui, &state.session_pair_token, 100.0);
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("🔄 Обновить токен").size(10.0)).clicked() {
                    state.session_pair_token = format!("nodex://pair/{}/{}", state.my_node_id, rand::random::<u32>());
                }
                if ui.button(egui::RichText::new("📋 Скопировать ссылку").size(10.0)).clicked() {
                    ui.output_mut(|o| o.copied_text = state.session_pair_token.clone());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🔑 12-24 слова").size(10.0).strong().color(theme.accent)).clicked() {
                        state.modals.show_mnemonic_modal = true;
                    }
                });
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
            ui.checkbox(&mut state.notifications_enabled, "Включить Push-уведомления сверху и звуки");
            ui.label(egui::RichText::new("Показывает всплывающие интерактивные баннеры входящих сообщений в верхней части экрана").size(10.0).color(theme.text_muted));
            ui.add_space(4.0);
            ui.checkbox(&mut state.ringtone_enabled, "Звук входящего вызова");
            ui.checkbox(&mut state.zen_mode, "Режим «Не беспокоить» (без звуков и баннеров)");

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

fn render_chat_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Настройки диалогов и сообщений").size(14.0).strong().color(theme.text_primary));
    ui.add_space(6.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("ПОВЕДЕНИЕ ВВОДА").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.checkbox(&mut state.send_on_enter, "Отправка сообщения по Enter (Shift+Enter — новая строка)");
            ui.checkbox(&mut state.compact_bubbles, "Компактный режим пузырей сообщений");

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(egui::RichText::new("АВТОЗАГРУЗКА МЕДИА И ФАЙЛОВ").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.checkbox(&mut state.auto_download_media, "Автозагрузка фото и видео файлов");
            ui.checkbox(&mut state.auto_download_voice, "Автозагрузка голосовых сообщений");
            ui.checkbox(&mut state.auto_download_files, "Автозагрузка вложенных документов и торрентов");

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(egui::RichText::new("ТАЙМЕР САМОУНИЧТОЖЕНИЯ СООБЩЕНИЙ").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let options = [
                    (0_u64, "Отключен"),
                    (86400_u64, "24 часа"),
                    (604800_u64, "7 дней"),
                    (2592000_u64, "30 дней"),
                ];
                for (secs, lbl) in options {
                    let is_active = state.auto_delete_secs == secs;
                    let btn = egui::Button::new(
                        egui::RichText::new(lbl)
                            .size(10.5)
                            .color(if is_active { Color32::WHITE } else { theme.text_muted })
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(5.0));

                    if ui.add(btn).clicked() {
                        state.auto_delete_secs = secs;
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(egui::RichText::new("МОДУЛЯТОР ГОЛОСА В ЗВОНКАХ").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let modulators = [
                    ("normal", "Оригинал"),
                    ("robot", "Робот"),
                    ("deep", "Бас"),
                    ("crypto", "Крипто-модулятор"),
                ];
                for (val, lbl) in modulators {
                    let is_active = state.voice_modulator == val;
                    let btn = egui::Button::new(
                        egui::RichText::new(lbl)
                            .size(10.5)
                            .color(if is_active { Color32::WHITE } else { theme.text_muted })
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(5.0));

                    if ui.add(btn).clicked() {
                        state.voice_modulator = val.to_string();
                    }
                }
            });
        });
}

fn render_appearance_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme, ctx: &egui::Context) {
    ui.label(egui::RichText::new("Темы и оформление").size(14.0).strong().color(theme.text_primary));
    ui.label(egui::RichText::new("Настройте цветовую схему приложения, акцентный оттенок и оформление сообщений").size(10.5).color(theme.text_muted));
    ui.add_space(8.0);

    // Section 1: Themes (4 Cards with mini previews)
    ui.label(egui::RichText::new("ТЕМА ОФОРМЛЕНИЯ").size(10.0).strong().color(theme.accent));
    ui.add_space(4.0);

    let themes_list = [
        (ThemeMode::Dark, "Темная", Color32::from_rgb(15, 20, 26), Color32::from_rgb(25, 34, 45)),
        (ThemeMode::Midnight, "Полночь", Color32::from_rgb(10, 10, 14), Color32::from_rgb(18, 18, 25)),
        (ThemeMode::Matrix, "Matrix", Color32::from_rgb(8, 18, 12), Color32::from_rgb(14, 28, 18)),
        (ThemeMode::Day, "Дневная", Color32::from_rgb(240, 242, 245), Color32::from_rgb(225, 230, 238)),
    ];

    ui.horizontal(|ui| {
        let card_w = ((ui.available_width() - 24.0) / 4.0).max(85.0);
        for (mode, title, bg_col, bubble_col) in themes_list {
            let is_selected = state.theme_mode == mode;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 88.0), Sense::click());

            let border_stroke = if is_selected {
                Stroke::new(2.0_f32, theme.accent)
            } else {
                Stroke::new(1.0_f32, theme.border_color)
            };
            ui.painter().rect_filled(rect, Rounding::same(10.0), theme.bubble_other_bg);
            ui.painter().rect_stroke(rect, Rounding::same(10.0), border_stroke);

            // Mini preview box inside card
            let preview_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + 6.0, rect.min.y + 6.0),
                Vec2::new(rect.width() - 12.0, 48.0),
            );
            ui.painter().rect_filled(preview_rect, Rounding::same(6.0), bg_col);

            // Incoming bubble in mini preview
            let in_bubble = egui::Rect::from_min_size(
                egui::pos2(preview_rect.min.x + 4.0, preview_rect.min.y + 7.0),
                Vec2::new(preview_rect.width() * 0.55, 12.0),
            );
            ui.painter().rect_filled(in_bubble, Rounding::same(4.0), bubble_col);

            // Outgoing bubble in mini preview
            let out_accent = if let Some(rgb) = state.custom_accent {
                Color32::from_rgb(rgb[0], rgb[1], rgb[2])
            } else {
                match mode {
                    ThemeMode::Matrix => Color32::from_rgb(0, 230, 118),
                    ThemeMode::Midnight => Color32::from_rgb(139, 92, 246),
                    _ => Color32::from_rgb(0, 122, 255),
                }
            };
            let out_bubble = egui::Rect::from_min_size(
                egui::pos2(preview_rect.max.x - preview_rect.width() * 0.62 - 4.0, preview_rect.min.y + 26.0),
                Vec2::new(preview_rect.width() * 0.62, 14.0),
            );
            ui.painter().rect_filled(out_bubble, Rounding::same(4.0), out_accent);

            // Radio indicator + Title
            let circle_center = egui::pos2(rect.min.x + 14.0, rect.max.y - 15.0);
            ui.painter().circle_stroke(circle_center, 6.0, Stroke::new(1.5_f32, if is_selected { theme.accent } else { theme.text_muted }));
            if is_selected {
                ui.painter().circle_filled(circle_center, 3.5, theme.accent);
            }

            let text_pos = egui::pos2(rect.min.x + 25.0, rect.max.y - 15.0);
            ui.painter().text(
                text_pos,
                Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(11.0),
                if is_selected { theme.text_primary } else { theme.text_muted },
            );

            if resp.clicked() {
                state.theme_mode = mode;
                let new_theme = Theme::from_mode_with_accent(mode, state.custom_accent);
                new_theme.apply_to_ctx(ctx);
            }
        }
    });

    ui.add_space(14.0);

    // Section 2: Цветовая палитра / Акцентный цвет (9 Swatches)
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("ЦВЕТОВАЯ ПАЛИТРА И АКЦЕНТ").size(10.0).strong().color(theme.accent));
        if state.custom_accent.is_some() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("По умолчанию").size(10.0).color(theme.text_muted)).clicked() {
                    state.custom_accent = None;
                    let new_theme = Theme::from_mode_with_accent(state.theme_mode, None);
                    new_theme.apply_to_ctx(ctx);
                }
            });
        }
    });
    ui.add_space(6.0);

    let swatches: [([u8; 3], &str); 9] = [
        ([0, 122, 255], "Лазурный"),
        ([0, 229, 255], "Неоновый циан"),
        ([0, 200, 83], "Изумруд"),
        ([124, 77, 255], "Фиолетовый"),
        ([255, 64, 129], "Пурпурный"),
        ([255, 171, 0], "Янтарь"),
        ([255, 82, 82], "Рубин"),
        ([0, 191, 165], "Бирюзовый"),
        ([120, 144, 156], "Стелс"),
    ];

    ui.horizontal(|ui| {
        for (rgb, name) in swatches {
            let swatch_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            let is_active = state.custom_accent == Some(rgb);

            let (_rect, resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
            let center = _rect.center();

            if resp.hovered() {
                ui.painter().circle_filled(center, 15.0, swatch_color);
            } else {
                ui.painter().circle_filled(center, 13.0, swatch_color);
            }

            if is_active {
                ui.painter().circle_stroke(center, 16.5, Stroke::new(2.5_f32, theme.text_primary));
                ui.painter().text(center, Align2::CENTER_CENTER, "✓", egui::FontId::proportional(12.0), Color32::WHITE);
            }

            let resp = resp.on_hover_text(name);
            if resp.clicked() {
                state.custom_accent = Some(rgb);
                let new_theme = Theme::from_mode_with_accent(state.theme_mode, state.custom_accent);
                new_theme.apply_to_ctx(ctx);
            }
            ui.add_space(2.0);
        }
    });

    ui.add_space(10.0);

    // Section 3: Предпросмотр имени и бейджа
    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Предпросмотр профиля:").size(11.0).color(theme.text_muted));
                ui.label(
                    egui::RichText::new(format!("{} (NodeX P2P)", state.my_name))
                        .size(12.0)
                        .strong()
                        .color(theme.accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let badge = egui::RichText::new("E2EE Защищено")
                        .size(10.0)
                        .strong()
                        .color(Color32::WHITE);
                    let badge_btn = egui::Button::new(badge)
                        .fill(theme.accent)
                        .rounding(Rounding::same(12.0));
                    ui.add(badge_btn);
                });
            });
        });

    ui.add_space(14.0);

    // Section 4: Настройки чатов (Chat typography, geometry & wallpaper)
    ui.label(egui::RichText::new("НАСТРОЙКИ ЧАТОВ И ОТОБРАЖЕНИЯ").size(10.0).strong().color(theme.accent));
    ui.add_space(6.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            // Font Size
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Размер текста сообщений:").size(11.5).strong().color(theme.text_primary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{:.1} px", state.chat_font_size)).size(11.5).strong().color(theme.accent));
                });
            });
            ui.add(egui::Slider::new(&mut state.chat_font_size, 11.0..=18.0).step_by(0.5).show_value(false));

            // Font preview snippet
            egui::Frame::none()
                .fill(theme.card_bg)
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Пример: Привет! Это P2P сеть NodeX с шифрованием ChaCha20-Poly1305.")
                            .size(state.chat_font_size)
                            .color(theme.text_primary),
                    );
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Bubble Rounding
            ui.label(egui::RichText::new("Скругление углов сообщений:").size(11.5).strong().color(theme.text_primary));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let roundings = [
                    (4.0_f32, "4px (Строгие)"),
                    (10.0_f32, "10px (Классика)"),
                    (16.0_f32, "16px (NodeX Soft)"),
                    (20.0_f32, "20px (Капсулы)"),
                ];
                for (val, lbl) in roundings {
                    let is_active = (state.bubble_rounding - val).abs() < 0.5;
                    let btn = egui::Button::new(
                        egui::RichText::new(lbl)
                            .size(10.5)
                            .color(if is_active { Color32::WHITE } else { theme.text_muted })
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(6.0));

                    if ui.add(btn).clicked() {
                        state.bubble_rounding = val;
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Chat Wallpaper / Texture
            ui.label(egui::RichText::new("Текстура фона чата:").size(11.5).strong().color(theme.text_primary));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let wallpapers = [
                    ("solid", "Сплошной цвет"),
                    ("mesh", "P2P Mesh (Сетка узлов)"),
                    ("hexagons", "Cyber Hex (Кибер-соты)"),
                ];
                for (w_type, lbl) in wallpapers {
                    let is_active = state.chat_wallpaper_type == w_type;
                    let btn = egui::Button::new(
                        egui::RichText::new(lbl)
                            .size(10.5)
                            .color(if is_active { Color32::WHITE } else { theme.text_muted })
                    )
                    .fill(if is_active { theme.accent } else { theme.card_bg })
                    .rounding(Rounding::same(6.0));

                    if ui.add(btn).clicked() {
                        state.chat_wallpaper_type = w_type.to_string();
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.checkbox(&mut state.auto_night_mode, "Автоматическое переключение ночной темы");
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
                let relay_text = if state.network_stats.relay_active { "Вкл" } else { "Выкл" };
                let relay_color = if state.network_stats.relay_active { theme.accent } else { theme.text_muted };
                if ui.button(egui::RichText::new(relay_text).strong().color(relay_color)).clicked() {
                    state.network_stats.relay_active = !state.network_stats.relay_active;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("UPnP Port Mapping:").size(11.0).color(theme.text_primary));
                let upnp_text = if state.network_stats.upnp_active { "Вкл" } else { "Выкл" };
                let upnp_color = if state.network_stats.upnp_active { theme.accent } else { theme.text_muted };
                if ui.button(egui::RichText::new(upnp_text).strong().color(upnp_color)).clicked() {
                    state.network_stats.upnp_active = !state.network_stats.upnp_active;
                }
            });

            ui.add_space(6.0);
            ui.monospace(egui::RichText::new(format!("DHT Nodes: {} • NAT: {}", state.network_stats.known_nodes, state.network_stats.nat_type)).size(10.0).color(theme.text_muted));
        });
}

fn render_advanced_tab(ui: &mut egui::Ui, state: &mut AppState, theme: &Theme) {
    ui.label(egui::RichText::new("Продвинутые настройки и P2P").size(14.0).strong().color(theme.text_primary));
    ui.add_space(6.0);

    egui::Frame::none()
        .fill(theme.bubble_other_bg)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0_f32, theme.border_color))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("МАРШРУТИЗАЦИЯ И ШИФРОВАНИЕ").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.checkbox(&mut state.p2p_relay_enabled, "Многоузловая P2P-ретрансляция пакетов (Relay Node)");
            ui.checkbox(&mut state.onion_routing_enabled, "Сквозной Onion Routing (2-Hop защищенный туннель)");
            ui.checkbox(&mut state.embed_node_identity, "Стегографическое встраивание ID в аватарах");

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(egui::RichText::new("ОЧИСТКА ДАННЫХ И КЭША").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("🗑 Очистить кэш медиа").size(11.0)).clicked() {
                    state.file_downloads.clear();
                    state.toast_notification = Some(crate::state::ToastNotification {
                        sender_id: "system".to_string(),
                        sender_name: "Система".to_string(),
                        text: "Кэш медиа и временные файлы успешно очищены!".to_string(),
                        created_at: std::time::Instant::now(),
                    });
                    state.audio_player.play_sent_sound();
                }

                if ui.button(egui::RichText::new("⚠️ Очистить историю сообщений").size(11.0).color(Color32::from_rgb(239, 68, 68))).clicked() {
                    state.modals.show_delete_session_confirm = Some("all_chat_history".to_string());
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(egui::RichText::new("ДИАГНОСТИКА СЕТИ").size(10.0).strong().color(theme.accent));
            ui.add_space(4.0);
            ui.monospace(egui::RichText::new(format!("UDP Порт: {} • Узлов DHT: {}", state.port, state.network_stats.known_nodes)).size(10.5).color(theme.text_muted));
            ui.monospace(egui::RichText::new(format!("NAT: {} • Relay: {}", state.network_stats.nat_type, if state.network_stats.relay_active { "Активен" } else { "Выключен" })).size(10.5).color(theme.text_muted));
            ui.monospace(egui::RichText::new(format!("Пакетов отправлено: {} • Доставлено: {}", state.network_stats.messages_sent, state.network_stats.messages_delivered)).size(10.5).color(theme.text_muted));
            ui.monospace(egui::RichText::new(format!("Активных сеансов устройств: {}", state.device_sessions.len())).size(10.5).color(theme.online_indicator));
        });
}
