use egui::{Color32, Pos2, Rect, Stroke};

/// Draw clean, sleek background patterns on the chat canvas
pub fn draw_chat_wallpaper(
    painter: &egui::Painter,
    rect: Rect,
    base_color: Color32,
    wallpaper_type: &str,
    accent: Color32,
) {
    // Fill base
    painter.rect_filled(rect, egui::Rounding::ZERO, base_color);

    match wallpaper_type {
        "mesh" => {
            // Elegant subtle cyber mesh grid
            let step = 32.0_f32;
            let grid_color = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 10);
            let stroke = Stroke::new(1.0_f32, grid_color);

            let mut x = rect.min.x;
            while x <= rect.max.x {
                painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], stroke);
                x += step;
            }

            let mut y = rect.min.y;
            while y <= rect.max.y {
                painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], stroke);
                y += step;
            }
        }
        "hexagons" => {
            // Tech hexagon dot matrix
            let step = 28.0;
            let dot_color = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 18);
            let mut row = 0;
            let mut y = rect.min.y;
            while y <= rect.max.y {
                let offset_x = if row % 2 == 0 { 0.0 } else { step * 0.5 };
                let mut x = rect.min.x + offset_x;
                while x <= rect.max.x {
                    painter.circle_filled(Pos2::new(x, y), 1.2, dot_color);
                    x += step;
                }
                y += step * 0.866;
                row += 1;
            }
        }
        _ => {
            // "solid" - clean solid base color
        }
    }
}

