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
    let title = if is_saved_messages {
        "Saved Messages"
    } else {
        contact.map(|c| c.name.as_str()).or_else(|| group.map(|g| g.title.as_str())).unwrap_or("Select a chat")
    };
    let target_id = if is_saved_messages {
        "self_saved_messages"
    } else {
        contact.map(|c| c.user_id_hex.as_str()).or_else(|| group.map(|g| g.group_id.as_str())).unwrap_or("self")
    };

    ui.vertical(|ui| {
        // 1. Top Chat Header (Strictly Flat Minimalist)
        egui::Frame::none()
            .fill(theme.sidebar_bg)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    draw_avatar(ui, target_id, title, 36.0, true);
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(title).strong().size(14.5).color(theme.text_primary));
                        });

                        // Quiet UX: Lowercase text status
                        if is_saved_messages {
                            ui.label(egui::RichText::new("encrypted personal cloud").size(11.0).color(theme.text_muted));
                        } else if let Some(g) = group {
                            ui.label(egui::RichText::new(format!("{} members", g.members.len())).size(11.0).color(theme.text_muted));
                        } else {
                            ui.horizontal(|ui| {
                                if state.onion_routing_enabled {
                                    ui.label(egui::RichText::new("• onion 2-hop").size(11.0).color(Color32::from_rgb(168, 85, 247)));
                                } else {
                                    // Quiet UX: direct (neon-green) or relay (gray)
                                    ui.label(egui::RichText::new("• direct").size(11.0).strong().color(theme.accent));
                                }
                            });
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let info_btn = egui::Button::new(
                            egui::RichText::new("[ Info ]").size(11.5).color(theme.text_primary)
                        )
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(Vec2::new(58.0, 26.0), info_btn).clicked() {
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
                            egui::RichText::new(if state.onion_routing_enabled { "[ Onion 2-Hop ]" } else { "[ Direct P2P ]" })
                                .size(11.0)
                                .color(if state.onion_routing_enabled { Color32::WHITE } else { theme.text_muted }),
                        )
                        .fill(if state.onion_routing_enabled { Color32::from_rgb(45, 20, 60) } else { theme.card_bg })
                        .stroke(Stroke::new(1.0_f32, onion_color))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(Vec2::new(100.0, 26.0), onion_btn).clicked() {
                            state.onion_routing_enabled = !state.onion_routing_enabled;
                        }

                        if !is_saved_messages && contact.is_some() {
                            ui.add_space(4.0);
                            let call_btn = egui::Button::new(
                                egui::RichText::new("[ Call ]").size(11.5).color(theme.accent)
                            )
                            .fill(theme.card_bg)
                            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                            .rounding(Rounding::same(8.0));

                            if ui.add_sized(Vec2::new(58.0, 26.0), call_btn).clicked() {
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

        // 3. Message History Canvas (Clean matte dark canvas)
        let available_height = ui.available_height() - 95.0;

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
                            egui::RichText::new("No messages yet")
                                .size(13.5)
                                .color(theme.text_muted),
                        );
                        ui.label(
                            egui::RichText::new("Send a message or voice note to begin.")
                                .size(11.5)
                                .color(theme.text_muted),
                        );
                    });
                } else {
                    for msg in messages {
                        draw_message_bubble(ui, msg, &mut state.voice_playback, &mut bubble_action);
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

                    // Quiet UX 3.2: [ ] Embed Node Identity checkbox
                    ui.checkbox(&mut state.embed_node_identity, egui::RichText::new("Embed ID").size(10.5).color(theme.text_muted));
                    ui.add_space(4.0);

                    if state.voice_recorder.is_recording {
                        let elapsed = state.voice_recorder.start_time.map_or(0.0, |t| t.elapsed().as_secs_f32());
                        let mins = (elapsed as u32) / 60;
                        let secs = (elapsed as u32) % 60;

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("● RECORDING {:02}:{:02}", mins, secs)).color(Color32::from_rgb(231, 76, 60)).strong().size(12.5));
                            
                            ui.add_space(ui.available_width() - 140.0);

                            if ui.button(egui::RichText::new("[ Cancel ]").size(11.0).color(theme.text_muted)).clicked() {
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
                                .hint_text("Write a message...")
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
