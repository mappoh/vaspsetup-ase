//! Welcome screen: detect directory, show files, confirm.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction};

pub struct WelcomeScreen;

impl WelcomeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, 0, 0, ActiveScreen::Welcome.label());
        footer::render(frame, footer_area, footer::KEYS_WELCOME);

        let dir_display = state
            .work_dir
            .file_name()
            .map(|n| format!("./{}", n.to_string_lossy()))
            .unwrap_or_else(|| state.work_dir.to_string_lossy().to_string());

        let file_list = widgets::file_list::format_horizontal(
            &state.files,
            content_area.width as usize,
        );

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Directory:  "),
                Span::styled(dir_display.clone(), widgets::STYLE_HIGHLIGHT),
            ]),
            Line::from(Span::styled(
                "  This is your current directory",
                STYLE_MUTED,
            )),
            Line::raw(""),
            Line::raw(format!("  Detected files in {} directory:", dir_display)),
        ];

        for line in file_list.lines {
            lines.push(line);
        }

        lines.push(Line::raw(""));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]));
        lines.push(Line::raw(""));
        lines.push(Line::raw("  Is this the correct directory?"));
        lines.push(Line::from(Span::styled(
            "  Confirm this is where your structure files live.",
            STYLE_MUTED,
        )));

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, _state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => ScreenAction::Advance,
            KeyCode::Char('n') | KeyCode::Char('N') => ScreenAction::Quit,
            KeyCode::Esc => ScreenAction::Back, // app handles Back on first screen as Quit
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}
