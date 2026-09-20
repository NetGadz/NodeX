use egui::{Align2, Color32, FontId, Rounding, Sense, Stroke, Ui, Vec2};
use crate::components::icons::draw_gear_icon;
use crate::state::{ActiveNavTab, AppState};
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_nav_rail(ui: &mut Ui, state: &mut AppState, theme: &Theme) {
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);

        // App Logo Icon (NodeX Photo Logo from Assets)
        crate::components::icons::render_nodex_logo_widget(ui, 38.0);

        ui.add_space(16.0);

        // 1. Chats
        nav_item(ui, "💬", "Chats", ActiveNavTab::Chats, state.active_nav_tab == ActiveNavTab::Chats, theme, || {
            state.active_nav_tab = ActiveNavTab::Chats;
            state.folder_filter = crate::state::FolderFilter::All;
        });

        // 2. Groups
        nav_item(ui, "👥", "Groups", ActiveNavTab::Groups, state.active_nav_tab == ActiveNavTab::Groups, theme, || {
            state.active_nav_tab = ActiveNavTab::Groups;
            state.folder_filter = crate::state::FolderFilter::Groups;
        });

        // 3. Stego Studio (Крипто-аватары и мемы)
        nav_item(ui, "🎭", "Stego Studio", ActiveNavTab::StegoStudio, state.active_nav_tab == ActiveNavTab::StegoStudio, theme, || {
            state.active_nav_tab = ActiveNavTab::StegoStudio;
        });

        // 4. Saved Messages (Избранное)
        nav_item(ui, "⭐", "Saved", ActiveNavTab::SavedMessages, state.active_nav_tab == ActiveNavTab::SavedMessages, theme, || {
            state.active_nav_tab = ActiveNavTab::SavedMessages;
            state.selected_chat_id = Some("self_saved_messages".to_string());
            state.selected_group_id = None;
        });

        // 5. Network DHT Diagnostics
        nav_item(ui, "🌐", "Network", ActiveNavTab::Network, state.active_nav_tab == ActiveNavTab::Network, theme, || {
            state.modals.show_network = true;
        });

        ui.add_space((ui.available_height() - 110.0).max(10.0));

        // Quick lock button
        if state.passcode_hash.is_some() {
            let (lock_rect, lock_resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
            let lock_painter = ui.painter_at(lock_rect);
            if lock_resp.hovered() {
                lock_painter.rect_filled(lock_rect, Rounding::same(8.0), theme.bubble_other_bg);
                lock_painter.rect_stroke(lock_rect, Rounding::same(8.0), Stroke::new(1.0_f32, BORDER_COLOR));
            }
            lock_painter.text(
                lock_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🔒",
                egui::FontId::proportional(17.0),
                if lock_resp.hovered() { theme.accent } else { theme.text_muted },
            );
            if lock_resp.clicked() {
                state.is_app_locked = true;
            }
            ui.add_space(4.0);
        }

        // 6. Settings at bottom
        let (gear_rect, gear_resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
        let gear_painter = ui.painter_at(gear_rect);

        let is_hover = gear_resp.hovered();
        let bg_color = if is_hover { theme.bubble_other_bg } else { Color32::TRANSPARENT };
        if is_hover {
            gear_painter.rect_filled(gear_rect, Rounding::same(8.0), bg_color);
            gear_painter.rect_stroke(gear_rect, Rounding::same(8.0), Stroke::new(1.0_f32, BORDER_COLOR));
        }

        draw_gear_icon(&gear_painter, gear_rect, if is_hover { theme.accent } else { theme.text_muted });

        if gear_resp.clicked() {
            state.modals.show_settings = true;
        }
    });
}

fn nav_item<F: FnOnce()>(
    ui: &mut Ui,
    icon: &str,
    _tooltip: &str,
    _tab: ActiveNavTab,
    is_active: bool,
    theme: &Theme,
    on_click: F,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    let painter = ui.painter_at(rect);

    if is_active {
        painter.rect_filled(rect, Rounding::same(8.0), theme.bubble_outgoing_bg);
        painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, theme.accent));
    } else if resp.hovered() {
        painter.rect_filled(rect, Rounding::same(8.0), theme.bubble_other_bg);
        painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, BORDER_COLOR));
    }

    let text_color = if is_active { Color32::WHITE } else { theme.text_muted };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(19.0),
        text_color,
    );

    if resp.clicked() {
        on_click();
    }

    ui.add_space(4.0);
}
