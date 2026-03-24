//! File selection screen: checkbox list of detected structure files.
//!
//! Mirrors the SubmitSetup pattern: Enter toggles checkboxes on file items,
//! and a Confirm button at the bottom advances to the next screen.
//!
//! When >5 files, displays as a multi-column grid (column-major, like `ls`).
//! When ≤5 files, displays as a simple vertical list.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_ACTION, STYLE_ERROR, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, CheckboxList, ScreenAction};

/// Threshold: use grid layout when file count exceeds this.
const GRID_THRESHOLD: usize = 5;

/// Maximum usable width for content (matches app viewport cap).
const MAX_WIDTH: usize = 80;

/// Grid layout parameters computed from the file list.
#[derive(Clone)]
struct GridLayout {
    cols: usize,
    rows: usize,
    col_width: usize,
}

impl GridLayout {
    /// Compute grid dimensions for column-major ordering.
    ///
    /// Each cell needs: prefix(4) + "[x] "(4) + filename + gap(2).
    /// We find the widest filename, then fit as many columns as possible
    /// within MAX_WIDTH.
    fn compute(files: &[String]) -> Self {
        let longest = files.iter().map(|f| f.len()).max().unwrap_or(0);
        // prefix "  > " or "    " = 4, checkbox "[x] " = 4, gap between cols = 2
        let cell_width = 4 + 4 + longest;
        let col_width = cell_width + 2; // include gap

        // How many columns fit? At least 1.
        let cols = ((MAX_WIDTH - 2) / col_width).max(1); // subtract 2 for outer indent
        let rows = (files.len() + cols - 1) / cols; // ceiling division

        Self { cols, rows, col_width }
    }

    /// Map (row, col) to file index using column-major ordering.
    /// Returns None if the position is past the end of the file list.
    fn index(&self, row: usize, col: usize, file_count: usize) -> Option<usize> {
        let idx = col * self.rows + row;
        if idx < file_count { Some(idx) } else { None }
    }

    /// Map a file index back to (row, col).
    #[cfg(test)]
    fn position(&self, index: usize) -> (usize, usize) {
        let col = index / self.rows;
        let row = index % self.rows;
        (row, col)
    }
}

pub struct FilePickScreen {
    checkboxes: CheckboxList,
    /// Which item is focused: 0..file_count = file items, file_count = Confirm button.
    focus: usize,
    /// Number of file items (excludes the Confirm button).
    file_count: usize,
    /// Whether grid mode is active (>5 files).
    use_grid: bool,
    /// Cached grid layout (computed once at init, never changes).
    cached_layout: Option<GridLayout>,
    /// Current grid column when in grid mode.
    grid_col: usize,
    /// Current grid row when in grid mode.
    grid_row: usize,
    error: Option<String>,
}

impl FilePickScreen {
    pub fn new(file_count: usize, files: &[String]) -> Self {
        let use_grid = file_count > GRID_THRESHOLD;
        let cached_layout = if use_grid {
            Some(GridLayout::compute(files))
        } else {
            None
        };
        Self {
            checkboxes: CheckboxList::new(file_count),
            focus: 0,
            file_count,
            use_grid,
            cached_layout,
            grid_col: 0,
            grid_row: 0,
            error: None,
        }
    }

    /// Get the cached grid layout (panics if not in grid mode).
    fn layout(&self) -> &GridLayout {
        self.cached_layout.as_ref().expect("layout called outside grid mode")
    }

    /// Whether the Confirm button is focused.
    fn on_confirm(&self) -> bool {
        self.focus == self.file_count
    }

