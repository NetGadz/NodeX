use egui::{Color32, Rounding, ScrollArea, Sense, Stroke, Ui, Vec2};
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use nodex_messenger::groups::P2PGroup;
use crate::components::avatar::draw_avatar;
use crate::components::icons::draw_search_icon;
use crate::state::{AppState, FolderFilter};
use crate::theme::{BORDER_COLOR, Theme};

pub enum SidebarAction {
    ClearHistory(String),
    DeleteChat(String),
    DeleteGroup(String),
}

pub fn render_sidebar(
    ui: &mut Ui,
    state: &mut AppState,
    theme: &Theme,
    contacts: &[SavedContact],
    groups: &[P2PGroup],
    messages: &[SavedChatMessage],
    action_out: &mut Option<SidebarAction>,
) {
    ui.vertical(|ui| {
        // 1. Search Bar & [ Add Peer ] Action Button
        ui.horizontal(|ui| {
            let search_icon_rect = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover()).0;
            draw_search_icon(&ui.painter(), search_icon_rect, theme.text_muted);

            ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search...")
                    .desired_width(ui.available_width() - 95.0),
            );
            if !state.search_query.is_empty() && ui.button("✕").clicked() {
                state.search_query.clear();
            }

            // Swiss Minimalist [ Add Peer ] button
            let add_peer_btn = egui::Button::new(
                egui::RichText::new("[ Add Peer ]").size(11.5).strong().color(theme.accent)
            )
            .fill(Color32::from_rgb(22, 23, 30))
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .rounding(Rounding::same(8.0));

            if ui.add(add_peer_btn).clicked() {
                state.modals.show_add_contact = true;
            }
        });

        ui.add_space(8.0);

        // 2. Folder Tabs (Flat underline style)
        ui.horizontal(|ui| {
            folder_tab_button(ui, "All", FolderFilter::All, &mut state.folder_filter, theme);
            folder_tab_button(ui, "Direct", FolderFilter::Direct, &mut state.folder_filter, theme);
            folder_tab_button(ui, "Groups", FolderFilter::Groups, &mut state.folder_filter, theme);
            folder_tab_button(ui, "Unread", FolderFilter::Unread, &mut state.folder_filter, theme);
        });

        ui.add_space(6.0);
        ui.separator();

        // 3. Conversation List
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // A) Always show "Saved Messages" (Избранное) item at the top
                let is_saved_selected = state.selected_chat_id.as_deref() == Some("self_saved_messages");
                let saved_messages: Vec<_> = messages
                    .iter()
                    .filter(|m| m.sender_id_hex == state.my_node_id && m.recipient_id_hex == state.my_node_id || m.recipient_id_hex == "self_saved_messages")
                    .collect();

                let last_saved_msg = saved_messages.last();
                let last_saved_text = last_saved_msg.map_or("Encrypted personal storage", |m| {
                    if m.voice_note.is_some() {
                        "Voice Note"
                    } else {
                        &m.text
                    }
                });
                let last_saved_time = last_saved_msg.map_or(0u64, |m| m.timestamp);

                let (saved_clicked, _) = draw_sidebar_item(
                    ui,
                    "self_saved_messages",
                    "Saved Messages",
                    last_saved_text,
                    last_saved_time as i64,
                    0,
                    true,
                    is_saved_selected,
                    theme,
                );

                if saved_clicked {
                    state.selected_chat_id = Some("self_saved_messages".to_string());
                    state.selected_group_id = None;
                }

                // B) Direct Contacts
                if state.folder_filter == FolderFilter::All || state.folder_filter == FolderFilter::Direct {
                    for contact in contacts {
                        if !state.search_query.is_empty()
                            && !contact.name.to_lowercase().contains(&state.search_query.to_lowercase())
                            && !contact.user_id_hex.contains(&state.search_query)
                        {
                            continue;
                        }

                        let last_msg = messages
                            .iter()
                            .rev()
                            .find(|m| m.sender_id_hex == contact.user_id_hex || m.recipient_id_hex == contact.user_id_hex);

                        let is_selected = state.selected_chat_id.as_deref() == Some(&contact.user_id_hex);

                        let last_text = last_msg.map_or("No messages", |m| {
                            if m.voice_note.is_some() {
                                "Voice Note"
                            } else {
                                &m.text
                            }
                        });

                        let last_time = last_msg.map_or(0u64, |m| m.timestamp);

                        let unread = messages
                            .iter()
                            .filter(|m| m.sender_id_hex == contact.user_id_hex && m.incoming && !m.delivered)
                            .count();

                        if state.folder_filter == FolderFilter::Unread && unread == 0 {
                            continue;
                        }

                        let (clicked, resp) = draw_sidebar_item(
                            ui,
                            &contact.user_id_hex,
                            &contact.name,
                            last_text,
                            last_time as i64,
                            unread,
                            true,
                            is_selected,
                            theme,
                        );

                        resp.context_menu(|ui| {
                            if ui.button("Clear History").clicked() {
                                *action_out = Some(SidebarAction::ClearHistory(contact.user_id_hex.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Delete Chat & Contact").clicked() {
                                *action_out = Some(SidebarAction::DeleteChat(contact.user_id_hex.clone()));
                                ui.close_menu();
                            }
                        });

                        if clicked {
                            state.selected_chat_id = Some(contact.user_id_hex.clone());
                            state.selected_group_id = None;
                        }
                    }
                }

                // C) Groups
                if state.folder_filter == FolderFilter::All || state.folder_filter == FolderFilter::Groups {
                    for group in groups {
                        if !state.search_query.is_empty()
                            && !group.title.to_lowercase().contains(&state.search_query.to_lowercase())
                        {
                            continue;
                        }

                        let is_selected = state.selected_group_id.as_deref() == Some(&group.group_id);

                        let last_msg = messages
                            .iter()
                            .rev()
                            .find(|m| m.recipient_id_hex == group.group_id);

                        let last_text = last_msg.map_or("Group created", |m| &m.text);
                        let last_time = last_msg.map_or(group.created_at, |m| m.timestamp);

                        let (clicked, resp) = draw_sidebar_item(
                            ui,
                            &group.group_id,
                            &group.title,
                            last_text,
                            last_time as i64,
                            0,
                            false,
                            is_selected,
                            theme,
                        );

                        resp.context_menu(|ui| {
                            if ui.button("Leave / Delete Group").clicked() {
                                *action_out = Some(SidebarAction::DeleteGroup(group.group_id.clone()));
                                ui.close_menu();
                            }
                        });

                        if clicked {
                            state.selected_group_id = Some(group.group_id.clone());
                            state.selected_chat_id = None;
                        }
                    }
                }
            });
    });
}

