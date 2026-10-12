//! Central colour palette for the whole TUI.
//!
//! Kept in one place so a "theme" is a single struct rather than a colour
//! literal scattered through the drawing code. Defaults form a near-black
//! canvas with a violet accent; a user can override any entry under `"theme"`
//! in `soracode.jsonc`, given as `#rrggbb`.

use std::sync::OnceLock;

use ratatui::style::Color;

/// The resolved palette, filled once at startup from the config overrides.
#[derive(Clone)]
pub(super) struct Theme {
    pub(super) bg: Color,
    pub(super) panel_bg: Color,
    pub(super) selection_bg: Color,
    pub(super) text: Color,
    pub(super) text_muted: Color,
    pub(super) text_dim: Color,
    pub(super) accent: Color,
    pub(super) interactive: Color,
    pub(super) border: Color,
    pub(super) border_active: Color,
    pub(super) success: Color,
    pub(super) error: Color,
    pub(super) warning: Color,
    pub(super) info: Color,
    pub(super) role_user: Color,
    pub(super) role_assistant: Color,
    pub(super) role_tool: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(17, 17, 22),
            panel_bg: Color::Rgb(24, 24, 31),
            selection_bg: Color::Rgb(45, 42, 62),
            text: Color::Rgb(232, 230, 236),
            text_muted: Color::Rgb(130, 127, 145),
            text_dim: Color::Rgb(80, 79, 90),
            accent: Color::Rgb(167, 139, 250),
            interactive: Color::Rgb(250, 178, 131),
            border: Color::Rgb(48, 48, 58),
            border_active: Color::Rgb(167, 139, 250),
            success: Color::Rgb(134, 222, 150),
            error: Color::Rgb(255, 123, 137),
            warning: Color::Rgb(247, 178, 103),
            info: Color::Rgb(122, 194, 240),
            role_user: Color::Rgb(167, 139, 250),
            role_assistant: Color::Rgb(134, 222, 150),
            role_tool: Color::Rgb(122, 194, 240),
        }
    }
}

static THEME: OnceLock<Theme> = OnceLock::new();

/// Install the palette for this run. Later calls are ignored.
pub(super) fn install(t: Theme) {
    let _ = THEME.set(t);
}

/// Build a palette from config overrides, keeping defaults for absent or
/// malformed slots.
pub(super) fn from_map(map: &std::collections::BTreeMap<String, String>) -> Theme {
    let mut t = Theme::default();
    for (key, val) in map {
        let slot: &mut Color = match key.as_str() {
            "bg" => &mut t.bg,
            "panel_bg" => &mut t.panel_bg,
            "selection_bg" => &mut t.selection_bg,
            "text" => &mut t.text,
            "text_muted" => &mut t.text_muted,
            "text_dim" => &mut t.text_dim,
            "accent" => &mut t.accent,
            "interactive" => &mut t.interactive,
            "border" => &mut t.border,
            "border_active" => &mut t.border_active,
            "success" => &mut t.success,
            "error" => &mut t.error,
            "warning" => &mut t.warning,
            "info" => &mut t.info,
            "role_user" => &mut t.role_user,
            "role_assistant" => &mut t.role_assistant,
            "role_tool" => &mut t.role_tool,
            _ => continue,
        };
        *slot = parse_hex(val, *slot);
    }
    t
}

fn t() -> &'static Theme {
    THEME.get_or_init(Theme::default)
}

/// Parse `#rrggbb` or `#rgb` into a colour, falling back to `default`.
pub(super) fn parse_hex(s: &str, default: Color) -> Color {
    let h = s.trim().trim_start_matches('#');
    let expand = |digits: &str| u8::from_str_radix(digits, 16).ok();
    let (r, g, b) = match h.len() {
        6 => (expand(&h[0..2]), expand(&h[2..4]), expand(&h[4..6])),
        3 => {
            let d = |i| expand(&h[i..i + 1]).map(|v| v * 17);
            (d(0), d(1), d(2))
        }
        _ => (None, None, None),
    };
    match (r, g, b) {
        (Some(r), Some(g), Some(b)) => Color::Rgb(r, g, b),
        _ => default,
    }
}

// ── canvas ─────────────────────────────────────────────────────────────────
pub(super) fn bg() -> Color {
    t().bg
}
pub(super) fn panel_bg() -> Color {
    t().panel_bg
}
pub(super) fn selection_bg() -> Color {
    t().selection_bg
}

// ── text ───────────────────────────────────────────────────────────────────
pub(super) fn text() -> Color {
    t().text
}
pub(super) fn text_muted() -> Color {
    t().text_muted
}
pub(super) fn text_dim() -> Color {
    t().text_dim
}

// ── accent ─────────────────────────────────────────────────────────────────
pub(super) fn accent() -> Color {
    t().accent
}
pub(super) fn interactive() -> Color {
    t().interactive
}

// ── lines ──────────────────────────────────────────────────────────────────
pub(super) fn border() -> Color {
    t().border
}
pub(super) fn border_active() -> Color {
    t().border_active
}

// ── status ─────────────────────────────────────────────────────────────────
pub(super) fn success() -> Color {
    t().success
}
pub(super) fn error() -> Color {
    t().error
}
pub(super) fn warning() -> Color {
    t().warning
}
pub(super) fn info() -> Color {
    t().info
}

// ── roles ──────────────────────────────────────────────────────────────────
pub(super) fn role_user() -> Color {
    t().role_user
}
pub(super) fn role_assistant() -> Color {
    t().role_assistant
}
pub(super) fn role_tool() -> Color {
    t().role_tool
}

// ── severity ───────────────────────────────────────────────────────────────
/// Finding severity colors, keyed by the lowercase badge text.
pub(super) fn severity(badge: &str) -> Color {
    match badge {
        "critical" => Color::Rgb(255, 92, 118),
        "high" => error(),
        "medium" => warning(),
        "low" => info(),
        _ => text_muted(),
    }
}