    /// Sync focus from grid position. Returns false if position is invalid.
    fn sync_focus_from_grid(&mut self, layout: &GridLayout) -> bool {
        if let Some(idx) = layout.index(self.grid_row, self.grid_col, self.file_count) {
            self.focus = idx;
            self.checkboxes.cursor = idx;
            true
        } else {
            false
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::FilePick.label());
        let footer_keys = if self.use_grid {
            footer::KEYS_CHECKBOX_GRID
        } else {
            footer::KEYS_CHECKBOX
        };
        footer::render(frame, footer_area, footer_keys);

        let mut lines = vec![
            Line::raw(""),
            Line::from(vec![Span::raw("  "), Span::styled(" Action Required ", STYLE_ACTION)]),
            Line::raw(""),
            Line::raw("  Select files to calculate:"),
            Line::from(Span::styled(
                "  Press Enter to select, ↑↓ to navigate.",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        if self.use_grid {
            self.render_grid(&mut lines, state);
        } else {
            self.render_vertical(&mut lines, state);
        }

        // Selected count (above Confirm button)
        let selected_count = self.checkboxes.selected_indices().len();
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {} file(s) selected", selected_count),
            STYLE_MUTED,
        )));

        // Confirm button
        let confirm_focused = self.on_confirm();
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

    /// Render files as a simple vertical list (≤5 files).
    fn render_vertical(&self, lines: &mut Vec<Line<'static>>, state: &AppState) {
        for (i, file) in state.files.iter().enumerate() {
            let is_cursor = i == self.focus && !self.on_confirm();
            let is_checked = i < self.checkboxes.checked.len() && self.checkboxes.checked[i];

            let checkbox = if is_checked { "[x]" } else { "[ ]" };
            let prefix = if is_cursor { "  > " } else { "    " };
            let style = if is_cursor { STYLE_HIGHLIGHT } else { STYLE_MUTED };

            lines.push(Line::from(Span::styled(
                format!("{}{} {}", prefix, checkbox, file),
                style,
            )));
        }
    }

    /// Render files as a multi-column grid (>5 files, column-major like `ls`).
    fn render_grid(&self, lines: &mut Vec<Line<'static>>, state: &AppState) {
        let layout = self.layout();

        for row in 0..layout.rows {
            let mut spans: Vec<Span<'static>> = Vec::new();

            for col in 0..layout.cols {
                if let Some(idx) = layout.index(row, col, self.file_count) {
                    let is_cursor = idx == self.focus && !self.on_confirm();
                    let is_checked = idx < self.checkboxes.checked.len() && self.checkboxes.checked[idx];

                    let checkbox = if is_checked { "[x]" } else { "[ ]" };
                    let prefix = if is_cursor { "  > " } else { "    " };
                    let style = if is_cursor { STYLE_HIGHLIGHT } else { STYLE_MUTED };

                    let file = &state.files[idx];
                    let cell = format!("{}{} {}", prefix, checkbox, file);
                    // Pad cell to column width for alignment
                    let padded = format!("{:<width$}", cell, width = layout.col_width);
                    spans.push(Span::styled(padded, style));
                }
            }

            lines.push(Line::from(spans));
        }
    }

    pub fn handle_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        if self.use_grid {
            self.handle_grid_input(code, state)
        } else {
            self.handle_vertical_input(code, state)
        }
    }

