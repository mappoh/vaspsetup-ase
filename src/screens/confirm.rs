//! Confirmation screen: show summary of all settings before submission.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, FlowPath, SpinMode};
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction};

pub struct ConfirmScreen;

impl ConfirmScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, 0, 0, ActiveScreen::Confirm.label());
        footer::render(frame, footer_area, footer::KEYS_CONFIRM);

        let mut lines = vec![Line::raw("")];

        // Calculation box (only for PerformCalculation path)
        if state.flow_path == Some(FlowPath::PerformCalculation) {
            let calc_name = state
                .calc_type
                .as_ref()
                .map_or("Unknown", |ct| ct.display_name());
            let spin_label = state
                .spin_mode
                .as_ref()
                .map_or("Unknown", |sm| match sm {
                    SpinMode::Restricted => "Restricted",
                    SpinMode::Unrestricted => "Unrestricted",
                });

            lines.push(Line::from(Span::styled(
                "  ── Calculation ──────────────────────────────────────",
                STYLE_MUTED,
            )));
            lines.push(summary_row("Type", calc_name));
            lines.push(summary_row("Spin", spin_label));

            // Files
            let file_names: Vec<&str> = state
                .selected_files
                .iter()
                .filter_map(|&i| state.files.get(i).map(|s| s.as_str()))
                .collect();
            lines.push(summary_row("Files", &file_names.join(", ")));

            // KPOINTS
            let [k1, k2, k3] = state.kpoints;
            lines.push(summary_row(
                "KPOINTS",
                &format!("{} x {} x {}  (Gamma)", k1, k2, k3),
            ));

            // MAGMOM if set
            if let Some(magmom) = state.magmom_string() {
                lines.push(summary_row("MAGMOM", &magmom));
            }

            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────────────────────────────",
                STYLE_MUTED,
            )));
        }

        // Submission box
        lines.push(Line::from(Span::styled(
            "  ── Submission ───────────────────────────────────────",
            STYLE_MUTED,
        )));
        lines.push(summary_row("Directory", &state.output_dir));
        if state.job_names.len() == 1 {
            lines.push(summary_row("Job name", &state.job_names[0]));
        } else {
            lines.push(summary_row(
                "Jobs",
                &format!("{} jobs ({}...)", state.job_names.len(), state.job_names[0]),
            ));
        }
        lines.push(summary_row("Queue", &state.queue));
        lines.push(summary_row("Cores", &state.cores.to_string()));
        lines.push(summary_row("Binary", state.vasp_binary()));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────────────────",
            STYLE_MUTED,
        )));

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]));
        lines.push(Line::raw(""));
        lines.push(Line::raw("  Submit to cluster?"));

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, _state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => ScreenAction::Advance,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }
}

fn summary_row<'a>(label: &str, value: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("    {:<10} ", label), STYLE_MUTED),
        Span::styled(value.to_string(), STYLE_HIGHLIGHT),
    ])
}
