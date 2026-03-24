//! KPOINTS editing screen: three editable k-mesh fields.
//!
//! Uses the same form pattern as EditIncar: navigate fields with ↑↓,
//! Enter to edit, detail line below border, Confirm button at bottom.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_ERROR, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction, TextBuffer};

const FIELD_LABELS: [&str; 3] = ["K1", "K2", "K3"];
const MAX_WIDTH: usize = 80;

pub struct EditKpointsScreen {
    buffers: [TextBuffer; 3],
    /// 0..3 = fields, 3 = Confirm button.
    focus: usize,
    editing: bool,
    error: Option<String>,
}

impl EditKpointsScreen {
    pub fn new(state: &AppState) -> Self {
        let [k1, k2, k3] = state.kpoints;
        Self {
            buffers: [
                TextBuffer::with_content(&k1.to_string()),
                TextBuffer::with_content(&k2.to_string()),
                TextBuffer::with_content(&k3.to_string()),
            ],
            focus: 0,
            editing: false,
            error: None,
        }
    }

    fn on_confirm(&self) -> bool {
        self.focus == 3
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::EditKpoints.label());

        let footer_keys = if self.editing {
            footer::KEYS_FORM_EDITING
        } else {
            footer::KEYS_FORM
        };
        footer::render(frame, footer_area, footer_keys);

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled("  KPOINTS:", widgets::STYLE_HEADER)),
            Line::from(Span::styled(
                "  Navigate fields. Enter: edit, Esc: back.",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        // Top border
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(MAX_WIDTH - 4)),
            STYLE_MUTED,
        )));

        // K1, K2, K3 fields (inside bordered area, no inline hints)
        for (i, label) in FIELD_LABELS.iter().enumerate() {
            let is_focused = i == self.focus && !self.editing;
            let is_editing = i == self.focus && self.editing;
            let prefix = if is_focused || is_editing { "  > " } else { "    " };
            let style = if is_focused || is_editing { STYLE_HIGHLIGHT } else { STYLE_MUTED };

            lines.push(Line::from(Span::styled(
                format!("{}{:<8} = {}", prefix, label, self.buffers[i].content),
                style,
            )));
        }

        // Bottom border
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(MAX_WIDTH - 4)),
            STYLE_MUTED,
        )));

        // Detail line below border (matches INCAR pattern)
        if !self.on_confirm() {
            let i = self.focus;
            let label = FIELD_LABELS[i];
            if self.editing {
                let mut spans = vec![
                    Span::styled(format!("  > {:<8} = ", label), STYLE_HIGHLIGHT),
                ];
                spans.extend(self.buffers[i].cursor_spans(STYLE_HIGHLIGHT));
                spans.push(Span::styled("  press Esc to finish", STYLE_MUTED));
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  > {:<8} = {}", label, self.buffers[i].content),
                        STYLE_HIGHLIGHT,
                    ),
                    Span::styled("  press Enter to edit", STYLE_MUTED),
                ]));
            }
        } else {
            lines.push(Line::raw(""));
        }

        // Mesh preview
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("  Mesh: ", STYLE_MUTED),
            Span::styled(
                format!("{} x {} x {}  (Gamma)",
                    self.buffers[0].content,
                    self.buffers[1].content,
                    self.buffers[2].content,
                ),
                STYLE_HIGHLIGHT,
            ),
        ]));

        // Confirm button
        let confirm_focused = self.on_confirm() && !self.editing;
        let confirm_prefix = if confirm_focused { "> " } else { "  " };
        let confirm_style = if confirm_focused { STYLE_HIGHLIGHT } else { STYLE_MUTED };
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}{}", confirm_prefix, "Confirm and proceed"),
            confirm_style,
        )));

        if let Some(ref err) = self.error {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", err),
                STYLE_ERROR,
            )));
        }

        let content = Paragraph::new(lines);
        frame.render_widget(content, content_area);
    }

    pub fn handle_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        if self.editing {
            match code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.editing = false;
                    self.error = None;
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
                    if self.focus < 3 {
                        self.focus += 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                other => {
                    self.buffers[self.focus].handle_key(other);
                    self.error = None;
                    ScreenAction::Continue
                }
            }
        } else {
            match code {
                KeyCode::Up => {
                    if self.focus > 0 {
                        self.focus -= 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::Down | KeyCode::Tab => {
                    if self.focus < 3 {
                        self.focus += 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::BackTab => {
                    if self.focus > 0 {
                        self.focus -= 1;
                    }
                    self.error = None;
                    ScreenAction::Continue
                }
                KeyCode::Enter => {
                    if self.on_confirm() {
                        self.try_confirm(state)
                    } else {
                        self.editing = true;
                        ScreenAction::Continue
                    }
                }
                KeyCode::Esc => ScreenAction::Back,
                KeyCode::Char('q') => ScreenAction::Quit,
                _ => ScreenAction::Continue,
            }
        }
    }

    fn try_confirm(&mut self, state: &mut AppState) -> ScreenAction {
        let mut kpts = [0u32; 3];
        for (i, label) in FIELD_LABELS.iter().enumerate() {
            let val = self.buffers[i].content.trim();
            match val.parse::<u32>() {
                Ok(v) if v > 0 => kpts[i] = v,
                _ => {
                    self.error = Some(format!("{} must be a positive number", label));
                    self.focus = i;
                    return ScreenAction::Continue;
                }
            }
        }
        state.kpoints = kpts;
        ScreenAction::Advance
    }
}
