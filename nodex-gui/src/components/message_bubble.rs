use egui::{Color32, Rounding, Stroke, Ui};
use nodex_messenger::db::SavedChatMessage;
use crate::components::attachment_card::draw_attachment_card;
use crate::components::voice_player::{draw_voice_note, VoicePlaybackState};
use crate::theme::{
    BORDER_COLOR, BUBBLE_INCOMING, BUBBLE_OUTGOING, ONLINE_GREEN, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
    ACCENT_BLUE,
};

pub enum MessageAction {
    Reply(SavedChatMessage),
    Edit(SavedChatMessage),
    React(String, String), // msg_id, emoji
    Pin(String),
    Delete(String),
    DeleteEveryone(String),
}

pub fn draw_message_bubble(
    ui: &mut Ui,
    msg: &SavedChatMessage,
    playback_state: &mut VoicePlaybackState,
    action_out: &mut Option<MessageAction>,
) {
    let is_outgoing = !msg.incoming;
    let bubble_bg = if is_outgoing { BUBBLE_OUTGOING } else { BUBBLE_INCOMING };

    let is_call_event = msg.text.starts_with("📞") || msg.text.starts_with("📵") || msg.text.starts_with("🚫");

    // Centered compact rounded call banner
    if is_call_event {
        ui.vertical_centered(|ui| {
            ui.add_space(2.0);
            let icon = if msg.text.starts_with("📞") { "📞" } else if msg.text.starts_with("📵") { "📵" } else { "🚫" };
            let color = if msg.text.starts_with("📞") { ONLINE_GREEN } else { Color32::from_rgb(239, 68, 68) };
            let time_str = format_time(msg.timestamp);

            egui::Frame::none()
                .fill(Color32::from_rgb(20, 24, 34))
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(egui::Margin::symmetric(14.0, 5.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).size(12.0));
                        ui.label(egui::RichText::new(&msg.text).size(11.5).color(color).strong());
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(time_str).size(10.0).color(TEXT_MUTED));
                    });
                });
            ui.add_space(2.0);
        });
        return;
    }

    let layout = if is_outgoing {
        egui::Layout::right_to_left(egui::Align::Min)
    } else {
        egui::Layout::left_to_right(egui::Align::Min)
    };

    let bubble_rounding = if is_outgoing {
        Rounding { nw: 14.0, ne: 14.0, sw: 14.0, se: 3.0 }
    } else {
        Rounding { nw: 14.0, ne: 14.0, se: 14.0, sw: 3.0 }
    };

    let max_bubble_w = (ui.available_width() * 0.65).clamp(140.0, 460.0);

    ui.with_layout(layout, |ui| {
        ui.set_max_width(max_bubble_w);
        let frame_resp = egui::Frame::none()
            .fill(bubble_bg)
            .rounding(bubble_rounding)
            .stroke(Stroke::new(1.0_f32, if is_outgoing { Color32::from_rgb(37, 99, 235) } else { BORDER_COLOR }))
            .inner_margin(egui::Margin::symmetric(11.0, 7.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                    let is_file = msg.text.starts_with("[File: ");
                    let has_reply = msg.reply_snippet.is_some();
                    let has_reactions = !msg.reactions.is_empty();

                    // Pinned indicator badge
                    if msg.is_pinned {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📌 Pinned").size(9.5).color(TEXT_MUTED));
                        });
                        ui.add_space(2.0);
                    }

                    // Reply snippet header
                    if let Some(ref reply_text) = msg.reply_snippet {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(16, 17, 23))
                            .rounding(Rounding::ZERO)
                            .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(reply_text).size(11.0).color(TEXT_SECONDARY));
                            });
                        ui.add_space(4.0);
                    }

                    if let Some(ref vn) = msg.voice_note {
                        draw_voice_note(ui, &msg.id, vn, is_outgoing, msg.timestamp, msg.delivered, playback_state);
                    } else if is_file {
                        let is_image = msg.text.to_lowercase().ends_with(".png]")
                            || msg.text.to_lowercase().ends_with(".jpg]")
                            || msg.text.to_lowercase().ends_with(".jpeg]");

                        let mut drawn_image = false;
                        if is_image {
                            if let Some(base64_str) = &msg.image_base64 {
                                use base64::Engine;
                                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(base64_str) {
                                    let img = egui::Image::from_bytes(
                                        format!("img_{}", msg.id),
                                        bytes
                                    ).max_width(ui.available_width().min(260.0)).rounding(Rounding::ZERO);
                                    ui.add(img);
                                    drawn_image = true;
                                }
                            }
                        }
                        
                        if !drawn_image {
                            draw_attachment_card(ui, &msg.text, 1024 * 1024, is_outgoing);
                        }
                        ui.add_space(2.0);
                        render_bubble_time(ui, msg, is_outgoing);
                    } else {
                        let is_short_single_line = !has_reply && !has_reactions && !msg.text.contains('\n') && msg.text.chars().count() < 30;

                        if is_short_single_line {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&msg.text).size(13.5).color(TEXT_PRIMARY));
                                ui.add_space(8.0);
                                render_time_and_ticks(ui, msg, is_outgoing);
                            });
                        } else {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&msg.text)
                                        .size(13.5)
                                        .color(TEXT_PRIMARY),
                                )
                                .wrap_mode(egui::TextWrapMode::Wrap),
                            );
                            ui.add_space(2.0);
                            render_bubble_time(ui, msg, is_outgoing);
                        }
                    }

                    // Emoji Reactions bar
                    if has_reactions {
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            for (emoji, reactors) in &msg.reactions {
                                let reaction_pill = format!("{} {}", emoji, reactors.len());
                                if ui.button(egui::RichText::new(reaction_pill).size(10.5).color(TEXT_SECONDARY)).clicked() {
                                    *action_out = Some(MessageAction::React(msg.id.clone(), emoji.clone()));
                                }
                            }
                        });
                    }
                });
            });

        // Right-Click Context Menu
        frame_resp.response.context_menu(|ui| {
            if ui.button("Reply").clicked() {
                *action_out = Some(MessageAction::Reply(msg.clone()));
                ui.close_menu();
            }
            if is_outgoing && ui.button("Edit").clicked() {
                *action_out = Some(MessageAction::Edit(msg.clone()));
                ui.close_menu();
            }
            if ui.button("Pin / Unpin").clicked() {
                *action_out = Some(MessageAction::Pin(msg.id.clone()));
                ui.close_menu();
            }
            ui.menu_button("React", |ui| {
                for &emoji in nodex_messenger::reactions::DEFAULT_REACTIONS {
                    if ui.button(emoji).clicked() {
                        *action_out = Some(MessageAction::React(msg.id.clone(), emoji.to_string()));
                        ui.close_menu();
                    }
                }
            });
            ui.separator();
            if ui.button("Copy Text").clicked() {
                ui.output_mut(|o| o.copied_text = msg.text.clone());
                ui.close_menu();
            }
            if ui.button("Delete for Me").clicked() {
                *action_out = Some(MessageAction::Delete(msg.id.clone()));
                ui.close_menu();
            }
            if ui.button("Delete for Everyone").clicked() {
                *action_out = Some(MessageAction::DeleteEveryone(msg.id.clone()));
                ui.close_menu();
            }
        });
    });
}

fn render_bubble_time(ui: &mut Ui, msg: &SavedChatMessage, is_outgoing: bool) {
    ui.horizontal(|ui| {
        if msg.is_edited {
            ui.label(egui::RichText::new("edited").size(9.5).color(TEXT_MUTED));
            ui.add_space(2.0);
        }
        render_time_and_ticks(ui, msg, is_outgoing);
    });
}

fn render_time_and_ticks(ui: &mut Ui, msg: &SavedChatMessage, is_outgoing: bool) {
    let time_str = format_time(msg.timestamp);
    let time_color = TEXT_MUTED;
    ui.label(egui::RichText::new(time_str).size(10.0).color(time_color));

    if is_outgoing {
        ui.add_space(2.0);
        // Quiet UX Delivery Check: + (sent to DHT), ++ (delivered to peer)
        if msg.delivered {
            ui.label(egui::RichText::new("++").size(10.5).strong().color(ONLINE_GREEN));
        } else {
            ui.label(egui::RichText::new("+").size(10.5).color(TEXT_MUTED));
        }
    }
}

fn format_time(ts: u64) -> String {
    let hours = (ts % 86400) / 3600;
    let mins = (ts % 3600) / 60;
    format!("{:02}:{:02}", hours, mins)
}
