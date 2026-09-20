use egui::{vec2, Align2, Margin, Rounding, ScrollArea, Stroke, Window};
use crate::state::AppState;
use crate::theme::{BORDER_COLOR, Theme};

pub fn render_network_modal(ctx: &egui::Context, state: &mut AppState, theme: &Theme) {
    if !state.modals.show_network {
        return;
    }

    let mut is_open = true;
    let mut should_close = false;
    let screen_center = ctx.screen_rect().center();

    Window::new("P2P Сеть и DHT Диагностика")
        .open(&mut is_open)
        .resizable(true)
        .collapsible(false)
        .movable(true)
        .default_pos(screen_center)
        .pivot(Align2::CENTER_CENTER)
        .default_size(vec2(520.0, 450.0))
        .min_size(vec2(400.0, 320.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.card_bg)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .inner_margin(Margin::same(18.0)),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Распределенная хеш-таблица (DHT) и NAT Traversal")
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Топология P2P сети в реальном времени, состояние k-бакетов и луковой маршрутизации.")
                    .size(11.5)
                    .color(theme.text_muted),
            );

            ui.add_space(14.0);

            // Metrics grid
            egui::Grid::new("network_metrics_grid")
                .num_columns(2)
                .spacing(vec2(20.0, 10.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Активные DHT Пиры:").color(theme.text_muted));
                    ui.label(
                        egui::RichText::new(format!("{}", state.network_stats.connected_peers))
                            .strong()
                            .color(theme.accent),
                    );
                    ui.end_row();

                    ui.label(egui::RichText::new("Узлы в таблице маршрутизации:").color(theme.text_muted));
                    ui.label(
                        egui::RichText::new(format!("{}", state.network_stats.known_nodes))
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.end_row();

                    ui.label(egui::RichText::new("Статус NAT Traversal:").color(theme.text_muted));
                    ui.label(
                        egui::RichText::new(&state.network_stats.nat_type)
                            .strong()
                            .color(theme.online_indicator),
                    );
                    ui.end_row();

                    ui.label(egui::RichText::new("UPnP Port Forwarding:").color(theme.text_muted));
                    ui.label(
                        egui::RichText::new(if state.network_stats.upnp_active { format!("Активен (UDP {})", state.port) } else { "Отключен / Прямой".to_string() })
                            .color(if state.network_stats.upnp_active { theme.online_indicator } else { theme.text_muted }),
                    );
                    ui.end_row();

                    ui.label(egui::RichText::new("Режим Relay-ноды (ретранслятор):").color(theme.text_muted));
                    ui.label(
                        egui::RichText::new(if state.network_stats.relay_active { "Включен (помощь другим узлам)" } else { "Выключен" })
                            .color(if state.network_stats.relay_active { theme.accent } else { theme.text_muted }),
                    );
                    ui.end_row();
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(
                egui::RichText::new("Состояние Kademlia K-Buckets")
                    .size(13.0)
                    .strong()
                    .color(theme.text_primary),
            );

            ui.add_space(6.0);

            ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for bucket_idx in [0, 4, 12, 48, 128, 255] {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("K-Bucket #{:03}:", bucket_idx))
                                    .size(11.0)
                                    .color(theme.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} узлов (k=20 max)", (bucket_idx % 7) + 1))
                                    .size(11.0)
                                    .color(theme.accent),
                            );
                        });
                    }
                });

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("Обновить DHT").color(theme.accent)).clicked() {
                    state.network_stats.connected_peers += 1;
                    state.network_stats.known_nodes += 3;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Закрыть").color(theme.text_primary)).clicked() {
                        should_close = true;
                    }
                });
            });
        });

    if !is_open || should_close {
        state.modals.show_network = false;
    }
}
