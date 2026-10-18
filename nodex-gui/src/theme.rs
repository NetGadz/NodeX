#![allow(dead_code)]
use egui::{Color32, Rounding, Stroke, Style, Visuals};
use crate::state::ThemeMode;

// Telegram Desktop / Modern Charcoal Slate Palette (Zero Neon)
#[allow(dead_code)]
pub const BG_NAV_RAIL: Color32 = Color32::from_rgb(17, 24, 34);   // #111822
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(23, 33, 43);    // #17212B (Telegram dark sidebar)
pub const BG_CHAT: Color32 = Color32::from_rgb(14, 22, 33);       // #0E1621 (Telegram dark chat canvas)
pub const BG_CARD: Color32 = Color32::from_rgb(27, 39, 51);       // #1B2733
pub const BG_INPUT: Color32 = Color32::from_rgb(23, 33, 43);      // #17212B

// Authentic Telegram message bubbles
pub const BUBBLE_OUTGOING: Color32 = Color32::from_rgb(43, 82, 120); // #2B5278 Calm Slate Blue
pub const BUBBLE_INCOMING: Color32 = Color32::from_rgb(24, 34, 45);  // #18222D Deep Slate Gray

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(82, 136, 193);  // #5288C1 Calm Telegram Blue
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(78, 174, 134); // #4EAE86 Natural Emerald
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(245, 245, 245); // #F5F5F5
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 195, 210); // #B4C3D2
pub const TEXT_MUTED: Color32 = Color32::from_rgb(127, 145, 164);     // #7F91A4
pub const ONLINE_GREEN: Color32 = Color32::from_rgb(78, 174, 134);    // #4EAE86 Soft Emerald
pub const BORDER_COLOR: Color32 = Color32::from_rgb(36, 47, 61);      // #242F3D

pub fn get_avatar_color(key: &str) -> Color32 {
    let hash = key.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let colors = [
        Color32::from_rgb(51, 102, 153),  // Muted Slate Blue
        Color32::from_rgb(102, 68, 136),  // Muted Purple
        Color32::from_rgb(46, 117, 112),  // Muted Teal
        Color32::from_rgb(148, 92, 42),   // Muted Amber
        Color32::from_rgb(140, 54, 76),   // Muted Rose
        Color32::from_rgb(70, 86, 138),   // Muted Indigo
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
                nav_bg: Color32::from_rgb(13, 17, 24),       // Refined Obsidian
                sidebar_bg: Color32::from_rgb(18, 24, 34),    // Deep Navy Slate
                window_bg: Color32::from_rgb(13, 17, 24),     // Seamless canvas
                card_bg: Color32::from_rgb(22, 30, 43),       // Elevated Card
                bubble_outgoing_bg: Color32::from_rgb(37, 99, 235), // Electric Vibrant Blue
                bubble_incoming_bg: Color32::from_rgb(28, 38, 52),  // Polished incoming slate
                bubble_other_bg: Color32::from_rgb(24, 33, 46),
                accent: Color32::from_rgb(59, 130, 246),      // Vivid Accent Blue
                text_primary: Color32::from_rgb(248, 250, 252),
                text_muted: Color32::from_rgb(148, 163, 184),
                border_color: Color32::from_rgb(34, 46, 64),  // Crisp modern border
                online_indicator: Color32::from_rgb(34, 197, 94), // Emerald green
            },
            ThemeMode::Midnight => Self {
                mode,
                nav_bg: Color32::from_rgb(9, 11, 16),
                sidebar_bg: Color32::from_rgb(13, 16, 23),
                window_bg: Color32::from_rgb(7, 9, 13),
                card_bg: Color32::from_rgb(17, 22, 31),
                bubble_outgoing_bg: Color32::from_rgb(30, 64, 175),
                bubble_incoming_bg: Color32::from_rgb(17, 24, 39),
                bubble_other_bg: Color32::from_rgb(19, 25, 36),
                accent: Color32::from_rgb(59, 130, 246),
                text_primary: Color32::from_rgb(250, 250, 250),
                text_muted: Color32::from_rgb(107, 114, 128),
                border_color: Color32::from_rgb(31, 41, 55),
                online_indicator: Color32::from_rgb(34, 197, 94),
            },
            ThemeMode::Matrix => Self {
                mode,
                nav_bg: Color32::from_rgb(5, 16, 11),
                sidebar_bg: Color32::from_rgb(9, 26, 18),
                window_bg: Color32::from_rgb(5, 16, 11),
                card_bg: Color32::from_rgb(14, 38, 27),
                bubble_outgoing_bg: Color32::from_rgb(5, 150, 105),
                bubble_incoming_bg: Color32::from_rgb(15, 42, 30),
                bubble_other_bg: Color32::from_rgb(12, 33, 24),
                accent: Color32::from_rgb(16, 185, 129),
                text_primary: Color32::from_rgb(236, 253, 245),
                text_muted: Color32::from_rgb(110, 231, 183),
                border_color: Color32::from_rgb(19, 78, 57),
                online_indicator: Color32::from_rgb(52, 211, 153),
            },
            ThemeMode::Day => Self {
                mode,
                nav_bg: Color32::from_rgb(238, 242, 246),
                sidebar_bg: Color32::from_rgb(244, 246, 249),
                window_bg: Color32::from_rgb(255, 255, 255),
                card_bg: Color32::from_rgb(245, 247, 250),
                bubble_outgoing_bg: Color32::from_rgb(238, 255, 222), // Soft WhatsApp/Telegram day green
                bubble_incoming_bg: Color32::from_rgb(255, 255, 255),
                bubble_other_bg: Color32::from_rgb(240, 242, 245),
                accent: Color32::from_rgb(51, 144, 236),
                text_primary: Color32::from_rgb(26, 32, 44),
                text_muted: Color32::from_rgb(113, 128, 150),
                border_color: Color32::from_rgb(226, 232, 240),
                online_indicator: Color32::from_rgb(78, 174, 134),
            },
        }
    }

    pub fn from_mode_with_accent(mode: ThemeMode, custom_accent: Option<[u8; 3]>) -> Self {
        let mut t = Self::from_mode(mode);
        if let Some([r, g, b]) = custom_accent {
            let col = Color32::from_rgb(r, g, b);
            t.accent = col;
            t.bubble_outgoing_bg = col;
        }
        t
    }

    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut visuals = match self.mode {
            ThemeMode::Day => Visuals::light(),
            ThemeMode::Dark | ThemeMode::Midnight | ThemeMode::Matrix => Visuals::dark(),
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
