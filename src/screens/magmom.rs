//! MAGMOM entry screen: grouped by species.
//!
//! Displays species table and editable MAGMOM value per species.
//! Shows the resulting VASP MAGMOM string as a live preview.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction, TextBuffer};

pub struct MagmomScreen {
    /// One text buffer per species for MAGMOM input.
    buffers: Vec<TextBuffer>,
    /// Which species field or "Confirm" slot is focused.
    focus: usize,
    /// Whether the focused field is in edit mode.
    editing: bool,
    /// Validation error message, if any.
    error: Option<String>,
}

impl MagmomScreen {
    pub fn new(species_count: usize) -> Self {
        let buffers = (0..species_count)
            .map(|_| TextBuffer::with_content("0.0"))
            .collect();
        Self {
            buffers,
            focus: 0,
            editing: false,
            error: None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::Magmom.label());
        let footer_keys = if self.editing { footer::KEYS_FORM_EDITING } else { footer::KEYS_FORM };
        footer::render(frame, footer_area, footer_keys);

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]),
            Line::raw(""),
            Line::raw("  Set MAGMOM values (per species):"),
            Line::from(Span::styled(
                "  Press Enter on a field to edit. Use ↑↓ to navigate.",
                STYLE_MUTED,
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Species    Count    MAGMOM (per atom)", STYLE_MUTED),
            ]),
            Line::from(Span::styled(
                "  ─────────────────────────────────────",
                STYLE_MUTED,
            )),
        ];

        for (i, sp) in state.species.iter().enumerate() {
            let is_focused = i == self.focus;
            let prefix = if is_focused { "> " } else { "  " };
            let is_editing = is_focused && self.editing;
            let buf_content = if i < self.buffers.len() {
                &self.buffers[i].content
            } else {
                "0.0"
            };

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
                    Span::styled(
                        format!("  {}{:<9} {:<8} ", prefix, sp.symbol, sp.count),
                        style,
                    ),
                ];
                spans.extend(self.buffers[i].cursor_spans(STYLE_HIGHLIGHT));
                spans.push(Span::styled(hint, STYLE_MUTED));
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}{:<9} {:<8} ", prefix, sp.symbol, sp.count),
                        style,
                    ),
                    Span::styled(buf_content.to_string(), STYLE_HIGHLIGHT),
                    Span::styled(hint, STYLE_MUTED),
                ]));
            }
        }

        // Confirm slot
        let confirm_focused = self.focus == self.buffers.len();
        let confirm_prefix = if confirm_focused { "> " } else { "  " };
        let confirm_style = if confirm_focused { STYLE_HIGHLIGHT } else { STYLE_MUTED };
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}Confirm", confirm_prefix),
            confirm_style,
        )));

        // Show live MAGMOM preview
        lines.push(Line::raw(""));
        if let Some(preview) = self.preview_magmom(state) {
            lines.push(Line::from(vec![
                Span::styled("  MAGMOM string: ", STYLE_MUTED),
                Span::styled(preview, STYLE_HIGHLIGHT),
            ]));
        }

        // Show error if any
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
                    if self.focus < self.buffers.len() {
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
                    if self.focus < self.buffers.len() {
                        self.buffers[self.focus].handle_key(other);
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
            }
        } else {
            match code {
                KeyCode::Tab | KeyCode::Down => {
                    if self.focus < self.buffers.len() {
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
                    if self.focus < self.buffers.len() {
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
        if self.buffers.is_empty() {
            self.error = Some("No species found — cannot set MAGMOM".to_string());
            return ScreenAction::Continue;
        }
        let mut values = Vec::new();
        for (i, buf) in self.buffers.iter().enumerate() {
            match buf.content.trim().parse::<f64>() {
                Ok(v) => values.push(v),
                Err(_) => {
                    self.error = Some(format!(
                        "Invalid value for {}: '{}'",
                        state.species.get(i).map_or("?", |s| &s.symbol),
                        buf.content
                    ));
                    self.focus = i;
                    return ScreenAction::Continue;
                }
            }
        }
        state.magmom_per_species = values;
        ScreenAction::Advance
    }

    fn preview_magmom(&self, state: &AppState) -> Option<String> {
        if state.species.is_empty() {
            return None;
        }
        let parts: Vec<String> = state
            .species
            .iter()
            .zip(self.buffers.iter())
            .map(|(sp, buf)| {
                if let Ok(v) = buf.content.trim().parse::<f64>() {
                    format!("{}*{}", sp.count, v)
                } else {
                    format!("{}*???", sp.count)
                }
            })
            .collect();
        Some(parts.join(" "))
    }
}
