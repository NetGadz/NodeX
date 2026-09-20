use egui::{Margin, Rounding, ScrollArea, Stroke, Ui};
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use nodex_messenger::groups::P2PGroup;
use crate::components::avatar::draw_avatar;
use crate::state::{AppState, SharedMediaTab};
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_right_drawer(
    ui: &mut Ui,
    state: &mut AppState,
    theme: &Theme,
    contact: Option<&SavedContact>,
    group: Option<&P2PGroup>,
    messages: &[SavedChatMessage],
) {
    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(if group.is_some() {
                    "Group Info"
                } else {
                    "Peer Info"
                })
                .size(15.0)
                .color(theme.text_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("✕").color(theme.text_muted)).clicked() {
                    state.show_info_drawer = false;
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let title = contact.map(|c| c.name.as_str()).or_else(|| group.map(|g| g.title.as_str())).unwrap_or("Saved Messages");
        let target_id = contact.map(|c| c.user_id_hex.as_str()).or_else(|| group.map(|g| g.group_id.as_str())).unwrap_or("self");

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Profile Avatar & Name
                ui.vertical_centered(|ui| {
                    draw_avatar(ui, target_id, title, 56.0, true);
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(title)
                            .size(16.0)
                            .strong()
                            .color(theme.text_primary),
                    );

                    let status_text = if let Some(g) = group {
                        format!("{} members", g.members.len())
                    } else {
                        "• direct p2p".to_string()
                    };

                    ui.label(
                        egui::RichText::new(status_text)
                            .size(11.5)
                            .color(theme.online_indicator),
                    );
                });

                ui.add_space(14.0);

                // Information Card
                egui::Frame::none()
                    .fill(theme.bubble_other_bg)
                    .rounding(Rounding::same(8.0))
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Node ID:")
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                        ui.horizontal(|ui| {
                            let short_id = crate::theme::truncate_id(target_id, 8, 8);
                            ui.monospace(
                                egui::RichText::new(&short_id)
                                    .size(11.0)
                                    .color(theme.accent),
                            );
                            if ui.button(egui::RichText::new("[ Copy ]").size(11.0)).clicked() {
                                ui.output_mut(|o| o.copied_text = target_id.to_string());
                            }
                        });

                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Encryption:")
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                        ui.label(
                            egui::RichText::new("Double Ratchet + XChaCha20-Poly1305")
                                .size(11.0)
                                .color(theme.online_indicator),
                        );
                    });

                ui.add_space(14.0);

                // Group Members list if group
                if let Some(g) = group {
                    ui.label(
                        egui::RichText::new("Group Members")
                            .size(13.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.add_space(6.0);

                    for member in g.members.values() {
                        ui.horizontal(|ui| {
                            draw_avatar(ui, &member.user_id_hex, &member.display_name, 28.0, true);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&member.display_name)
                                        .size(13.0)
                                        .color(theme.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:?}", member.role))
                                        .size(10.0)
                                        .color(theme.text_muted),
                                );
                            });
                        });
                        ui.add_space(4.0);
                    }
                    ui.add_space(14.0);
                }

                // Shared Media Tabs
                ui.horizontal(|ui| {
                    let tabs = [
                        (SharedMediaTab::Media, "Media"),
                        (SharedMediaTab::Files, "Files"),
                        (SharedMediaTab::Voice, "Voice"),
                        (SharedMediaTab::Links, "Links"),
                    ];
                    for (tab, label) in tabs {
                        let is_active = state.shared_media_tab == tab;
                        let text = egui::RichText::new(label)
                            .size(12.0)
                            .color(if is_active { theme.accent } else { theme.text_muted });
                        if ui.selectable_label(is_active, text).clicked() {
                            state.shared_media_tab = tab;
                        }
                    }
                });

                ui.add_space(10.0);

                // Filtered Messages in current chat
                match state.shared_media_tab {
                    SharedMediaTab::Voice => {
                        let voice_msgs: Vec<_> = messages.iter().filter(|m| m.voice_note.is_some()).collect();
                        if voice_msgs.is_empty() {
                            ui.label(egui::RichText::new("No voice notes.").size(12.0).color(theme.text_muted));
                        } else {
                            for v in voice_msgs {
                                if let Some(vn) = &v.voice_note {
                                    ui.horizontal(|ui| {
                                        ui.label("🎙");
                                        ui.label(
                                            egui::RichText::new(format!("Voice Note ({:.1}s)", vn.duration_secs))
                                                .size(12.0)
                                                .color(theme.text_primary),
                                        );
                                    });
                                    ui.add_space(4.0);
                                }
                            }
                        }
                    }
                    _ => {
                        ui.label(egui::RichText::new("No media shared yet.").size(12.0).color(theme.text_muted));
                    }
                }
            });
    });
}
