//! TUI screens — one module per step in the flow.
//!
//! Each screen implements render() and handle_input(). The app event loop
//! dispatches to the current screen based on the Screen enum.
//!
//! Flow:
//!   Welcome → ChoosePath ─┬─→ CalcType → SpinMode → [Magmom] → FilePick
//!                          │     → EditIncar → EditKpoints → SubmitSetup → Confirm → Result
//!                          └─→ SubmitSetup (quick) → Confirm → Result

pub mod calc_type;
pub mod choose_path;
pub mod confirm;
pub mod edit_incar;
pub mod edit_kpoints;
pub mod file_pick;
pub mod magmom;
pub mod result;
pub mod spin_mode;
pub mod submit_setup;
pub mod tst_method;
pub mod welcome;

use crossterm::event::KeyCode;

/// What the app should do after handling input on a screen.
#[derive(Debug, PartialEq)]
pub enum ScreenAction {
    /// Stay on the current screen.
    Continue,
    /// Advance to the next screen in the flow.
    Advance,
    /// Go back to the previous screen.
    Back,
    /// Quit the application.
    Quit,
}

/// Which screen is currently active.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveScreen {
    Welcome,
    ChoosePath,
    CalcType,
    TstMethod,
    SpinMode,
    Magmom,
    FilePick,
    EditIncar,
    EditKpoints,
    SubmitSetup,
    Confirm,
    Result,
}

impl ActiveScreen {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Welcome => "",
            Self::ChoosePath => "Choose Path",
            Self::CalcType => "Calculation Type",
            Self::TstMethod => "TST Method",
            Self::SpinMode => "Spin Configuration",
            Self::Magmom => "Magnetic Moments",
            Self::FilePick => "Select Files",
            Self::EditIncar => "INCAR Parameters",
            Self::EditKpoints => "KPOINTS",
            Self::SubmitSetup => "Job Setup",
            Self::Confirm => "Summary",
            Self::Result => "Complete",
        }
    }
}

// ── Reusable input helpers ──────────────────────────────────────

/// A cursor for navigating a list of items. Used by selection screens.
#[derive(Debug)]
pub struct ListCursor {
    pub index: usize,
    pub count: usize,
}

impl ListCursor {
    pub fn new(count: usize) -> Self {
        Self { index: 0, count }
    }

    pub fn up(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    pub fn down(&mut self) {
        if self.index < self.count.saturating_sub(1) {
            self.index += 1;
        }
    }
}

/// A text input buffer. Used by text entry screens.
#[derive(Debug)]
pub struct TextBuffer {
    pub content: String,
    pub cursor_pos: usize,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
        }
    }

    pub fn with_content(s: &str) -> Self {
        let len = s.len();
        Self {
            content: s.to_string(),
            cursor_pos: len,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.content.insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    // Move back to the previous char boundary
                    let prev = self.content[..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.content.remove(prev);
                    self.cursor_pos = prev;
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    // Move to previous char boundary
                    self.cursor_pos = self.content[..self.cursor_pos]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.content.len() {
                    // Move to next char boundary
                    self.cursor_pos = self.content[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(self.content.len());
                }
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Render the buffer content as spans with a visible block cursor at cursor_pos.
    ///
    /// Returns: [text_before, cursor_char, text_after] where cursor_char is
    /// rendered with reversed style (bg=text_color, fg=bg_color) so it's always visible.
    pub fn cursor_spans(&self, text_style: ratatui::style::Style) -> Vec<ratatui::text::Span<'static>> {
        use ratatui::style::Modifier;
        let cursor_style = text_style.add_modifier(Modifier::REVERSED);

        let before = &self.content[..self.cursor_pos];
        let after = &self.content[self.cursor_pos..];

        let mut spans = Vec::with_capacity(3);
        if !before.is_empty() {
            spans.push(ratatui::text::Span::styled(before.to_string(), text_style));
        }
        // The cursor character: show the char at cursor_pos, or a space if at end
        if let Some(c) = after.chars().next() {
            let char_len = c.len_utf8();
            spans.push(ratatui::text::Span::styled(c.to_string(), cursor_style));
            let rest = &self.content[self.cursor_pos + char_len..];
            if !rest.is_empty() {
                spans.push(ratatui::text::Span::styled(rest.to_string(), text_style));
            }
        } else {
            // Cursor at end — show a block space
            spans.push(ratatui::text::Span::styled(" ".to_string(), cursor_style));
        }
        spans
    }
}

/// Checkbox state for multi-select screens.
#[derive(Debug)]
pub struct CheckboxList {
    pub cursor: usize,
    pub checked: Vec<bool>,
}

impl CheckboxList {
    pub fn new(count: usize) -> Self {
        Self {
            cursor: 0,
            checked: vec![false; count],
        }
    }

    #[allow(dead_code)]
    pub fn up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    #[allow(dead_code)]
    pub fn down(&mut self) {
        if self.cursor < self.checked.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn toggle(&mut self) {
        if self.cursor < self.checked.len() {
            self.checked[self.cursor] = !self.checked[self.cursor];
        }
    }

    pub fn select_all(&mut self) {
        let all_checked = self.checked.iter().all(|&c| c);
        for c in &mut self.checked {
            *c = !all_checked;
        }
    }

    /// Indices of checked items.
    pub fn selected_indices(&self) -> Vec<usize> {
        self.checked
            .iter()
            .enumerate()
            .filter(|(_, &c)| c)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn any_selected(&self) -> bool {
        self.checked.iter().any(|&c| c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_cursor() {
        let mut cursor = ListCursor::new(3);
        assert_eq!(cursor.index, 0);
        cursor.down();
        assert_eq!(cursor.index, 1);
        cursor.down();
        assert_eq!(cursor.index, 2);
        cursor.down(); // should not go past end
        assert_eq!(cursor.index, 2);
        cursor.up();
        assert_eq!(cursor.index, 1);
    }

    #[test]
    fn test_text_buffer() {
        let mut buf = TextBuffer::new();
        buf.handle_key(KeyCode::Char('h'));
        buf.handle_key(KeyCode::Char('i'));
        assert_eq!(buf.content, "hi");
        buf.handle_key(KeyCode::Backspace);
        assert_eq!(buf.content, "h");
    }

    #[test]
    fn test_checkbox_list() {
        let mut cb = CheckboxList::new(3);
        assert!(!cb.any_selected());
        cb.toggle();
        assert!(cb.any_selected());
        assert_eq!(cb.selected_indices(), vec![0]);
        cb.select_all();
        assert_eq!(cb.selected_indices(), vec![0, 1, 2]);
        cb.select_all(); // toggle all off
        assert!(!cb.any_selected());
    }

    #[test]
    fn test_text_buffer_with_content() {
        let buf = TextBuffer::with_content("hello");
        assert_eq!(buf.content, "hello");
        assert_eq!(buf.cursor_pos, 5);
    }
}
