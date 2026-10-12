//! Central colour palette for the whole TUI.
//!
//! Kept in one place so a "theme" is a single file rather than a colour literal
//! scattered through the drawing code. The base is a near-black canvas with a
//! violet accent; status colours (success/error/warning) sit on top.

use ratatui::style::Color;

// ── canvas ─────────────────────────────────────────────────────────────────
/// The window background.
pub(super) const BG: Color = Color::Rgb(17, 17, 22);
/// A raised surface: panels, palettes, the input box.
pub(super) const PANEL_BG: Color = Color::Rgb(24, 24, 31);
/// The selected row inside a list.
pub(super) const SELECTION_BG: Color = Color::Rgb(45, 42, 62);

// ── text ───────────────────────────────────────────────────────────────────
/// Primary text.
pub(super) const TEXT: Color = Color::Rgb(232, 230, 236);
/// Secondary text: labels, hints, separators.
pub(super) const TEXT_MUTED: Color = Color::Rgb(130, 127, 145);
/// Tertiary text: barely there, for rule lines and trailing info.
pub(super) const TEXT_DIM: Color = Color::Rgb(80, 79, 90);

// ── accent ─────────────────────────────────────────────────────────────────
/// The brand accent, used for titles and the active focus ring.
pub(super) const ACCENT: Color = Color::Rgb(167, 139, 250);
/// The interactive/secondary accent (warm), used for input and movement.
pub(super) const INTERACTIVE: Color = Color::Rgb(250, 178, 131);

// ── lines ──────────────────────────────────────────────────────────────────
/// Idle borders.
pub(super) const BORDER: Color = Color::Rgb(48, 48, 58);
/// The border of the pane the user is working in.
pub(super) const BORDER_ACTIVE: Color = Color::Rgb(167, 139, 250);

// ── status ─────────────────────────────────────────────────────────────────
pub(super) const SUCCESS: Color = Color::Rgb(134, 222, 150);
pub(super) const ERROR: Color = Color::Rgb(255, 123, 137);
pub(super) const WARNING: Color = Color::Rgb(247, 178, 103);
/// Informational callouts (paths, IDs, secondary structure).
pub(super) const INFO: Color = Color::Rgb(122, 194, 240);

// ── roles ──────────────────────────────────────────────────────────────────
/// The user's own message.
pub(super) const ROLE_USER: Color = Color::Rgb(167, 139, 250);
/// The assistant's reply.
pub(super) const ROLE_ASSISTANT: Color = Color::Rgb(134, 222, 150);
/// A tool call or its result.
pub(super) const ROLE_TOOL: Color = Color::Rgb(122, 194, 240);

// ── severity ───────────────────────────────────────────────────────────────
/// Finding severity colors, keyed by the lowercase badge text.
pub(super) fn severity(badge: &str) -> Color {
    match badge {
        "critical" => Color::Rgb(255, 92, 118),
        "high" => Color::Rgb(255, 123, 137),
        "medium" => Color::Rgb(247, 178, 103),
        "low" => Color::Rgb(122, 194, 240),
        _ => TEXT_MUTED,
    }
}
