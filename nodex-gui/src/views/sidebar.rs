use egui::{Color32, Rounding, ScrollArea, Sense, Stroke, Ui, Vec2};
use nodex_messenger::db::{SavedChatMessage, SavedContact};
use nodex_messenger::groups::P2PGroup;
use crate::components::avatar::draw_avatar;
use crate::components::icons::draw_search_icon;
use crate::state::{AppState, FolderFilter};
use crate::theme::{BORDER_COLOR, Theme};

#[allow(dead_code)]
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
                    .hint_text("Поиск или Node ID...")
                    .desired_width((ui.available_width() - 170.0).max(60.0)),
            );
            if !state.search_query.is_empty() && ui.button("✕").clicked() {
                state.search_query.clear();
            }

            // Clean + Контакт button
            let add_peer_btn = egui::Button::new(
                egui::RichText::new("+ Контакт").size(11.0).strong().color(theme.accent)
            )
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .rounding(Rounding::same(8.0));

            if ui.add(add_peer_btn).clicked() {
                state.modals.show_add_contact = true;
            }

            // Clean + Группа button
            let add_group_btn = egui::Button::new(
                egui::RichText::new("+ Группа").size(11.0).strong().color(theme.accent)
            )
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
            .rounding(Rounding::same(8.0));

            if ui.add(add_group_btn).clicked() {
                state.modals.show_create_group = true;
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

                let (saved_clicked, _, _) = draw_sidebar_item(
                    ui,
                    "self_saved_messages",
                    "Избранное",
                    last_saved_text,
                    last_saved_time as i64,
                    0,
                    false,
                    is_saved_selected,
                    &mut state.swipe_offsets,
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

                        let is_online = state.is_peer_online(&contact.user_id_hex);

                        let (clicked, resp, delete_requested) = draw_sidebar_item(
                            ui,
                            &contact.user_id_hex,
                            &contact.name,
                            last_text,
                            last_time as i64,
                            unread,
                            is_online,
                            is_selected,
                            &mut state.swipe_offsets,
                            theme,
                        );

                        if delete_requested {
                            state.modals.show_delete_session_confirm = Some(contact.user_id_hex.clone());
                        }

                        resp.context_menu(|ui| {
                            if ui.button("Очистить историю").clicked() {
                                *action_out = Some(SidebarAction::ClearHistory(contact.user_id_hex.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Удалить сессию и контакт").clicked() {
                                state.modals.show_delete_session_confirm = Some(contact.user_id_hex.clone());
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

                        let (clicked, resp, delete_requested) = draw_sidebar_item(
                            ui,
                            &group.group_id,
                            &group.title,
                            last_text,
                            last_time as i64,
                            0,
                            false,
                            is_selected,
                            &mut state.swipe_offsets,
                            theme,
                        );

                        if delete_requested {
                            state.modals.show_delete_session_confirm = Some(group.group_id.clone());
                        }

                        resp.context_menu(|ui| {
                            if ui.button("Покинуть / Удалить группу").clicked() {
                                state.modals.show_delete_session_confirm = Some(group.group_id.clone());
                                ui.close_menu();
                            }
                        });

                        if clicked {
                            state.selected_group_id = Some(group.group_id.clone());
                            state.selected_chat_id = None;
                        }
                    }
                }

                // D) Global Search (Finding self or unadded DHT peer)
                if !state.search_query.trim().is_empty() {
                    let query = state.search_query.trim().to_lowercase();
                    let is_self_query = query == state.my_node_id.to_lowercase()
                        || (!state.my_name.is_empty() && state.my_name.to_lowercase() == query)
                        || query == "я"
                        || query == "me"
                        || query == "себе"
                        || query == "себя"
                        || query == "вы"
                        || query == "заметки"
                        || query == "избранное"
                        || query == "saved";

                    let matches_existing = contacts.iter().any(|c| c.user_id_hex.to_lowercase() == query);

                    if is_self_query && !matches_existing {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("ГЛОБАЛЬНЫЙ ПОИСК").size(10.5).color(theme.text_muted));
                        let is_sel = state.selected_chat_id.as_deref() == Some(&state.my_node_id);
                        let (clicked, _, _) = draw_sidebar_item(
                            ui,
                            &state.my_node_id,
                            &format!("{} (Вы)", state.my_name),
                            "Написать в свой личный узел",
                            0,
                            0,
                            false,
                            is_sel,
                            &mut state.swipe_offsets,
                            theme,
                        );
                        if clicked {
                            state.selected_chat_id = Some(state.my_node_id.clone());
                            state.selected_group_id = None;
                        }
                    } else if query.len() >= 4 && !matches_existing && query.chars().all(|c| c.is_ascii_hexdigit()) && query != state.my_node_id.to_lowercase() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("ГЛОБАЛЬНЫЙ ПОИСК").size(10.5).color(theme.text_muted));
                        let is_sel = state.selected_chat_id.as_deref() == Some(&query);
                        let (clicked, _, _) = draw_sidebar_item(
                            ui,
                            &query,
                            &format!("Узел {}", &query[..6.min(query.len())]),
                            "Начать P2P диалог...",
                            0,
                            0,
                            false,
                            is_sel,
                            &mut state.swipe_offsets,
                            theme,
                        );
                        if clicked {
                            state.selected_chat_id = Some(query.clone());
                            state.selected_group_id = None;
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
    swipe_offsets: &mut std::collections::HashMap<String, f32>,
    theme: &Theme,
) -> (bool, egui::Response, bool) {
    let mut clicked = false;
    let mut delete_requested = false;
    let item_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(item_w, 60.0), Sense::click_and_drag());

    // Swipe offset calculation
    let current_offset = swipe_offsets.entry(id_key.to_string()).or_insert(0.0);

    if resp.dragged() {
        let delta = resp.drag_delta().x;
        *current_offset = (*current_offset + delta).clamp(-90.0, 0.0);
    } else {
        // Snap to -75.0 if dragged sufficiently, otherwise spring back to 0.0
        if *current_offset < -35.0 {
            *current_offset = -75.0;
        } else {
            *current_offset = 0.0;
        }
    }

    let is_swiped_open = *current_offset <= -40.0;

    // Draw background delete reveal area behind the item when swiping left
    if *current_offset < -2.0 {
        let reveal_w = (-*current_offset).min(rect.width());
        let delete_box = egui::Rect::from_min_max(
            egui::pos2(rect.max.x - reveal_w, rect.min.y),
            rect.max,
        );
        // Red danger background
        ui.painter().rect_filled(delete_box, Rounding::same(10.0), Color32::from_rgb(220, 38, 38));

        // Trash icon + label
        if reveal_w >= 45.0 {
            ui.painter().text(
                delete_box.center() + egui::vec2(0.0, -6.0),
                egui::Align2::CENTER_CENTER,
                "🗑",
                egui::FontId::proportional(17.0),
                Color32::WHITE,
            );
            ui.painter().text(
                delete_box.center() + egui::vec2(0.0, 10.0),
                egui::Align2::CENTER_CENTER,
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
                delete_requested = true;
            }
            // Reset swipe on click
            *current_offset = 0.0;
        } else {
            clicked = true;
        }
    }

    // Now render the shifting card on top
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + *current_offset, rect.min.y),
        rect.size(),
    );

    let is_hover = resp.hovered();
    let bg_color = if is_selected {
        Color32::from_rgb(30, 41, 59) // Deep slate active #1E293B
    } else if is_hover {
        Color32::from_rgb(23, 32, 46) // Subtle highlight #17202E
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(content_rect, Rounding::same(10.0), bg_color);
    if is_hover && !is_selected {
        ui.painter().rect_stroke(content_rect, Rounding::same(10.0), Stroke::new(1.0_f32, Color32::from_rgb(39, 51, 68)));
    } else if is_selected {
        // Accent border on the left
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(content_rect.min.x + 2.0, content_rect.min.y + 12.0),
            egui::pos2(content_rect.min.x + 5.0, content_rect.max.y - 12.0),
        );
        ui.painter().rect_filled(bar_rect, Rounding::same(2.0), theme.accent);
    }

    let inner_rect = content_rect.shrink2(egui::vec2(10.0, 6.0));
    let mut child_ui = ui.child_ui(
        inner_rect,
        egui::Layout::left_to_right(egui::Align::Center),
        None,
    );

    child_ui.horizontal(|ui| {
        draw_avatar(ui, id_key, title, 40.0, is_online);
        ui.add_space(10.0);

        ui.vertical(|ui| {
            ui.set_width(ui.available_width());

            // Top Row: Title + Time
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(13.5)
                        .strong()
                        .color(if is_selected { Color32::WHITE } else { theme.text_primary }),
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

            ui.add_space(3.0);

            // Bottom Row: Last message preview + Unread badge
            ui.horizontal(|ui| {
                let short_text = crate::theme::truncate_str(last_text, 28);
                ui.label(
                    egui::RichText::new(short_text)
                        .size(11.5)
                        .color(if unread_count > 0 { Color32::from_rgb(203, 213, 225) } else { theme.text_muted }),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if unread_count > 0 {
                        let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        ui.painter().rect_filled(badge_rect, Rounding::same(9.0), theme.accent);
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            unread_count.to_string(),
                            egui::FontId::proportional(10.5),
                            Color32::WHITE,
                        );
                    }
                });
            });
        });
    });

    (clicked, resp, delete_requested)
}
