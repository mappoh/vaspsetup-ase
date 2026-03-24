//! Choose path screen: Quick Submit vs Perform Calculation.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, FlowPath};
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ListCursor, ScreenAction};

const OPTIONS: &[(&str, &str)] = &[
    ("Quick Job Submission", "Submit an existing calculation to the cluster"),
    (
        "Perform Calculation",
        "Set up a new VASP calculation and submit",
    ),
];

pub struct ChoosePathScreen {
    cursor: ListCursor,
}

impl ChoosePathScreen {
    pub fn new() -> Self {
        Self {
            cursor: ListCursor::new(OPTIONS.len()),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::ChoosePath.label());
        footer::render(frame, footer_area, footer::KEYS_LIST);

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]),
            Line::raw(""),
            Line::raw("  What would you like to do?"),
            Line::from(Span::styled(
                "  Quick Job submission",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        for (i, (label, desc)) in OPTIONS.iter().enumerate() {
            let prefix = if i == self.cursor.index { "  > " } else { "    " };
            let style = if i == self.cursor.index {
                STYLE_HIGHLIGHT
            } else {
                STYLE_MUTED
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, label),
                style,
            )));
            lines.push(Line::from(Span::styled(
                format!("    {}", desc),
                STYLE_MUTED,
            )));
            lines.push(Line::raw(""));
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
                state.flow_path = Some(match self.cursor.index {
                    0 => FlowPath::QuickSubmit,
                    _ => FlowPath::PerformCalculation,
                });
                ScreenAction::Advance
            }
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}
