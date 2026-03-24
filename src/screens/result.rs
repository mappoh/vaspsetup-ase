//! Result screen: show submission success or failure, then exit.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_HIGHLIGHT, STYLE_MUTED, STYLE_SUCCESS, STYLE_ERROR};

use super::{ActiveScreen, ScreenAction};

/// Submission result for display.
#[derive(Debug)]
pub struct JobResult {
    pub job_name: String,
    pub success: bool,
    pub job_id: Option<String>,
    pub message: String,
}

impl JobResult {
    pub fn error(job_name: String, message: String) -> Self {
        Self {
            job_name,
            success: false,
            job_id: None,
            message,
        }
    }
}

pub struct ResultScreen {
    pub results: Vec<JobResult>,
}

impl ResultScreen {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, 0, 0, ActiveScreen::Result.label());
        footer::render(frame, footer_area, footer::KEYS_EXIT);

        let mut lines = vec![Line::raw("")];

        let success_count = self.results.iter().filter(|r| r.success).count();
        let total = self.results.len();

        if success_count == total && total > 0 {
            lines.push(Line::from(Span::styled(
                format!("  {} job(s) submitted successfully", total),
                STYLE_SUCCESS,
            )));
        } else if success_count == 0 && total > 0 {
            lines.push(Line::from(Span::styled(
                "  Submission failed",
                STYLE_ERROR,
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {}/{} job(s) submitted", success_count, total),
                STYLE_HIGHLIGHT,
            )));
        }

        lines.push(Line::raw(""));

        for result in &self.results {
            if result.success {
                let id_str = result
                    .job_id
                    .as_deref()
                    .unwrap_or("unknown");
                lines.push(Line::from(vec![
                    Span::styled("  + ", STYLE_SUCCESS),
                    Span::styled(&result.job_name, STYLE_HIGHLIGHT),
                    Span::styled(format!("  (Job ID: {})", id_str), STYLE_MUTED),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  x ", STYLE_ERROR),
                    Span::styled(&result.job_name, STYLE_HIGHLIGHT),
                    Span::styled(format!("  {}", result.message), STYLE_ERROR),
                ]));
            }
        }

        lines.push(Line::raw(""));
        lines.push(Line::raw("  Press Enter to exit."));

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, _state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}
