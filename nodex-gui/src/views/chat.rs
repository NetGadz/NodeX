use egui::{Color32, Margin, Rounding, ScrollArea, Stroke, Ui, Vec2, Sense};
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use nodex_messenger::groups::P2PGroup;
use nodex_messenger::voice::VoiceNote;
use crate::components::avatar::draw_avatar;
use crate::components::icons::{draw_paperclip_icon, draw_send_icon};
use crate::components::message_bubble::{draw_message_bubble, MessageAction};
use crate::components::voice_recorder::draw_voice_record_button;
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub enum ChatEventOut {
    SendMessage { text: String, reply_to_id: Option<String> },
    SendFile { filepath: String },
    SendVoiceNote { voice_note: VoiceNote },
    ReactMessage { msg_id: String, emoji: String },
    EditMessage { msg_id: String, new_text: String },
    PinMessage { msg_id: String },
    DeleteMessage { msg_id: String },
    DeleteEveryone { msg_id: String },
    StartCall { peer_id: String },
    AcceptCall { peer_id: String },
    DeclineCall { peer_id: String },
    EndCall { peer_id: String },
}

pub fn render_chat_view(
    ui: &mut Ui,
    state: &mut AppState,
    theme: &Theme,
    contact: Option<&SavedContact>,
    group: Option<&P2PGroup>,
    messages: &[SavedChatMessage],
    event_out: &mut Option<ChatEventOut>,
) {
    let is_saved_messages = state.selected_chat_id.as_deref() == Some("self_saved_messages");
    let is_self_chat = state.selected_chat_id.as_deref() == Some(&state.my_node_id);
    let title: String = if is_saved_messages {
        "Избранное".to_string()
    } else if is_self_chat {
        format!("{} (Вы)", state.my_name)
    } else {
        contact.map(|c| c.name.clone()).or_else(|| group.map(|g| g.title.clone())).unwrap_or_else(|| "Диалог".to_string())
    };
    let target_id: String = if is_saved_messages {
        "self_saved_messages".to_string()
    } else if is_self_chat {
        state.my_node_id.clone()
    } else {
        contact.map(|c| c.user_id_hex.clone()).or_else(|| group.map(|g| g.group_id.clone())).unwrap_or_else(|| "self".to_string())
    };
    let is_online = if is_saved_messages || is_self_chat { false } else { state.is_peer_online(&target_id) };

    ui.vertical(|ui| {
        // 1. Top Chat Header (Strictly Flat Minimalist)
        egui::Frame::none()
            .fill(theme.sidebar_bg)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    draw_avatar(ui, &target_id, &title, 36.0, is_online);
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&title).strong().size(14.5).color(theme.text_primary));
                        });

                        if is_saved_messages || is_self_chat {
                            ui.label(egui::RichText::new("персональное защищенное хранилище").size(11.5).color(theme.text_muted));
                        } else if let Some(g) = group {
                            ui.label(egui::RichText::new(format!("{} участников", g.members.len())).size(11.5).color(theme.text_muted));
                        } else {
                            ui.horizontal(|ui| {
                                if is_online {
                                    ui.label(egui::RichText::new("● в сети").size(11.5).color(theme.online_indicator));
                                } else {
                                    ui.label(egui::RichText::new("был(а) недавно").size(11.5).color(theme.text_muted));
                                }

                                if state.onion_routing_enabled {
                                    ui.add_space(6.0);
                                    ui.label(egui::RichText::new("• 🛡 Onion 2-Hop").size(11.0).color(Color32::from_rgb(168, 85, 247)));
                                }
                            });
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let info_btn = egui::Button::new(
                            egui::RichText::new("Профиль").size(11.5).color(theme.text_primary)
                        )
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(Vec2::new(64.0, 26.0), info_btn).clicked() {
                            state.show_info_drawer = !state.show_info_drawer;
                        }

                        // Onion 2-Hop Privacy Shield toggle
                        ui.add_space(4.0);
                        let onion_color = if state.onion_routing_enabled {
                            Color32::from_rgb(168, 85, 247)
                        } else {
                            BORDER_COLOR
                        };
                        let onion_btn = egui::Button::new(
                            egui::RichText::new(if state.onion_routing_enabled { "🛡 Onion" } else { "⚡ Direct" })
                                .size(11.0)
                                .color(if state.onion_routing_enabled { Color32::WHITE } else { theme.text_muted }),
                        )
                        .fill(if state.onion_routing_enabled { Color32::from_rgb(45, 20, 60) } else { theme.card_bg })
                        .stroke(Stroke::new(1.0_f32, onion_color))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(Vec2::new(76.0, 26.0), onion_btn).clicked() {
                            state.onion_routing_enabled = !state.onion_routing_enabled;
                        }

                        if !is_saved_messages && !is_self_chat && contact.is_some() {
                            ui.add_space(4.0);
                            let call_btn = egui::Button::new(
                                egui::RichText::new("📞 Звонок").size(11.5).color(theme.accent)
                            )
                            .fill(theme.card_bg)
                            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                            .rounding(Rounding::same(8.0));

                            if ui.add_sized(Vec2::new(72.0, 26.0), call_btn).clicked() {
                                *event_out = Some(ChatEventOut::StartCall { peer_id: target_id.to_string() });
                            }
                        }
                    });
                });
            });

        // 2. Pinned Message Banner (if any)
        if let Some(pinned_msg) = messages.iter().rev().find(|m| m.is_pinned) {
            egui::Frame::none()
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::symmetric(14.0, 6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📌").size(12.0).color(theme.accent));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Pinned").strong().size(10.5).color(theme.accent));
                            let short_text = crate::theme::truncate_str(&pinned_msg.text, 50);
                            ui.label(egui::RichText::new(short_text).size(11.0).color(theme.text_primary));
                        });
                    });
                });
        }

        // 3. Message History Canvas (with wallpaper support)
        let available_height = ui.available_height() - 95.0;
        let wallpaper_rect = ui.available_rect_before_wrap();
        crate::components::chat_background::draw_chat_wallpaper(
            &ui.painter(),
            wallpaper_rect,
            theme.window_bg,
            &state.chat_wallpaper_type,
            theme.accent,
        );

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(available_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add_space(8.0);
                let mut bubble_action: Option<MessageAction> = None;

                if messages.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("Нет сообщений")
                                .size(13.5)
                                .color(theme.text_muted),
                        );
                        ui.label(
                            egui::RichText::new("Отправьте сообщение или голосовую заметку, чтобы начать диалог.")
                                .size(11.5)
                                .color(theme.text_muted),
                        );
                    });
                } else {
                    for msg in messages {
                        draw_message_bubble(
                            ui,
                            msg,
                            &mut state.voice_playback,
                            &mut bubble_action,
                            theme,
                            state.chat_font_size,
                            state.bubble_rounding,
                        );
                        ui.add_space(4.0);
                    }
                }

                if let Some(action) = bubble_action {
                    match action {
                        MessageAction::Reply(m) => state.replying_to = Some(m),
                        MessageAction::Edit(m) => {
                            state.message_input = m.text.clone();
                            state.editing_msg = Some(m);
                        }
                        MessageAction::React(msg_id, emoji) => {
                            *event_out = Some(ChatEventOut::ReactMessage { msg_id, emoji });
                        }
                        MessageAction::Pin(msg_id) => {
                            *event_out = Some(ChatEventOut::PinMessage { msg_id });
                        }
                        MessageAction::Delete(msg_id) => {
                            *event_out = Some(ChatEventOut::DeleteMessage { msg_id });
                        }
                        MessageAction::DeleteEveryone(msg_id) => {
                            *event_out = Some(ChatEventOut::DeleteEveryone { msg_id });
                        }
                    }
                }
            });

        // 4. Replying / Editing Context Bar
        let mut clear_reply = false;
        let mut clear_edit = false;

        if let Some(reply_msg) = &state.replying_to {
            let reply_text = reply_msg.text.clone();
            egui::Frame::none()
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::symmetric(12.0, 5.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("↩ Replying:").size(11.0).color(theme.accent));
                        ui.label(egui::RichText::new(reply_text).size(11.0).color(theme.text_muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                clear_reply = true;
                            }
                        });
                    });
                });
        } else if let Some(edit_msg) = &state.editing_msg {
            let edit_text = edit_msg.text.clone();
            egui::Frame::none()
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::symmetric(12.0, 5.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✏ Editing:").size(11.0).color(theme.accent));
                        ui.label(egui::RichText::new(edit_text).size(11.0).color(theme.text_muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").clicked() {
                                clear_edit = true;
                            }
                        });
                    });
                });
        }

        if clear_reply {
            state.replying_to = None;
        }
        if clear_edit {
            state.editing_msg = None;
            state.message_input.clear();
        }

        // 5. Message Composer Bar (Clean Swiss Minimalist)
        egui::Frame::none()
            .fill(theme.sidebar_bg)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .inner_margin(Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Attachment Paperclip button
                    let (clip_rect, clip_resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                    let clip_hover = clip_resp.hovered();
                    if clip_hover {
                        ui.painter().rect_filled(clip_rect, Rounding::same(6.0), theme.bubble_other_bg);
                    }
                    draw_paperclip_icon(&ui.painter(), clip_rect, if clip_hover { theme.accent } else { theme.text_muted });

                    if clip_resp.clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            *event_out = Some(ChatEventOut::SendFile {
                                filepath: path.to_string_lossy().to_string(),
                            });
                        }
                    }

                    if state.voice_recorder.is_recording {
                        let elapsed = state.voice_recorder.start_time.map_or(0.0, |t| t.elapsed().as_secs_f32());
                        let mins = (elapsed as u32) / 60;
                        let secs = (elapsed as u32) % 60;

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("● ЗАПИСЬ {:02}:{:02}", mins, secs)).color(Color32::from_rgb(231, 76, 60)).strong().size(12.5));
                            
                            ui.add_space(ui.available_width() - 140.0);

                            if ui.button(egui::RichText::new("Отмена").size(11.5).color(theme.text_muted)).clicked() {
                                state.mic_recorder.stop();
                                state.voice_recorder.is_recording = false;
                                state.voice_recorder.start_time = None;
                            }

                            let mut recorded_voice: Option<VoiceNote> = None;
                            draw_voice_record_button(ui, &mut state.voice_recorder, &mut state.mic_recorder, theme, &mut recorded_voice);

                            if let Some(vn) = recorded_voice {
                                *event_out = Some(ChatEventOut::SendVoiceNote { voice_note: vn });
                            }
                        });
                    } else {
                        // Text Input Box (smooth rounding, 1px border)
                        let text_edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut state.message_input)
                                .hint_text("Написать сообщение...")
                                .desired_width(ui.available_width() - 76.0),
                        );

                        let enter_pressed = text_edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let has_text = !state.message_input.trim().is_empty();

                        // Microphone Button OR Send Button
                        if has_text {
                            let (send_rect, send_resp) = ui.allocate_exact_size(Vec2::new(56.0, 28.0), Sense::click());
                            let send_painter = ui.painter_at(send_rect);
                            send_painter.rect_filled(send_rect, Rounding::same(8.0), theme.accent);
                            draw_send_icon(&send_painter, send_rect, Color32::WHITE);

                            if send_resp.clicked() || enter_pressed {
                                let text = state.message_input.trim().to_string();
                                if let Some(edit_msg) = &state.editing_msg {
                                    *event_out = Some(ChatEventOut::EditMessage {
                                        msg_id: edit_msg.id.clone(),
                                        new_text: text,
                                    });
                                    state.editing_msg = None;
                                } else {
                                    let reply_to_id = state.replying_to.as_ref().map(|r| r.id.clone());
                                    *event_out = Some(ChatEventOut::SendMessage { text, reply_to_id });
                                    state.replying_to = None;
                                }
                                state.message_input.clear();
                            }
                        } else {
                            let mut recorded_voice: Option<VoiceNote> = None;
                            draw_voice_record_button(ui, &mut state.voice_recorder, &mut state.mic_recorder, theme, &mut recorded_voice);

                            if let Some(vn) = recorded_voice {
                                *event_out = Some(ChatEventOut::SendVoiceNote { voice_note: vn });
                            }
                        }
                    }
                });
            });
    });
}
