use egui::{Color32, Pos2, Rect, Ui, Vec2, Rounding};
use qrcode::{QrCode, Version, EcLevel};

pub fn draw_qr_code(ui: &mut Ui, text: &str, target_size: f32) {
    if let Ok(code) = QrCode::with_version(text.as_bytes(), Version::Normal(4), EcLevel::M) {
        let width = code.width();
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(target_size), egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // White background border
        painter.rect_filled(rect, Rounding::same(8.0), Color32::WHITE);

        let pad = 12.0;
        let qr_area = rect.shrink(pad);
        let cell_size = qr_area.width() / width as f32;

        for y in 0..width {
            for x in 0..width {
                if code[(x, y)] == qrcode::Color::Dark {
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(qr_area.min.x + (x as f32 * cell_size), qr_area.min.y + (y as f32 * cell_size)),
                        Vec2::splat(cell_size + 0.5), // slight overlap to prevent anti-aliasing gaps
                    );
                    painter.rect_filled(cell_rect, Rounding::ZERO, Color32::BLACK);
                }
            }
        }
    } else {
        ui.label(egui::RichText::new("[QR Code Generation Error]").color(Color32::RED));
    }
}
