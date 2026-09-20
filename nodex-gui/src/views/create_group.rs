use egui::{vec2, Align2, Color32, Margin, Rounding, ScrollArea, Stroke, Window};
use nodex_messenger::db::SavedContact;
use crate::components::avatar::draw_avatar;
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_create_group_modal(
    ctx: &egui::Context,
    state: &mut AppState,
    theme: &Theme,
    contacts: &[SavedContact],
    create_group_out: &mut Option<(String, Vec<String>)>,
) {
    if !state.modals.show_create_group {
        return;
    }

    let mut is_open = state.modals.show_create_group;
    let mut create_group_requested = false;
    let screen_center = ctx.screen_rect().center();

    Window::new("Создать P2P группу")
        .open(&mut is_open)
        .resizable(true)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(Align2::CENTER_CENTER)
        .default_size(vec2(460.0, 380.0))
        .min_size(vec2(360.0, 300.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::same(18.0)),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Новая зашифрованная Mesh-группа")
                    .size(13.5)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Все сообщения группы распределяются и шифруются сквозным P2P протоколом.")
                    .size(11.5)
                    .color(theme.text_muted),
            );

            ui.add_space(12.0);

            ui.label(egui::RichText::new("Название группы:").size(11.5).color(theme.text_primary));
            ui.add(
                egui::TextEdit::singleline(&mut state.new_group_title)
                    .hint_text("Например: Команда разработчиков")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);

            ui.label(
                egui::RichText::new("Выберите участников:")
                    .size(11.5)
                    .color(theme.text_primary),
            );

            ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    if contacts.is_empty() {
                        ui.label(egui::RichText::new("Контакты не найдены. Сначала добавьте пиров.").size(11.0).color(theme.text_muted));
                    } else {
                        for c in contacts {
                            let mut is_selected = state.selected_group_members.contains(&c.user_id_hex);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut is_selected, "").changed() {
                                    if is_selected {
                                        state.selected_group_members.insert(c.user_id_hex.clone());
                                    } else {
                                        state.selected_group_members.remove(&c.user_id_hex);
                                    }
                                }
                                draw_avatar(ui, &c.user_id_hex, &c.name, 22.0, true);
                                ui.label(egui::RichText::new(&c.name).size(12.5).color(theme.text_primary));
                            });
                            ui.add_space(4.0);
                        }
                    }
                });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("[ Отмена ]").color(theme.text_muted)).clicked() {
                    state.modals.show_create_group = false;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_create = !state.new_group_title.trim().is_empty();
                    let btn = egui::Button::new(
                        egui::RichText::new("Создать группу")
                            .color(if can_create { Color32::WHITE } else { theme.text_muted })
                            .strong(),
                    )
                    .fill(if can_create { theme.accent } else { theme.bubble_other_bg })
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .rounding(Rounding::same(8.0));

                    if ui.add_enabled(can_create, btn).clicked() {
                        create_group_requested = true;
                    }
                });
            });
        });

    if create_group_requested {
        let title = state.new_group_title.trim().to_string();
        if !title.is_empty() {
            let members: Vec<String> = state.selected_group_members.iter().cloned().collect();
            *create_group_out = Some((title, members));
            state.new_group_title.clear();
            state.selected_group_members.clear();
            state.modals.show_create_group = false;
        }
    }

    state.modals.show_create_group = is_open;
}