    /// Handle input for vertical list mode (≤5 files).
    fn handle_vertical_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Up => {
                if self.focus > 0 {
                    self.focus -= 1;
                    self.checkboxes.cursor = self.focus.min(self.file_count.saturating_sub(1));
                }
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Down => {
                if self.focus < self.file_count {
                    self.focus += 1;
                    self.checkboxes.cursor = self.focus.min(self.file_count.saturating_sub(1));
                }
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Enter => self.handle_enter(state),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.checkboxes.select_all();
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }

    /// Handle input for grid mode (>5 files).
    fn handle_grid_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        let layout = self.cached_layout.as_ref().expect("grid input without layout").clone();

        match code {
            KeyCode::Up => {
                self.error = None;
                if self.on_confirm() {
                    // Move from Confirm to last row of current column
                    // Use the column we were in before entering Confirm
                    self.grid_row = layout.rows - 1;
                    // Make sure this position has a file
                    while self.grid_row > 0
                        && layout.index(self.grid_row, self.grid_col, self.file_count).is_none()
                    {
                        self.grid_row -= 1;
                    }
                    self.sync_focus_from_grid(&layout);
                } else if self.grid_row > 0 {
                    self.grid_row -= 1;
                    self.sync_focus_from_grid(&layout);
                }
                ScreenAction::Continue
            }
            KeyCode::Down => {
                self.error = None;
                if self.on_confirm() {
                    // Already at bottom, clamp
                } else if self.grid_row + 1 < layout.rows {
                    // Try to move down within the grid
                    let new_row = self.grid_row + 1;
                    if layout.index(new_row, self.grid_col, self.file_count).is_some() {
                        self.grid_row = new_row;
                        self.sync_focus_from_grid(&layout);
                    } else {
                        // No file at this position, move to Confirm
                        self.focus = self.file_count;
                    }
                } else {
                    // Last row, move to Confirm
                    self.focus = self.file_count;
                }
                ScreenAction::Continue
            }
            KeyCode::Left => {
                self.error = None;
                if !self.on_confirm() && self.grid_col > 0 {
                    self.grid_col -= 1;
                    if !self.sync_focus_from_grid(&layout) {
                        self.grid_col += 1; // revert if invalid
                    }
                }
                ScreenAction::Continue
            }
            KeyCode::Right => {
                self.error = None;
                if !self.on_confirm() && self.grid_col + 1 < layout.cols {
                    let new_col = self.grid_col + 1;
                    if layout.index(self.grid_row, new_col, self.file_count).is_some() {
                        self.grid_col = new_col;
                        self.sync_focus_from_grid(&layout);
                    }
                    // else clamp — no file in that column at this row
                }
                ScreenAction::Continue
            }
            KeyCode::Enter => self.handle_enter(state),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.checkboxes.select_all();
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }

    /// Shared Enter handler for both modes.
    fn handle_enter(&mut self, state: &mut AppState) -> ScreenAction {
        self.error = None;
        if self.on_confirm() {
            if self.checkboxes.any_selected() {
                state.selected_files = self.checkboxes.selected_indices();
                ScreenAction::Advance
            } else {
                self.error = Some("Select at least one file".to_string());
                ScreenAction::Continue
            }
        } else {
            // Toggle the checkbox at the current file position
            self.checkboxes.cursor = self.focus;
            self.checkboxes.toggle();
            ScreenAction::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    fn test_state() -> AppState {
        let cfg = Config::default();
        AppState::new(
            PathBuf::from("/tmp/test"),
            vec!["POSCAR_Fe2O3".into(), "POSCAR_simple".into()],
            &cfg,
        )
    }

    fn grid_state() -> AppState {
        let cfg = Config::default();
        let files: Vec<String> = (1..=8).map(|i| format!("POSCAR_{:02}", i)).collect();
        AppState::new(PathBuf::from("/tmp/test"), files, &cfg)
    }

    // ── Vertical mode tests (≤5 files) ──────────────────────────

    #[test]
    fn test_enter_toggles_file() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        assert_eq!(screen.focus, 0);
        let action = screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(action, ScreenAction::Continue);
        assert!(screen.checkboxes.checked[0]);
    }

    #[test]
    fn test_enter_untoggles_file() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Enter, &mut state);
        assert!(screen.checkboxes.checked[0]);
        screen.handle_input(KeyCode::Enter, &mut state);
        assert!(!screen.checkboxes.checked[0]);
    }

