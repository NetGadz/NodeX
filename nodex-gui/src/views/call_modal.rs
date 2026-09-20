use egui::{vec2, Color32, Margin, Rounding, Stroke, Window};
use crate::components::avatar::draw_avatar;
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};
use crate::views::chat::ChatEventOut;

pub fn render_call_modal(ctx: &egui::Context, state: &mut AppState, theme: &Theme) -> Option<ChatEventOut> {
    let call_state = match &state.call_state {
        Some(cs) => cs.clone(),
        None => return None,
    };

    let mut event_out = None;

    let mut is_open = true;

    Window::new(if call_state.is_incoming { "Incoming P2P Call" } else { "P2P Voice Call (E2EE)" })
        .open(&mut is_open)
        .resizable(true)
        .movable(true)
        .collapsible(false)
        .default_pos(ctx.screen_rect().center())
        .pivot(egui::Align2::CENTER_CENTER)
        .default_size(vec2(360.0, 390.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(14.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::same(24.0)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);

                draw_avatar(ui, &call_state.peer_id, &call_state.peer_name, 72.0, true);
                ui.add_space(14.0);

                ui.label(
                    egui::RichText::new(&call_state.peer_name)
                        .size(18.0)
                        .strong()
                        .color(theme.text_primary),
                );

                let status_text = if call_state.is_connected {
                    let mins = (call_state.duration_secs as u64) / 60;
                    let secs = (call_state.duration_secs as u64) % 60;
                    format!("{:02}:{:02} • E2EE Direct Audio", mins, secs)
                } else if call_state.is_incoming {
                    "Incoming P2P Audio Call...".to_string()
                } else {
                    "Calling... Ringing Peer...".to_string()
                };

                ui.label(
                    egui::RichText::new(status_text)
                        .size(12.0)
                        .color(if call_state.is_connected { theme.online_indicator } else { theme.accent }),
                );

                ui.add_space(24.0);

                // Call Controls
                if call_state.is_incoming && !call_state.is_connected {
                    ui.horizontal(|ui| {
                        // Accept Call
                        let accept_btn = egui::Button::new(
                            egui::RichText::new("[ Accept ]")
                                .color(Color32::WHITE)
                                .size(13.0)
                                .strong(),
                        )
                        .fill(theme.online_indicator)
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(vec2(130.0, 36.0), accept_btn).clicked() {
                            if let Some(cs) = &mut state.call_state {
                                cs.is_connected = true;
                            }
                            event_out = Some(ChatEventOut::AcceptCall { peer_id: call_state.peer_id.clone() });
                        }

                        ui.add_space(16.0);

                        // Decline Call
                        let decline_btn = egui::Button::new(
                            egui::RichText::new("[ Decline ]")
                                .color(Color32::WHITE)
                                .size(13.0)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(231, 76, 60))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(vec2(130.0, 36.0), decline_btn).clicked() {
                            event_out = Some(ChatEventOut::DeclineCall { peer_id: call_state.peer_id.clone() });
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        // Mute button
                        let is_muted = call_state.is_muted;
                        let mute_btn = egui::Button::new(
                            egui::RichText::new(if is_muted { "[ Unmute ]" } else { "[ Mute ]" })
                                .size(12.0)
                                .color(theme.text_primary),
                        )
                        .fill(theme.bubble_other_bg)
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(vec2(95.0, 34.0), mute_btn).clicked() {
                            if let Some(cs) = &mut state.call_state {
                                cs.is_muted = !cs.is_muted;
                            }
                        }

                        ui.add_space(12.0);

                        // End Call Button
                        let hangup_btn = egui::Button::new(
                            egui::RichText::new("[ End Call ]")
                                .color(Color32::WHITE)
                                .size(12.0)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(231, 76, 60))
                        .rounding(Rounding::same(8.0));

                        if ui.add_sized(vec2(120.0, 34.0), hangup_btn).clicked() {
                            event_out = Some(ChatEventOut::EndCall { peer_id: call_state.peer_id.clone() });
                        }
                    });
                }
            });
        });

    if !is_open {
        if state.call_state.is_some() {
            event_out = Some(ChatEventOut::EndCall { peer_id: call_state.peer_id.clone() });
        }
    }
    
    event_out
}
