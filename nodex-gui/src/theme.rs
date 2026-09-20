use egui::{Color32, Rounding, Stroke, Style, Visuals};
use crate::state::ThemeMode;

pub const BG_NAV_RAIL: Color32 = Color32::from_rgb(14, 17, 24);   // #0E1118
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(19, 23, 34);    // #131722
pub const BG_CHAT: Color32 = Color32::from_rgb(11, 13, 19);       // #0B0D13
pub const BG_CARD: Color32 = Color32::from_rgb(24, 29, 42);       // #181D2A
pub const BG_INPUT: Color32 = Color32::from_rgb(19, 23, 34);      // #131722

pub const BUBBLE_OUTGOING: Color32 = Color32::from_rgb(37, 99, 235); // #2563EB
pub const BUBBLE_INCOMING: Color32 = Color32::from_rgb(30, 36, 51);  // #1E2433

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);  // #3B82F6 Electric Azure
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(16, 185, 129); // #10B981 Soft Emerald
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252); // #F8FAFC
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 163, 184); // #94A3B8
pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 116, 139);     // #64748B
pub const ONLINE_GREEN: Color32 = Color32::from_rgb(16, 185, 129);    // #10B981
pub const BORDER_COLOR: Color32 = Color32::from_rgb(34, 41, 58);      // #22293A

pub fn get_avatar_color(key: &str) -> Color32 {
    let hash = key.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let colors = [
        Color32::from_rgb(37, 99, 235),  // Blue
        Color32::from_rgb(124, 58, 237), // Purple
        Color32::from_rgb(13, 148, 136), // Teal
        Color32::from_rgb(217, 119, 6),  // Amber
        Color32::from_rgb(225, 29, 72),  // Rose
        Color32::from_rgb(79, 70, 229),  // Indigo
    ];
    colors[(hash as usize) % colors.len()]
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub nav_bg: Color32,
    pub sidebar_bg: Color32,
    pub window_bg: Color32,
    pub card_bg: Color32,
    pub bubble_outgoing_bg: Color32,
    pub bubble_incoming_bg: Color32,
    pub bubble_other_bg: Color32,
    pub accent: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub border_color: Color32,
    pub online_indicator: Color32,
}

impl Theme {
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                nav_bg: Color32::from_rgb(14, 17, 24),
                sidebar_bg: Color32::from_rgb(19, 23, 34),
                window_bg: Color32::from_rgb(11, 13, 19),
                card_bg: Color32::from_rgb(24, 29, 42),
                bubble_outgoing_bg: Color32::from_rgb(37, 99, 235),
                bubble_incoming_bg: Color32::from_rgb(30, 36, 51),
                bubble_other_bg: Color32::from_rgb(24, 29, 42),
                accent: Color32::from_rgb(59, 130, 246),
                text_primary: Color32::from_rgb(248, 250, 252),
                text_muted: Color32::from_rgb(148, 163, 184),
                border_color: Color32::from_rgb(34, 41, 58),
                online_indicator: Color32::from_rgb(16, 185, 129),
            },
            ThemeMode::Midnight => Self {
                mode,
                nav_bg: Color32::from_rgb(4, 4, 6),
                sidebar_bg: Color32::from_rgb(9, 9, 12),
                window_bg: Color32::from_rgb(0, 0, 0),
                card_bg: Color32::from_rgb(16, 16, 22),
                bubble_outgoing_bg: Color32::from_rgb(99, 102, 241),
                bubble_incoming_bg: Color32::from_rgb(22, 22, 30),
                bubble_other_bg: Color32::from_rgb(16, 16, 22),
                accent: Color32::from_rgb(129, 140, 248),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_muted: Color32::from_rgb(113, 113, 122),
                border_color: Color32::from_rgb(39, 39, 50),
                online_indicator: Color32::from_rgb(52, 211, 153),
            },
            ThemeMode::Day => Self {
                mode,
                nav_bg: Color32::from_rgb(226, 232, 240),
                sidebar_bg: Color32::from_rgb(241, 245, 249),
                window_bg: Color32::from_rgb(255, 255, 255),
                card_bg: Color32::from_rgb(248, 250, 252),
                bubble_outgoing_bg: Color32::from_rgb(37, 99, 235),
                bubble_incoming_bg: Color32::from_rgb(226, 232, 240),
                bubble_other_bg: Color32::from_rgb(241, 245, 249),
                accent: Color32::from_rgb(37, 99, 235),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_muted: Color32::from_rgb(100, 116, 139),
                border_color: Color32::from_rgb(203, 213, 225),
                online_indicator: Color32::from_rgb(16, 185, 129),
            },
        }
    }

    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut visuals = match self.mode {
            ThemeMode::Day => Visuals::light(),
            ThemeMode::Dark | ThemeMode::Midnight => Visuals::dark(),
        };

        visuals.window_rounding = Rounding::same(12.0);
        visuals.menu_rounding = Rounding::same(8.0);
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
        visuals.widgets.inactive.rounding = Rounding::same(8.0);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);
        visuals.widgets.active.rounding = Rounding::same(8.0);
        visuals.widgets.open.rounding = Rounding::same(8.0);

        visuals.widgets.noninteractive.bg_fill = self.card_bg;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, self.border_color);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, self.text_primary);

        visuals.widgets.inactive.bg_fill = self.card_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, self.border_color);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, self.text_primary);

        visuals.widgets.hovered.bg_fill = self.bubble_other_bg;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, self.accent);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, self.text_primary);

        visuals.widgets.active.bg_fill = self.accent;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, self.accent);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

        visuals.window_fill = self.card_bg;
        visuals.window_stroke = Stroke::new(1.0_f32, self.border_color);
        visuals.panel_fill = self.sidebar_bg;
        visuals.faint_bg_color = self.nav_bg;
        visuals.extreme_bg_color = self.window_bg;
        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(self.accent.r(), self.accent.g(), self.accent.b(), 60);
        visuals.selection.stroke = Stroke::new(1.0_f32, self.accent);

        let style = Style {
            visuals,
            ..Default::default()
        };
        ctx.set_style(style);
    }
}

/// Configure initial global visuals
pub fn configure_swiss_minimal_style(ctx: &egui::Context) {
    let default_theme = Theme::from_mode(ThemeMode::Dark);
    default_theme.apply_to_ctx(ctx);
}

/// Safely truncate a UTF-8 string by character count
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Safely truncate an ID or key preserving leading and trailing chars
pub fn truncate_id(s: &str, front: usize, back: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > front + back + 3 {
        let start: String = chars[..front].iter().collect();
        let end: String = chars[chars.len() - back..].iter().collect();
        format!("{}...{}", start, end)
    } else {
        s.to_string()
    }
}
