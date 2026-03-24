//! Header widget: left-aligned title + version, description, and optional teal label badge.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{STYLE_ACTION, STYLE_HEADER, STYLE_MUTED};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render the header with an optional step counter and descriptive label badge.
pub fn render(frame: &mut Frame, area: Rect, step: u32, total: u32, label: &str) {
    let right = if total > 0 && !label.is_empty() {
        format!("Step {}/{} · {}", step, total, label)
    } else if total > 0 {
        format!("Step {}/{}", step, total)
    } else if !label.is_empty() {
        label.to_string()
    } else {
        String::new()
    };
    render_with_label(frame, area, &right);
}

/// Render the header with a custom right-aligned label badge on the bottom separator.
fn render_with_label(frame: &mut Frame, area: Rect, right_label: &str) {
    let w = area.width as usize;

    // Line 1: gray separator
    let sep = Line::from(Span::styled("─".repeat(w), STYLE_MUTED));

    // Line 2: left-aligned title with version
    let title = format!("VASPsetup v{}", VERSION);
    let title_line = Line::from(Span::styled(title, STYLE_HEADER));

    // Line 3: left-aligned description
    let desc = "VASP calculation setup & submission";
    let desc_line = Line::from(Span::styled(desc, STYLE_MUTED));

    // Line 4: bottom separator — plain or with teal badge
    let bottom = if right_label.is_empty() {
        sep.clone()
    } else {
        let badge_text = format!(" {} ", right_label);
        let badge_len = badge_text.len();
        let sep_len = w.saturating_sub(badge_len).saturating_sub(1); // 1 for trailing space
        Line::from(vec![
            Span::styled("─".repeat(sep_len), STYLE_MUTED),
            Span::styled(badge_text, STYLE_ACTION),
            Span::raw(" "),
        ])
    };

    let header = Paragraph::new(vec![sep, title_line, desc_line, bottom]);
    frame.render_widget(header, area);
}