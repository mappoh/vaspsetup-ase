//! Calculation type selection screen.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, CalcType};
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ListCursor, ScreenAction};

pub struct CalcTypeScreen {
    cursor: ListCursor,
}

impl CalcTypeScreen {
    pub fn new() -> Self {
        Self {
            cursor: ListCursor::new(CalcType::all().len()),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::CalcType.label());
        footer::render(frame, footer_area, footer::KEYS_LIST);

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]),
            Line::raw(""),
            Line::raw("  Select calculation type:"),
            Line::from(Span::styled(
                "  This determines the INCAR preset and default parameters.",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        for (i, calc_type) in CalcType::all().iter().enumerate() {
            let prefix = if i == self.cursor.index { "  > " } else { "    " };
            let style = if i == self.cursor.index {
                STYLE_HIGHLIGHT
            } else {
                STYLE_MUTED
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, calc_type.display_name()),
                style,
            )));
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
                state.calc_type = Some(CalcType::all()[self.cursor.index].clone());
                ScreenAction::Advance
            }
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}
