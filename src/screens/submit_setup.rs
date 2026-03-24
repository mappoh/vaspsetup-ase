//! Submission setup screen: output directory, naming, queue, cores.
//!
//! Quick Submit: no output directory (uses work_dir), includes VASP binary field,
//! confirms and submits directly (no Confirm screen).
//! Normal flow: includes output directory, auto-selects VASP binary,
//! advances to Confirm screen.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::{AppState, FlowPath};
use crate::widgets::{self, file_list, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction, TextBuffer};

pub struct SubmitSetupScreen {
    buffers: Vec<TextBuffer>,
    labels: Vec<&'static str>,
    /// Whether this is the Quick Submit flow.
    is_quick: bool,
    /// Named indices into `buffers` for safe access.
    idx_output_dir: Option<usize>,
    idx_calc_name: usize,
    idx_queue: usize,
    idx_cores: usize,
    idx_vasp_binary: Option<usize>,
    /// Cached work_dir string (avoids to_string_lossy() every frame).
    work_dir_display: String,
    /// Which field or "Confirm" slot is focused.
    focus: usize,
    /// Whether the focused field is in edit mode.
    editing: bool,
    error: Option<String>,
}

impl SubmitSetupScreen {
    pub fn new(state: &AppState) -> Self {
        let default_name = state
            .calc_type
            .as_ref()
            .map_or("calc".to_string(), |ct| ct.preset_name().to_string());

        let is_quick = state.flow_path == Some(FlowPath::QuickSubmit);

        let mut buffers = Vec::new();
        let mut labels = Vec::new();

        // Output directory — only in normal flow
        let idx_output_dir = if !is_quick {
            let idx = buffers.len();
            buffers.push(TextBuffer::with_content("./"));
            labels.push("Output directory");
            Some(idx)
        } else {
            None
        };

        let idx_calc_name = buffers.len();
        buffers.push(TextBuffer::with_content(&default_name));
        labels.push("Job name");

        let idx_queue = buffers.len();
        buffers.push(TextBuffer::with_content(&state.queue));
        labels.push("Queue");

        let idx_cores = buffers.len();
        buffers.push(TextBuffer::with_content(&state.cores.to_string()));
        labels.push("Cores");

        // VASP binary — only in Quick Submit
        let idx_vasp_binary = if is_quick {
            let idx = buffers.len();
            buffers.push(TextBuffer::with_content("vasp_std"));
            labels.push("VASP binary");
            Some(idx)
        } else {
            None
        };

        Self {
            buffers,
            labels,
            is_quick,
            idx_output_dir,
            idx_calc_name,
            idx_queue,
            idx_cores,
            idx_vasp_binary,
            work_dir_display: state.work_dir.to_string_lossy().to_string(),
            focus: 0,
            editing: false,
            error: None,
        }
    }

    /// Number of editable fields (excludes the Confirm/Submit slot).
    fn field_count(&self) -> usize {
        self.buffers.len()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::SubmitSetup.label());
        let footer_keys = if self.editing {
            footer::KEYS_FORM_EDITING
        } else if self.is_quick {
            footer::KEYS_FORM_SUBMIT
        } else {
            footer::KEYS_FORM
        };
        footer::render(frame, footer_area, footer_keys);

        let mut lines = vec![Line::raw("")];

        // Directory context block — Quick Submit only
        if self.is_quick {
            lines.push(Line::from(vec![
                Span::styled("  Directory: ", STYLE_MUTED),
                Span::styled(
                    self.work_dir_display.clone(),
                    STYLE_HIGHLIGHT,
                ),
            ]));
            let file_text = file_list::format_horizontal(&state.files, area.width as usize);
            for line in file_text.lines {
                lines.push(line);
            }
            lines.push(Line::raw(""));
        }

