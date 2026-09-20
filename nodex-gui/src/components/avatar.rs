use egui::{Align2, Color32, FontId, Response, Rounding, Sense, Stroke, Ui, Vec2};
use crate::theme::{BORDER_COLOR, TEXT_SECONDARY};

pub fn draw_avatar(ui: &mut Ui, id_key: &str, display_name: &str, size: f32, is_online: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter_at(rect);

    let is_saved_messages = id_key == "self_saved_messages" || display_name.contains("Saved Messages");
    let rounding = Rounding::same(size * 0.28);

    if is_saved_messages {
        let bg_color = Color32::from_rgb(37, 99, 235); // Sleek Royal Blue
        painter.rect_filled(rect, rounding, bg_color);
        painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, Color32::from_rgb(59, 130, 246)));
        crate::components::icons::draw_star_bookmark_icon(&painter, rect, Color32::WHITE);
    } else {
        let bg_color = crate::theme::get_avatar_color(id_key);
        painter.rect_filled(rect, rounding, bg_color);
        painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, BORDER_COLOR));

        let center = rect.center();
        let badge_text = extract_badge_text(display_name, id_key);
        let font_size = (size * 0.38).clamp(11.0, 16.0);
        painter.text(
            center,
            Align2::CENTER_CENTER,
            badge_text,
            FontId::proportional(font_size),
            Color32::WHITE,
        );
    }

    if is_online {
        let dot_r = (size * 0.14).clamp(3.0, 6.0);
        let dot_pos = egui::pos2(rect.max.x - dot_r * 0.8, rect.max.y - dot_r * 0.8);
        painter.circle_filled(dot_pos, dot_r + 1.5, Color32::from_rgb(19, 23, 34));
        painter.circle_filled(dot_pos, dot_r, Color32::from_rgb(16, 185, 129));
    }

    response
}

fn extract_badge_text(name: &str, id_key: &str) -> String {
    let clean: String = name.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect();
    let trimmed = clean.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("User") {
        if id_key.len() >= 4 {
            return id_key[..4].to_uppercase();
        }
    }
    
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() >= 2 {
        let first = parts[0].chars().next().unwrap_or('?');
        let second = parts[1].chars().next().unwrap_or(' ');
        format!("{}{}", first.to_uppercase(), second.to_uppercase()).trim().to_string()
    } else if let Some(first_char) = trimmed.chars().next() {
        if trimmed.chars().count() >= 2 {
            trimmed.chars().take(2).collect::<String>().to_uppercase()
        } else {
            first_char.to_uppercase().to_string()
        }
    } else if id_key.len() >= 4 {
        id_key[..4].to_uppercase()
    } else {
        "?".to_string()
    }
}

/// Draw a 5x5 symmetric geometric Crypto-Identicon based on Node ID hash
pub fn draw_crypto_identicon(ui: &mut Ui, id_hex: &str, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect);

    let seed = id_hex.bytes().fold(0x9e3779b9u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let cell_color = TEXT_SECONDARY;
    let rounding = Rounding::same(size * 0.22);

    painter.rect_filled(rect, rounding, Color32::from_rgb(24, 29, 42));
    painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, BORDER_COLOR));

    let grid_size = 5;
    let margin = size * 0.14;
    let cell_size = (size - 2.0 * margin) / grid_size as f32;

    for row in 0..grid_size {
        for col in 0..=2 {
            let bit_idx = (row * 3 + col) % 32;
            let is_filled = ((seed >> bit_idx) & 1) == 1;

            if is_filled {
                let c_left = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + margin + col as f32 * cell_size, rect.min.y + margin + row as f32 * cell_size),
                    Vec2::splat(cell_size * 0.85),
                );
                painter.rect_filled(c_left, Rounding::same(2.0), cell_color);

                let sym_col = grid_size - 1 - col;
                if sym_col != col {
                    let c_right = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + margin + sym_col as f32 * cell_size, rect.min.y + margin + row as f32 * cell_size),
                        Vec2::splat(cell_size * 0.85),
                    );
                    painter.rect_filled(c_right, Rounding::same(2.0), cell_color);
                }
            }
        }
    }

    response
}

