//! Footer widget: navigation keys displayed in muted gray.
//! Same position and style on every screen.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{STYLE_HIGHLIGHT, STYLE_MUTED};

/// Render the footer with a separator line and navigation key hints.
///
/// Each key hint is formatted as "Key: Action" with the key in cyan
/// and the action in gray, separated by three spaces.
///
/// Example: `&[("↑↓", "Navigate"), ("Enter", "Select"), ("Esc", "Back"), ("q", "Quit")]`
pub fn render(frame: &mut Frame, area: Rect, keys: &[(&str, &str)]) {
    let mut spans = Vec::with_capacity(keys.len() * 4);

    for (i, (key, action)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(*key, STYLE_HIGHLIGHT));
        spans.push(Span::styled(": ", STYLE_MUTED));
        spans.push(Span::styled(*action, STYLE_MUTED));
    }

    let sep = Line::from(Span::styled(
        "─".repeat(area.width as usize),
        STYLE_MUTED,
    ));
    let footer = Paragraph::new(vec![sep, Line::from(spans)]);
    frame.render_widget(footer, area);
}

// ── Common key sets for reuse across screens ──────────────────────

/// Standard navigation: arrows + enter + esc + quit.
pub const KEYS_LIST: &[(&str, &str)] = &[
    ("↑↓", "Navigate"),
    ("Enter", "Select"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Checkbox selection (vertical): navigate + enter to toggle/confirm + select all + back + quit.
pub const KEYS_CHECKBOX: &[(&str, &str)] = &[
    ("↑↓", "Navigate"),
    ("Enter", "Select / Confirm"),
    ("a", "All"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Checkbox selection (grid): arrows for 2D navigation + enter to toggle/confirm + select all + back + quit.
pub const KEYS_CHECKBOX_GRID: &[(&str, &str)] = &[
    ("↑↓←→", "Navigate"),
    ("Enter", "Select / Confirm"),
    ("a", "All"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// INCAR param editing (vertical): navigate + edit/confirm + delete + add + back + quit.
pub const KEYS_INCAR: &[(&str, &str)] = &[
    ("↑↓", "Navigate"),
    ("Enter", "Edit / Confirm"),
    ("d", "Delete"),
    ("+", "Add"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// INCAR param editing (grid): arrows + edit/confirm + delete + add + back + quit.
pub const KEYS_INCAR_GRID: &[(&str, &str)] = &[
    ("↑↓←→", "Navigate"),
    ("Enter", "Edit / Confirm"),
    ("d", "Delete"),
    ("+", "Add"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Text input: enter to confirm + esc to go back + quit.
#[allow(dead_code)]
pub const KEYS_INPUT: &[(&str, &str)] = &[
    ("Bksp", "Delete"),
    ("Enter", "Confirm"),
    ("Esc", "Back"),
];

/// Confirmation: yes/no + back + quit.
pub const KEYS_CONFIRM: &[(&str, &str)] = &[
    ("Y", "Yes"),
    ("N", "No"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Form navigation: arrows + enter to edit + back + quit.
pub const KEYS_FORM: &[(&str, &str)] = &[
    ("↑↓", "Navigate"),
    ("Enter", "Edit / Confirm"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Form navigation for quick submit: arrows + enter to edit/submit + back + quit.
pub const KEYS_FORM_SUBMIT: &[(&str, &str)] = &[
    ("↑↓", "Navigate"),
    ("Enter", "Edit / Submit"),
    ("Esc", "Back"),
    ("q", "Quit"),
];

/// Form field editing: type to edit + esc to stop.
pub const KEYS_FORM_EDITING: &[(&str, &str)] = &[
    ("Bksp", "Delete"),
    ("Esc", "Done editing"),
];

/// Final screen: just exit.
pub const KEYS_EXIT: &[(&str, &str)] = &[("Enter", "Exit")];

/// Welcome screen: confirm directory.
pub const KEYS_WELCOME: &[(&str, &str)] = &[
    ("Y", "Yes"),
    ("N", "No / Exit"),
];