fn folder_tab_button(
    ui: &mut Ui,
    label: &str,
    tab: FolderFilter,
    current_tab: &mut FolderFilter,
    theme: &Theme,
) {
    let is_active = *current_tab == tab;
    let text = egui::RichText::new(label)
        .size(12.0)
        .color(if is_active { theme.accent } else { theme.text_muted });

    if ui.selectable_label(is_active, text).clicked() {
        *current_tab = tab;
    }
}

fn draw_sidebar_item(
    ui: &mut Ui,
    id_key: &str,
    title: &str,
    last_text: &str,
    timestamp: i64,
    unread_count: usize,
    is_online: bool,
    is_selected: bool,
    theme: &Theme,
) -> (bool, egui::Response) {
    let mut clicked = false;
    let item_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_w, 58.0), Sense::click());

    if resp.clicked() {
        clicked = true;
    }

    let is_hover = resp.hovered();
    let bg_color = if is_selected {
        Color32::from_rgb(26, 28, 36)
    } else if is_hover {
        Color32::from_rgb(22, 23, 30)
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(rect, Rounding::same(8.0), bg_color);
    if is_selected {
        ui.painter().rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, theme.accent));
    } else if is_hover {
        ui.painter().rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, BORDER_COLOR));
    }

    let inner_rect = rect.shrink2(egui::vec2(8.0, 6.0));
    let mut child_ui = ui.child_ui(
        inner_rect,
        egui::Layout::left_to_right(egui::Align::Center),
        None,
    );

    child_ui.horizontal(|ui| {
        draw_avatar(ui, id_key, title, 38.0, is_online);
        ui.add_space(8.0);

        ui.vertical(|ui| {
            ui.set_width(ui.available_width());

            // Top Row: Title + Time
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(13.5)
                        .strong()
                        .color(theme.text_primary),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if timestamp > 0 {
                        let dt = chrono::DateTime::from_timestamp(timestamp, 0)
                            .unwrap_or_else(|| chrono::Utc::now());
                        ui.label(
                            egui::RichText::new(dt.format("%H:%M").to_string())
                                .size(10.5)
                                .color(theme.text_muted),
                        );
                    }
                });
            });

            ui.add_space(2.0);

            // Bottom Row: Last message preview + Unread badge
            ui.horizontal(|ui| {
                let short_text = crate::theme::truncate_str(last_text, 26);
                ui.label(
                    egui::RichText::new(short_text)
                        .size(11.5)
                        .color(theme.text_muted),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if unread_count > 0 {
                        let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        ui.painter().rect_filled(badge_rect, Rounding::same(8.0), theme.accent);
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            unread_count.to_string(),
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                    }
                });
            });
        });
    });

    (clicked, resp)
}