    #[test]
    fn test_enter_on_confirm_advances() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Enter, &mut state); // toggle file 0
        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state);
        assert!(screen.on_confirm());

        let action = screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(action, ScreenAction::Advance);
        assert_eq!(state.selected_files, vec![0]);
    }

    #[test]
    fn test_enter_on_confirm_blocked_when_empty() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state);
        assert!(screen.on_confirm());

        let action = screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(action, ScreenAction::Continue);
        assert_eq!(screen.error.as_deref(), Some("Select at least one file"));
    }

    #[test]
    fn test_down_from_last_file_reaches_confirm() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state);
        assert_eq!(screen.focus, 2);
        assert!(screen.on_confirm());
    }

    #[test]
    fn test_up_from_confirm_reaches_last_file() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state);
        assert!(screen.on_confirm());

        screen.handle_input(KeyCode::Up, &mut state);
        assert_eq!(screen.focus, 1);
        assert!(!screen.on_confirm());
    }

    #[test]
    fn test_confirm_button_not_past_end() {
        let mut state = test_state();
        let mut screen = FilePickScreen::new(2, &state.files);

        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state);
        screen.handle_input(KeyCode::Down, &mut state); // clamp
        assert_eq!(screen.focus, 2);
    }

    // ── Grid mode tests (>5 files) ──────────────────────────────

    #[test]
    fn test_grid_mode_activates() {
        let state = grid_state();
        let screen = FilePickScreen::new(8, &state.files);
        assert!(screen.use_grid);

        let state = test_state();
        let screen = FilePickScreen::new(2, &state.files);
        assert!(!screen.use_grid);
    }

    #[test]
    fn test_grid_layout_column_major() {
        // 8 files, with filenames "POSCAR_01" (9 chars)
        // cell = 4(prefix) + 4(checkbox) + 9(name) = 17, col_width = 19
        // 80 cols → (80-2)/19 = 4 cols, but 8 files / 4 cols = 2 rows
        let files: Vec<String> = (1..=8).map(|i| format!("POSCAR_{:02}", i)).collect();
        let layout = GridLayout::compute(&files);

        assert!(layout.cols >= 2);
        // Column-major: file 0 at (0,0), file 1 at (1,0), etc.
        assert_eq!(layout.index(0, 0, 8), Some(0));
        assert_eq!(layout.index(1, 0, 8), Some(1));
        // First item in second column
        let second_col_start = layout.rows;
        assert_eq!(layout.index(0, 1, 8), Some(second_col_start));
    }

    #[test]
    fn test_grid_down_moves_within_column() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);

        // Start at (0,0) = file 0
        assert_eq!(screen.focus, 0);
        screen.handle_input(KeyCode::Down, &mut state);
        // Should be at (1,0) = file 1
        assert_eq!(screen.focus, 1);
        assert_eq!(screen.grid_row, 1);
        assert_eq!(screen.grid_col, 0);
    }

    #[test]
    fn test_grid_right_moves_to_next_column() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);
        let layout = GridLayout::compute(&state.files);

        // Start at (0,0) = file 0
        screen.handle_input(KeyCode::Right, &mut state);
        // Should be at (0,1) = file at second column start
        assert_eq!(screen.grid_row, 0);
        assert_eq!(screen.grid_col, 1);
        assert_eq!(screen.focus, layout.rows); // column-major: second col starts at rows
    }

    #[test]
    fn test_grid_left_clamps_at_first_column() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);

        // At (0,0), Left should do nothing
        screen.handle_input(KeyCode::Left, &mut state);
        assert_eq!(screen.grid_col, 0);
        assert_eq!(screen.focus, 0);
    }

    #[test]
    fn test_grid_right_clamps_at_last_column() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);
        let layout = GridLayout::compute(&state.files);

        // Move to last column
        for _ in 0..layout.cols {
            screen.handle_input(KeyCode::Right, &mut state);
        }
        let col_after = screen.grid_col;
        // One more Right should clamp
        screen.handle_input(KeyCode::Right, &mut state);
        assert_eq!(screen.grid_col, col_after);
    }

    #[test]
    fn test_grid_down_from_last_row_reaches_confirm() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);
        let layout = GridLayout::compute(&state.files);

        // Move to last row
        for _ in 0..layout.rows {
            screen.handle_input(KeyCode::Down, &mut state);
        }
        assert!(screen.on_confirm());
    }

    #[test]
    fn test_grid_up_from_confirm_returns_to_grid() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);
        let layout = GridLayout::compute(&state.files);

        // Move to Confirm
        for _ in 0..=layout.rows {
            screen.handle_input(KeyCode::Down, &mut state);
        }
        assert!(screen.on_confirm());

        // Up should return to grid
        screen.handle_input(KeyCode::Up, &mut state);
        assert!(!screen.on_confirm());
        assert!(screen.focus < screen.file_count);
    }

    #[test]
    fn test_grid_enter_toggles() {
        let mut state = grid_state();
        let mut screen = FilePickScreen::new(8, &state.files);

        // Move right to second column
        screen.handle_input(KeyCode::Right, &mut state);
        let idx = screen.focus;
        screen.handle_input(KeyCode::Enter, &mut state);
        assert!(screen.checkboxes.checked[idx]);
    }

    #[test]
    fn test_grid_position_roundtrip() {
        let files: Vec<String> = (1..=12).map(|i| format!("POSCAR_{:02}", i)).collect();
        let layout = GridLayout::compute(&files);

        for idx in 0..12 {
            let (row, col) = layout.position(idx);
            assert_eq!(layout.index(row, col, 12), Some(idx),
                "Roundtrip failed for index {}: got ({}, {})", idx, row, col);
        }
    }
}