        lines.push(Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]));
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("  Submission setup:", widgets::STYLE_HEADER)));
        lines.push(Line::from(Span::styled(
            "  Press Enter on a field to edit. Use ↑↓ to navigate.",
            STYLE_MUTED,
        )));
        lines.push(Line::raw(""));

        for (i, label) in self.labels.iter().enumerate() {
            let is_focused = i == self.focus;
            let is_editing = is_focused && self.editing;
            let prefix = if is_focused { "> " } else { "  " };
            let style = if is_focused { STYLE_HIGHLIGHT } else { STYLE_MUTED };

            let hint = if is_focused {
                if self.editing {
                    "  press Esc to exit editing"
                } else {
                    "  press Enter to edit"
                }
            } else {
                ""
            };

            if is_editing {
                let mut spans = vec![
                    Span::styled(format!("  {}{:<20}", prefix, format!("{}:", label)), style),
                ];
                spans.extend(self.buffers[i].cursor_spans(STYLE_HIGHLIGHT));
                spans.push(Span::styled(hint, STYLE_MUTED));
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}{:<20}", prefix, format!("{}:", label)), style),
                    Span::styled(
                        self.buffers[i].content.clone(),
                        STYLE_HIGHLIGHT,
                    ),
                    Span::styled(hint, STYLE_MUTED),
                ]));
            }
        }

        // VASP binary (auto-selected, not editable) — only in normal flow
        if !self.is_quick {
            let vasp_bin = state.vasp_binary();
            lines.push(Line::from(vec![
                Span::styled("    VASP binary:          ", STYLE_MUTED),
                Span::styled(format!("{}  (auto-selected)", vasp_bin), STYLE_MUTED),
            ]));
        }

        // Confirm/Submit slot
        let confirm_focused = self.focus == self.field_count();
        let confirm_prefix = if confirm_focused { "> " } else { "  " };
        let confirm_style = if confirm_focused { STYLE_HIGHLIGHT } else { STYLE_MUTED };
        let confirm_label = if self.is_quick {
            "Confirm and submit to cluster"
        } else {
            "Confirm"
        };
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}{}", confirm_prefix, confirm_label),
            confirm_style,
        )));

        // Job name preview
        let raw_name = &self.buffers[self.idx_calc_name].content;
        let sanitized = crate::shell::sanitize_job_name(raw_name);
        lines.push(Line::raw(""));
        if state.selected_files.len() > 1 {
            lines.push(Line::from(Span::styled("  Preview:", STYLE_MUTED)));
            for (i, &file_idx) in state.selected_files.iter().enumerate() {
                let file_name = state.files.get(file_idx).map_or("?", |s| s.as_str());
                lines.push(Line::from(Span::styled(
                    format!("    {}_{:02}  <- {}", sanitized, i + 1, file_name),
                    STYLE_MUTED,
                )));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("  Job name: ", STYLE_MUTED),
                Span::styled(sanitized, STYLE_HIGHLIGHT),
            ]));
        }

        if let Some(ref err) = self.error {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", err),
                widgets::STYLE_ERROR,
            )));
        }

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        let fc = self.field_count();
        if self.editing {
            match code {
                KeyCode::Esc => {
                    self.editing = false;
                    ScreenAction::Continue
                }
                KeyCode::Up => {
                    self.editing = false;
                    if self.focus > 0 {
                        self.focus -= 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::Down => {
                    self.editing = false;
                    if self.focus < fc {
                        self.focus += 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::Enter => {
                    self.editing = false;
                    ScreenAction::Continue
                }
                other => {
                    if self.focus < fc {
                        self.buffers[self.focus].handle_key(other);
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
            }
        } else {
            match code {
                KeyCode::Tab | KeyCode::Down => {
                    if self.focus < fc {
                        self.focus += 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::BackTab | KeyCode::Up => {
                    if self.focus > 0 {
                        self.focus -= 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::Enter => {
                    if self.focus < fc {
                        self.editing = true;
                        ScreenAction::Continue
                    } else {
                        self.try_confirm(state)
                    }
                }
                KeyCode::Esc => ScreenAction::Back,
                KeyCode::Char('q') => ScreenAction::Quit,
                _ => ScreenAction::Continue,
            }
        }
    }

    fn try_confirm(&mut self, state: &mut AppState) -> ScreenAction {
        // Validate output directory (normal flow only)
        if let Some(idx) = self.idx_output_dir {
            let output_dir = self.buffers[idx].content.trim().to_string();
            if output_dir.is_empty() {
                self.error = Some("Output directory cannot be empty".to_string());
                self.focus = idx;
                return ScreenAction::Continue;
            }
            state.output_dir = output_dir;
        } else {
            // Quick Submit: use work_dir
            state.output_dir = state.work_dir.to_string_lossy().to_string();
        }

        // Validate calc name
        let calc_name = self.buffers[self.idx_calc_name].content.trim().to_string();
        if calc_name.is_empty() {
            self.error = Some("Job name cannot be empty".to_string());
            self.focus = self.idx_calc_name;
            return ScreenAction::Continue;
        }

        // Validate queue
        let queue = self.buffers[self.idx_queue].content.trim().to_string();
        if queue.is_empty() {
            self.error = Some("Queue cannot be empty".to_string());
            self.focus = self.idx_queue;
            return ScreenAction::Continue;
        }

        // Validate cores
        let cores_str = self.buffers[self.idx_cores].content.trim().to_string();
        let cores = match cores_str.parse::<u32>() {
            Ok(c) if c > 0 => c,
            _ => {
                self.error = Some("Cores must be a positive number".to_string());
                self.focus = self.idx_cores;
                return ScreenAction::Continue;
            }
        };

        // Validate VASP binary (Quick Submit only)
        if let Some(idx) = self.idx_vasp_binary {
            let vasp_bin = self.buffers[idx].content.trim().to_string();
            if vasp_bin.is_empty() {
                self.error = Some("VASP binary cannot be empty".to_string());
                self.focus = idx;
                return ScreenAction::Continue;
            }
            state.vasp_binary_override = Some(vasp_bin);
        }

        state.queue = queue;
        state.cores = cores;

        // Generate job names
        let sanitized = crate::shell::sanitize_job_name(&calc_name);
        if state.selected_files.len() > 1 {
            state.job_names = (0..state.selected_files.len())
                .map(|i| format!("{}_{:02}", sanitized, i + 1))
                .collect();
        } else {
            state.job_names = vec![sanitized];
        }

        ScreenAction::Advance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    fn quick_submit_state() -> AppState {
        let cfg = Config::default();
        let mut state = AppState::new(
            PathBuf::from("/home/user/vasp/calc"),
            vec!["POSCAR".into(), "INCAR".into(), "KPOINTS".into()],
            &cfg,
        );
        state.flow_path = Some(FlowPath::QuickSubmit);
        state
    }

    fn normal_state() -> AppState {
        let cfg = Config::default();
        let mut state = AppState::new(
            PathBuf::from("/home/user/vasp/calc"),
            vec!["POSCAR".into()],
            &cfg,
        );
        state.flow_path = Some(FlowPath::PerformCalculation);
        state
    }

    #[test]
    fn test_quick_submit_no_output_dir() {
        let state = quick_submit_state();
        let screen = SubmitSetupScreen::new(&state);

        assert!(screen.is_quick);
        assert!(screen.idx_output_dir.is_none());
        assert!(screen.idx_vasp_binary.is_some());
        assert_eq!(screen.labels.len(), 4); // job name, queue, cores, vasp binary
        assert_eq!(screen.labels[0], "Job name");
        assert_eq!(screen.labels[3], "VASP binary");
    }

    #[test]
    fn test_normal_flow_has_output_dir() {
        let state = normal_state();
        let screen = SubmitSetupScreen::new(&state);

        assert!(!screen.is_quick);
        assert!(screen.idx_output_dir.is_some());
        assert!(screen.idx_vasp_binary.is_none());
        assert_eq!(screen.labels.len(), 4); // output dir, job name, queue, cores
        assert_eq!(screen.labels[0], "Output directory");
    }
}
