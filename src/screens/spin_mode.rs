//! Spin mode selection: Restricted vs Unrestricted.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, SpinMode};
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ListCursor, ScreenAction};

const OPTIONS: &[&str] = &["Spin Restricted", "Spin Unrestricted"];

pub struct SpinModeScreen {
    cursor: ListCursor,
}

impl SpinModeScreen {
    pub fn new() -> Self {
        Self {
            cursor: ListCursor::new(OPTIONS.len()),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::SpinMode.label());
        footer::render(frame, footer_area, footer::KEYS_LIST);

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]),
            Line::raw(""),
            Line::raw("  Spin polarization:"),
            Line::from(Span::styled(
                "  Unrestricted enables ISPIN=2 and requires MAGMOM values.",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        for (i, label) in OPTIONS.iter().enumerate() {
            let prefix = if i == self.cursor.index { "  > " } else { "    " };
            let style = if i == self.cursor.index {
                STYLE_HIGHLIGHT
            } else {
                STYLE_MUTED
            };
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), style)));
        }

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Up => {
                self.cursor.up();
                ScreenAction::Continue
            }
            KeyCode::Down => {
                self.cursor.down();
                ScreenAction::Continue
            }
            KeyCode::Enter => {
                state.spin_mode = Some(match self.cursor.index {
                    0 => SpinMode::Restricted,
                    _ => SpinMode::Unrestricted,
                });
                ScreenAction::Advance
            }
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}
