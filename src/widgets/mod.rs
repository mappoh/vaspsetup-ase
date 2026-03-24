//! Shared TUI widgets used by every screen.
//!
//! These enforce visual consistency across the entire application:
//! - Same header format on every screen
//! - Same footer format on every screen
//! - Same color scheme everywhere
//! - Same layout split (header | content | footer)

pub mod file_list;
pub mod footer;
pub mod header;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};

// ── Design system colors ──────────────────────────────────────────
pub const COLOR_TEXT: Color = Color::White;
pub const COLOR_HIGHLIGHT: Color = Color::Cyan;
pub const COLOR_SUCCESS: Color = Color::Green;
pub const COLOR_ERROR: Color = Color::Red;
pub const COLOR_MUTED: Color = Color::DarkGray;

pub const STYLE_HEADER: Style = Style::new().fg(COLOR_TEXT).add_modifier(Modifier::BOLD);
pub const STYLE_MUTED: Style = Style::new().fg(COLOR_MUTED);
pub const STYLE_HIGHLIGHT: Style = Style::new().fg(COLOR_HIGHLIGHT);
pub const STYLE_SUCCESS: Style = Style::new().fg(COLOR_SUCCESS);
pub const STYLE_ERROR: Style = Style::new().fg(COLOR_ERROR);
pub const COLOR_ACTION_BG: Color = Color::Rgb(95, 160, 175);
pub const COLOR_ACTION_FG: Color = Color::Rgb(30, 50, 55);
pub const STYLE_ACTION: Style = Style::new().fg(COLOR_ACTION_FG).bg(COLOR_ACTION_BG).add_modifier(Modifier::BOLD);

// ── Standard layout ──────────────────────────────────────────────

/// Split a screen area into header (4 lines), content (flexible), footer (2 lines).
/// Every screen uses this same split for consistency.
pub fn screen_layout(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(4), // header: sep + title + description + sep/badge
        Constraint::Min(1),   // content: screen-specific
        Constraint::Length(2), // footer: separator + navigation keys
    ])
    .areas(area)
}
