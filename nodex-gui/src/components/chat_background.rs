use egui::{Color32, Rect};

/// Draw clean, sleek Telegram-style background on the chat canvas
pub fn draw_chat_wallpaper(painter: &egui::Painter, rect: Rect, base_color: Color32) {
    // Fill clean solid background matching Telegram dark theme
    painter.rect_filled(rect, egui::Rounding::ZERO, base_color);
}

