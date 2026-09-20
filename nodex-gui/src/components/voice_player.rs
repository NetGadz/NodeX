use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2, Rounding};
use nodex_messenger::voice::VoiceNote;
use crate::theme::{ACCENT_BLUE, TEXT_MUTED};

#[derive(Clone, Debug, Default)]
pub struct VoicePlaybackState {
    pub playing_msg_id: Option<String>,
    pub current_progress: f32, // 0.0 .. 1.0
    pub requested_play: Option<(String, VoiceNote)>,
    pub requested_stop: bool,
}

pub fn draw_voice_note(
    ui: &mut Ui,
    msg_id: &str,
    voice_note: &VoiceNote,
    is_outgoing: bool,
    timestamp: u64,
    delivered: bool,
    playback_state: &mut VoicePlaybackState,
) {
    let is_playing = playback_state.playing_msg_id.as_deref() == Some(msg_id);

    let btn_bg = if is_outgoing {
        Color32::from_rgb(52, 120, 246)
    } else {
        ACCENT_BLUE
    };

    ui.push_id(format!("vn_box_{}", msg_id), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // Play/Pause circular button
            let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
            let btn_resp = btn_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let btn_painter = ui.painter_at(btn_rect);

            let pointer_pos = ui.ctx().pointer_hover_pos();
            let is_pointer_over_btn = pointer_pos.map_or(false, |p| btn_rect.contains(p));
            let is_pointer_down = is_pointer_over_btn && ui.input(|i| i.pointer.primary_down());
            let direct_btn_clicked = is_pointer_over_btn && ui.input(|i| i.pointer.primary_clicked());

            let is_active = btn_resp.is_pointer_button_down_on() || is_pointer_down;
            let is_hovered = btn_resp.hovered() || is_pointer_over_btn;

            let circle_color = if is_active {
                Color32::from_rgb(35, 100, 220)
            } else if is_hovered {
                Color32::from_rgb(70, 145, 255)
            } else {
                btn_bg
            };
            btn_painter.circle_filled(btn_rect.center(), 18.0, circle_color);

            if is_playing {
                crate::components::icons::draw_pause_icon(&btn_painter, btn_rect, Color32::WHITE);
            } else {
                crate::components::icons::draw_play_icon(&btn_painter, btn_rect, Color32::WHITE);
            }

            if btn_resp.clicked() || direct_btn_clicked {
                println!("[VOICE NOTE] Clicked button for msg_id: {} (currently playing: {})", msg_id, is_playing);
                if is_playing {
                    playback_state.playing_msg_id = None;
                    playback_state.requested_stop = true;
                } else {
                    playback_state.playing_msg_id = Some(msg_id.to_string());
                    playback_state.current_progress = 0.0;
                    playback_state.requested_play = Some((msg_id.to_string(), voice_note.clone()));
                }
                ui.ctx().request_repaint();
            }

            ui.add_space(8.0);

            ui.vertical(|ui| {
                // Waveform bar visualizer
                let waveform_width = 120.0;
                let waveform_height = 24.0;
                let (wave_rect, mut wave_resp) = ui.allocate_exact_size(Vec2::new(waveform_width, waveform_height), Sense::click());
                wave_resp = wave_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                let wave_painter = ui.painter_at(wave_rect);

                let is_pointer_over_wave = pointer_pos.map_or(false, |p| wave_rect.contains(p));
                let direct_wave_clicked = is_pointer_over_wave && ui.input(|i| i.pointer.primary_clicked());

                // Click on waveform to seek & play
                if wave_resp.clicked() || direct_wave_clicked {
                    let click_pos = wave_resp.interact_pointer_pos().or(pointer_pos);
                    if let Some(pos) = click_pos {
                        let rel_x = ((pos.x - wave_rect.min.x) / wave_rect.width()).clamp(0.0, 1.0);
                        playback_state.playing_msg_id = Some(msg_id.to_string());
                        playback_state.current_progress = rel_x;
                        playback_state.requested_play = Some((msg_id.to_string(), voice_note.clone()));
                        ui.ctx().request_repaint();
                    }
                }

                let samples = &voice_note.waveform;
                let num_bars = samples.len().max(1);
                let bar_width = 2.0;
                let bar_gap = ((waveform_width - (num_bars as f32 * bar_width)) / (num_bars as f32).max(1.0)).max(0.2);

                let max_amp = samples.iter().copied().max().unwrap_or(1).max(1) as f32;
                let progress = if is_playing { playback_state.current_progress } else { 0.0 };

                for (i, &amp) in samples.iter().enumerate() {
                    let bar_x = wave_rect.min.x + (i as f32 * (bar_width + bar_gap));
                    let norm_h = ((amp as f32 / max_amp) * waveform_height).clamp(3.0, waveform_height);
                    let bar_h = norm_h;
                    let bar_y = wave_rect.center().y - (bar_h / 2.0);

                    let is_played = (i as f32 / num_bars as f32) <= progress;
                    let bar_color = if is_played {
                        Color32::WHITE
                    } else if is_outgoing {
                        Color32::from_rgb(150, 185, 220)
                    } else {
                        TEXT_MUTED
                    };

                    let bar_rect = Rect::from_min_size(Pos2::new(bar_x, bar_y), Vec2::new(bar_width, bar_h));
                    wave_painter.rect_filled(bar_rect, Rounding::same(1.0), bar_color);
                }

            ui.add_space(2.0);

            // Bottom row: Duration on left, Time + delivery on right
            ui.horizontal(|ui| {
                let elapsed_secs = if is_playing {
                    ((progress * voice_note.duration_secs as f32).round() as u32).min(voice_note.duration_secs)
                } else {
                    voice_note.duration_secs
                };
                let dur_str = format!("{}:{:02}", elapsed_secs / 60, elapsed_secs % 60);

                ui.label(
                    egui::RichText::new(dur_str)
                        .size(11.0)
                        .color(if is_outgoing { Color32::from_rgb(200, 225, 250) } else { TEXT_MUTED }),
                );

                ui.add_space(14.0);

                let hours = (timestamp % 86400) / 3600;
                let mins = (timestamp % 3600) / 60;
                let time_str = format!("{:02}:{:02}", hours, mins);
                let time_color = if is_outgoing { Color32::from_rgb(160, 185, 210) } else { TEXT_MUTED };
                ui.label(egui::RichText::new(time_str).size(10.0).color(time_color));

                if is_outgoing {
                    let (tick_rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 10.0), egui::Sense::hover());
                    let painter = ui.painter_at(tick_rect);
                    if delivered {
                        crate::components::icons::draw_double_check(&painter, tick_rect.left_top(), 9.0, crate::theme::ONLINE_GREEN);
                    } else {
                        crate::components::icons::draw_single_check(&painter, tick_rect.left_top(), 9.0, TEXT_MUTED);
                    }
                }
            });
        });
    });
    });
}

