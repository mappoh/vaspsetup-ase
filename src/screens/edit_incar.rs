//! INCAR parameter editing screen.
//!
//! Shows all preset parameters as navigable, editable fields.
//! Uses grid layout (column-major like `ls`) when >10 params, vertical otherwise.
//! Enter = edit value, d = delete param, + = add new param inline.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::state::AppState;
use crate::widgets::{self, footer, header, STYLE_ERROR, STYLE_HIGHLIGHT, STYLE_MUTED};

use super::{ActiveScreen, ScreenAction, TextBuffer};

/// Threshold: use grid layout when param count exceeds this.
const GRID_THRESHOLD: usize = 10;

/// Maximum usable width for content.
const MAX_WIDTH: usize = 80;

/// Grid layout parameters for INCAR params.
#[derive(Clone)]
struct ParamGridLayout {
    cols: usize,
    rows: usize,
    col_width: usize,
}

impl ParamGridLayout {
    /// Compute grid dimensions for column-major ordering.
    fn compute(params: &[(String, String)]) -> Self {
        let longest = params
            .iter()
            .map(|(k, v)| k.len().max(8) + 3 + v.len()) // "KEY      = VALUE"
            .max()
            .unwrap_or(0);
        // prefix "  > " = 4, content, gap = 2
        let cell_width = 4 + longest + 2;
        let cols = ((MAX_WIDTH - 2) / cell_width).max(1);
        let rows = (params.len() + cols - 1) / cols;

        Self { cols, rows, col_width: cell_width }
    }

    /// Map (row, col) to param index using column-major ordering.
    fn index(&self, row: usize, col: usize, count: usize) -> Option<usize> {
        let idx = col * self.rows + row;
        if idx < count { Some(idx) } else { None }
    }
}

/// What mode the screen is in.
#[derive(Debug, PartialEq)]
enum Mode {
    /// Navigating params and Confirm button.
    Navigate,
    /// Editing a parameter value inline.
    EditValue,
    /// Editing the key of a new parameter (first phase of add).
    AddKey,
    /// Editing the value of a new parameter (second phase of add).
    AddValue,
}

pub struct EditIncarScreen {
    /// Sorted (key, display_value) pairs. Source of truth for the screen.
    params: Vec<(String, String)>,
    /// Which item is focused: 0..param_count = param, param_count = Confirm.
    focus: usize,
    /// Current mode.
    mode: Mode,
    /// Text buffer for editing.
    input: TextBuffer,
    /// Stores the key while editing the value during add.
    add_key: String,
    /// Whether grid mode is active.
    use_grid: bool,
    /// Cached grid layout (recomputed after add/delete).
    cached_layout: Option<ParamGridLayout>,
    /// Grid position tracking.
    grid_row: usize,
    grid_col: usize,
    error: Option<String>,
}

impl EditIncarScreen {
    pub fn new(state: &AppState) -> Self {
        let params = Self::params_from_state(state);
        let use_grid = params.len() > GRID_THRESHOLD;
        let cached_layout = if use_grid {
            Some(ParamGridLayout::compute(&params))
        } else {
            None
        };
        Self {
            focus: 0,
            use_grid,
            cached_layout,
            params,
            mode: Mode::Navigate,
            input: TextBuffer::new(),
            add_key: String::new(),
            grid_row: 0,
            grid_col: 0,
            error: state.error.clone(),
        }
    }

    /// Extract sorted (key, display_value) pairs from state.
    fn params_from_state(state: &AppState) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = state
            .incar_params
            .iter()
            .map(|(k, v)| (k.clone(), format_incar_value(v)))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// Recompute grid layout after params change (add/delete).
    fn recompute_layout(&mut self) {
        self.use_grid = self.params.len() > GRID_THRESHOLD;
        self.cached_layout = if self.use_grid {
            Some(ParamGridLayout::compute(&self.params))
        } else {
            None
        };
    }

    /// Get cached layout reference.
    fn layout(&self) -> &ParamGridLayout {
        self.cached_layout.as_ref().expect("layout called outside grid mode")
    }

    fn param_count(&self) -> usize {
        self.params.len()
    }

    fn on_confirm(&self) -> bool {
        self.focus == self.param_count()
    }

    /// Sync focus from grid position. Returns false if position is invalid.
    fn sync_focus_from_grid(&mut self, layout: &ParamGridLayout) -> bool {
        if let Some(idx) = layout.index(self.grid_row, self.grid_col, self.param_count()) {
            self.focus = idx;
            true
        } else {
            false
        }
    }

    /// Write current params back to state.
    fn write_to_state(&self, state: &mut AppState) {
        state.incar_params.clear();
        for (key, val_str) in &self.params {
            state.incar_params.insert(key.clone(), parse_incar_value(val_str));
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let [header_area, content_area, footer_area] = widgets::screen_layout(area);

        let calc_name = state
            .calc_type
            .as_ref()
            .map_or("Unknown", |ct| ct.display_name());
        let spin_label = state
            .spin_mode
            .as_ref()
            .map_or("", |sm| match sm {
                crate::state::SpinMode::Restricted => "Spin Restricted",
                crate::state::SpinMode::Unrestricted => "Spin Unrestricted",
            });

        header::render(frame, header_area, state.current_step, state.total_steps(), ActiveScreen::EditIncar.label());

        let footer_keys = match self.mode {
            Mode::EditValue | Mode::AddKey | Mode::AddValue => footer::KEYS_FORM_EDITING,
            Mode::Navigate if self.use_grid => footer::KEYS_INCAR_GRID,
            Mode::Navigate => footer::KEYS_INCAR,
        };
        footer::render(frame, footer_area, footer_keys);

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("  INCAR parameters ({}, {}):", calc_name, spin_label),
                widgets::STYLE_HEADER,
            )),
            Line::from(Span::styled(
                "  Navigate parameters. Enter: edit, d: delete, +: add new.",
                STYLE_MUTED,
            )),
            Line::raw(""),
        ];

        // Top border
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(MAX_WIDTH - 4)),
            STYLE_MUTED,
        )));

        if self.use_grid {
            self.render_grid(&mut lines);
        } else {
            self.render_vertical(&mut lines);
        }

        // Bottom border
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(MAX_WIDTH - 4)),
            STYLE_MUTED,
        )));

        // Focused param detail line (below border, always visible)
        if !self.on_confirm() && self.param_count() > 0 {
            let idx = self.focus.min(self.param_count().saturating_sub(1));
            let (key, val) = &self.params[idx];
            match self.mode {
                Mode::EditValue => {
                    let mut spans = vec![
                        Span::styled(format!("  > {:<8} = ", key), STYLE_HIGHLIGHT),
                    ];
                    spans.extend(self.input.cursor_spans(STYLE_HIGHLIGHT));
                    spans.push(Span::styled("  press Esc to finish", STYLE_MUTED));
                    lines.push(Line::from(spans));
                }
                Mode::Navigate => {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  > {:<8} = {}", key, val), STYLE_HIGHLIGHT),
                        Span::styled("  press Enter to edit", STYLE_MUTED),
                    ]));
                }
                _ => {
                    lines.push(Line::raw(""));
                }
            }
        } else {
            lines.push(Line::raw(""));
        }

        // Param count
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {} parameter(s)", self.param_count()),
            STYLE_MUTED,
        )));

        // Confirm button
        let confirm_focused = self.on_confirm() && self.mode == Mode::Navigate;
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

    /// Render params as a vertical list (≤10 params).
    fn render_vertical(&self, lines: &mut Vec<Line<'static>>) {
        for (i, (key, val)) in self.params.iter().enumerate() {
            self.render_param_line(lines, i, key, val, false);
        }
        // Render the new row if we're adding
        if self.mode == Mode::AddKey || self.mode == Mode::AddValue {
            self.render_add_row(lines);
        }
    }

    /// Render params as a multi-column grid (>10 params, column-major).
    fn render_grid(&self, lines: &mut Vec<Line<'static>>) {
        let layout = self.layout();

        for row in 0..layout.rows {
            let mut spans: Vec<Span<'static>> = Vec::new();

            for col in 0..layout.cols {
                if let Some(idx) = layout.index(row, col, self.param_count()) {
                    let is_focused = idx == self.focus && self.mode == Mode::Navigate;
                    let is_editing = idx == self.focus && self.mode == Mode::EditValue;

                    let prefix = if is_focused || is_editing { "  > " } else { "    " };
                    let style = if is_focused || is_editing { STYLE_HIGHLIGHT } else { STYLE_MUTED };

                    let (key, val) = &self.params[idx];
                    let cell = format!("{}{:<8} = {}", prefix, key, val);
                    let padded = format!("{:<width$}", cell, width = layout.col_width);
                    spans.push(Span::styled(padded, style));
                }
            }

            lines.push(Line::from(spans));
        }

        // Render the new row if we're adding (always at bottom, full width)
        if self.mode == Mode::AddKey || self.mode == Mode::AddValue {
            self.render_add_row(lines);
        }
    }

    /// Render a single param line (vertical mode).
    fn render_param_line(
        &self,
        lines: &mut Vec<Line<'static>>,
        idx: usize,
        key: &str,
        val: &str,
        _in_grid: bool,
    ) {
        let is_focused = idx == self.focus && self.mode == Mode::Navigate;
        let is_editing = idx == self.focus && self.mode == Mode::EditValue;

        let prefix = if is_focused || is_editing { "  > " } else { "    " };
        let style = if is_focused || is_editing { STYLE_HIGHLIGHT } else { STYLE_MUTED };

        lines.push(Line::from(Span::styled(
            format!("{}{:<8} = {}", prefix, key, val),
            style,
        )));
    }

    /// Render the inline add row at the bottom of the table.
    fn render_add_row(&self, lines: &mut Vec<Line<'static>>) {
        match self.mode {
            Mode::AddKey => {
                let mut spans = vec![
                    Span::styled("  > ", STYLE_HIGHLIGHT),
                ];
                spans.extend(self.input.cursor_spans(STYLE_HIGHLIGHT));
                spans.push(Span::styled("        = ", STYLE_MUTED));
                spans.push(Span::styled("  type key, Enter to set value", STYLE_MUTED));
                lines.push(Line::from(spans));
            }
            Mode::AddValue => {
                let mut spans = vec![
                    Span::styled(format!("  > {:<8} = ", self.add_key), STYLE_HIGHLIGHT),
                ];
                spans.extend(self.input.cursor_spans(STYLE_HIGHLIGHT));
                spans.push(Span::styled("  type value, Enter to save", STYLE_MUTED));
                lines.push(Line::from(spans));
            }
            _ => {}
        }
    }

    pub fn handle_input(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        match self.mode {
            Mode::Navigate => {
                if self.use_grid {
                    self.handle_grid_nav(code, state)
                } else {
                    self.handle_vertical_nav(code, state)
                }
            }
            Mode::EditValue => self.handle_edit(code),
            Mode::AddKey => self.handle_add_key(code),
            Mode::AddValue => self.handle_add_value(code),
        }
    }

    /// Handle vertical navigation (≤10 params).
    fn handle_vertical_nav(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        match code {
            KeyCode::Up => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Down => {
                if self.focus < self.param_count() {
                    self.focus += 1;
                }
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Enter => self.handle_enter(state),
            KeyCode::Char('d') | KeyCode::Char('D') => self.handle_delete(),
            KeyCode::Char('+') => self.start_add(),
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }

    /// Handle grid navigation (>10 params).
    fn handle_grid_nav(&mut self, code: KeyCode, state: &mut AppState) -> ScreenAction {
        // Copy layout values to avoid borrowing self immutably while mutating
        let layout = self.cached_layout.as_ref().expect("grid nav without layout").clone();

        match code {
            KeyCode::Up => {
                self.error = None;
                if self.on_confirm() {
                    self.grid_row = layout.rows - 1;
                    while self.grid_row > 0
                        && layout.index(self.grid_row, self.grid_col, self.param_count()).is_none()
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
                    // clamp
                } else if self.grid_row + 1 < layout.rows {
                    if layout.index(self.grid_row + 1, self.grid_col, self.param_count()).is_some() {
                        self.grid_row += 1;
                        self.sync_focus_from_grid(&layout);
                    } else {
                        self.focus = self.param_count();
                    }
                } else {
                    self.focus = self.param_count();
                }
                ScreenAction::Continue
            }
            KeyCode::Left => {
                self.error = None;
                if !self.on_confirm() && self.grid_col > 0 {
                    self.grid_col -= 1;
                    if !self.sync_focus_from_grid(&layout) {
                        self.grid_col += 1;
                    }
                }
                ScreenAction::Continue
            }
            KeyCode::Right => {
                self.error = None;
                if !self.on_confirm() && self.grid_col + 1 < layout.cols {
                    if layout.index(self.grid_row, self.grid_col + 1, self.param_count()).is_some() {
                        self.grid_col += 1;
                        self.sync_focus_from_grid(&layout);
                    }
                }
                ScreenAction::Continue
            }
            KeyCode::Enter => self.handle_enter(state),
            KeyCode::Char('d') | KeyCode::Char('D') => self.handle_delete(),
            KeyCode::Char('+') => self.start_add(),
            KeyCode::Esc => ScreenAction::Back,
            KeyCode::Char('q') => ScreenAction::Quit,
            _ => ScreenAction::Continue,
        }
    }

    /// Handle Enter: edit param value or confirm.
    fn handle_enter(&mut self, state: &mut AppState) -> ScreenAction {
        self.error = None;
        if self.on_confirm() {
            if self.params.is_empty() {
                self.error = Some("No parameters to submit".to_string());
                ScreenAction::Continue
            } else {
                self.write_to_state(state);
                ScreenAction::Advance
            }
        } else {
            // Start editing the focused param's value
            let (_, val) = &self.params[self.focus];
            self.input = TextBuffer::with_content(val);
            self.mode = Mode::EditValue;
            ScreenAction::Continue
        }
    }

    /// Handle delete: remove the focused param.
    fn handle_delete(&mut self) -> ScreenAction {
        if self.on_confirm() || self.params.is_empty() {
            return ScreenAction::Continue;
        }
        self.params.remove(self.focus);
        self.recompute_layout();
        if self.focus >= self.param_count() && self.focus > 0 {
            self.focus = self.param_count() - 1;
        }
        if self.use_grid && self.param_count() > 0 {
            let idx = self.focus.min(self.param_count().saturating_sub(1));
            self.grid_col = idx / self.layout().rows;
            self.grid_row = idx % self.layout().rows;
        }
        self.error = None;
        ScreenAction::Continue
    }

    /// Start adding: enter AddKey mode with empty input.
    fn start_add(&mut self) -> ScreenAction {
        self.input = TextBuffer::new();
        self.add_key = String::new();
        self.mode = Mode::AddKey;
        self.error = None;
        ScreenAction::Continue
    }

    /// Handle input while editing a param value.
    fn handle_edit(&mut self, code: KeyCode) -> ScreenAction {
        match code {
            KeyCode::Esc | KeyCode::Enter => {
                let new_val = self.input.content.trim().to_string();
                if !new_val.is_empty() {
                    self.params[self.focus].1 = new_val;
                }
                self.mode = Mode::Navigate;
                ScreenAction::Continue
            }
            other => {
                self.input.handle_key(other);
                self.error = None;
                ScreenAction::Continue
            }
        }
    }

    /// Handle input while typing the key for a new param.
    fn handle_add_key(&mut self, code: KeyCode) -> ScreenAction {
        match code {
            KeyCode::Esc => {
                // Cancel add
                self.mode = Mode::Navigate;
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Enter => {
                let key = self.input.content.trim().to_uppercase();
                if key.is_empty() {
                    // Empty key = cancel
                    self.mode = Mode::Navigate;
                    return ScreenAction::Continue;
                }
                // Check for duplicate
                if self.params.iter().any(|(k, _)| k == &key) {
                    self.error = Some(format!("{} already exists — edit it instead", key));
                    return ScreenAction::Continue;
                }
                // Move to value phase
                self.add_key = key;
                self.input = TextBuffer::new();
                self.mode = Mode::AddValue;
                self.error = None;
                ScreenAction::Continue
            }
            other => {
                self.input.handle_key(other);
                self.error = None;
                ScreenAction::Continue
            }
        }
    }

    /// Handle input while typing the value for a new param.
    fn handle_add_value(&mut self, code: KeyCode) -> ScreenAction {
        match code {
            KeyCode::Esc => {
                // Cancel add
                self.mode = Mode::Navigate;
                self.error = None;
                ScreenAction::Continue
            }
            KeyCode::Enter => {
                let val = self.input.content.trim().to_string();
                if val.is_empty() {
                    self.error = Some("Value cannot be empty".to_string());
                    return ScreenAction::Continue;
                }
                let key = self.add_key.clone();
                self.params.push((key.clone(), val));
                self.params.sort_by(|a, b| a.0.cmp(&b.0));
                self.recompute_layout();
                // Focus the newly added param and sync grid position
                if let Some(idx) = self.params.iter().position(|(k, _)| k == &key) {
                    self.focus = idx;
                    if self.use_grid && self.param_count() > 0 {
                        self.grid_col = idx / self.layout().rows;
                        self.grid_row = idx % self.layout().rows;
                    }
                }
                self.mode = Mode::Navigate;
                self.error = None;
                ScreenAction::Continue
            }
            other => {
                self.input.handle_key(other);
                self.error = None;
                ScreenAction::Continue
            }
        }
    }
}

/// Format a serde_json::Value for display.
fn format_incar_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => {
            if *b { ".TRUE.".to_string() } else { ".FALSE.".to_string() }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Parse an INCAR value string into a serde_json::Value.
fn parse_incar_value(s: &str) -> serde_json::Value {
    let upper = s.to_uppercase();
    if upper == ".TRUE." || upper == "TRUE" || upper == "T" {
        return serde_json::Value::Bool(true);
    }
    if upper == ".FALSE." || upper == "FALSE" || upper == "F" {
        return serde_json::Value::Bool(false);
    }
    if let Ok(i) = s.parse::<i64>() {
        return serde_json::json!(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return serde_json::json!(f);
    }
    serde_json::Value::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::CalcType;
    use std::path::PathBuf;

    fn test_state() -> AppState {
        let cfg = Config::default();
        let mut state = AppState::new(
            PathBuf::from("/tmp/test"),
            vec!["POSCAR".into()],
            &cfg,
        );
        state.calc_type = Some(CalcType::SinglePoint);
        state.incar_params.insert("ENCUT".into(), serde_json::json!(520));
        state.incar_params.insert("EDIFF".into(), serde_json::json!(1e-6));
        state.incar_params.insert("NSW".into(), serde_json::json!(0));
        state
    }

    fn large_state() -> AppState {
        let cfg = Config::default();
        let mut state = AppState::new(
            PathBuf::from("/tmp/test"),
            vec!["POSCAR".into()],
            &cfg,
        );
        state.calc_type = Some(CalcType::SinglePoint);
        for i in 0..12 {
            state.incar_params.insert(
                format!("PARAM_{:02}", i),
                serde_json::json!(i),
            );
        }
        state
    }

    // ── Basic tests ──────────────────────────────────────────────

    #[test]
    fn test_params_sorted() {
        let state = test_state();
        let screen = EditIncarScreen::new(&state);
        let keys: Vec<&str> = screen.params.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["EDIFF", "ENCUT", "NSW"]);
    }

    #[test]
    fn test_vertical_mode_for_small() {
        let state = test_state();
        let screen = EditIncarScreen::new(&state);
        assert!(!screen.use_grid);
    }

    #[test]
    fn test_grid_mode_for_large() {
        let state = large_state();
        let screen = EditIncarScreen::new(&state);
        assert!(screen.use_grid);
    }

    // ── Edit tests ──────────────────────────────────────────────

    #[test]
    fn test_enter_edits_param() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        assert_eq!(screen.focus, 0);
        screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(screen.mode, Mode::EditValue);
    }

    #[test]
    fn test_edit_saves_on_esc() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Enter, &mut state);
        screen.input = TextBuffer::with_content("1e-5");
        screen.handle_input(KeyCode::Esc, &mut state);

        assert_eq!(screen.mode, Mode::Navigate);
        assert_eq!(screen.params[0].1, "1e-5");
    }

    #[test]
    fn test_edit_saves_on_enter() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Enter, &mut state);
        screen.input = TextBuffer::with_content("600");
        screen.handle_input(KeyCode::Enter, &mut state);

        assert_eq!(screen.mode, Mode::Navigate);
        assert_eq!(screen.params[0].1, "600");
    }

    // ── Delete tests ─────────────────────────────────────────────

    #[test]
    fn test_delete_param() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        assert_eq!(screen.param_count(), 3);
        screen.handle_input(KeyCode::Char('d'), &mut state);
        assert_eq!(screen.param_count(), 2);
        assert_eq!(screen.params[0].0, "ENCUT");
    }

    #[test]
    fn test_delete_on_confirm_is_noop() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        for _ in 0..=screen.param_count() {
            screen.handle_input(KeyCode::Down, &mut state);
        }
        let count_before = screen.param_count();
        screen.handle_input(KeyCode::Char('d'), &mut state);
        assert_eq!(screen.param_count(), count_before);
    }

    // ── Inline add tests ─────────────────────────────────────────

    #[test]
    fn test_add_enters_key_mode() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Char('+'), &mut state);
        assert_eq!(screen.mode, Mode::AddKey);
    }

    #[test]
    fn test_add_key_then_value() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        // Start add
        screen.handle_input(KeyCode::Char('+'), &mut state);
        assert_eq!(screen.mode, Mode::AddKey);

        // Type key "ISIF"
        for c in "ISIF".chars() {
            screen.handle_input(KeyCode::Char(c), &mut state);
        }
        // Enter moves to value phase
        screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(screen.mode, Mode::AddValue);
        assert_eq!(screen.add_key, "ISIF");

        // Type value "3"
        screen.handle_input(KeyCode::Char('3'), &mut state);
        screen.handle_input(KeyCode::Enter, &mut state);

        assert_eq!(screen.mode, Mode::Navigate);
        assert!(screen.params.iter().any(|(k, v)| k == "ISIF" && v == "3"));
    }

    #[test]
    fn test_add_duplicate_rejected() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Char('+'), &mut state);
        for c in "ENCUT".chars() {
            screen.handle_input(KeyCode::Char(c), &mut state);
        }
        screen.handle_input(KeyCode::Enter, &mut state);

        // Still in AddKey mode because duplicate was rejected
        assert_eq!(screen.mode, Mode::AddKey);
        assert!(screen.error.is_some());
    }

    #[test]
    fn test_add_esc_cancels_key() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Char('+'), &mut state);
        for c in "NEW".chars() {
            screen.handle_input(KeyCode::Char(c), &mut state);
        }
        screen.handle_input(KeyCode::Esc, &mut state);

        assert_eq!(screen.mode, Mode::Navigate);
        assert_eq!(screen.param_count(), 3); // nothing added
    }

    #[test]
    fn test_add_esc_cancels_value() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Char('+'), &mut state);
        for c in "ISIF".chars() {
            screen.handle_input(KeyCode::Char(c), &mut state);
        }
        screen.handle_input(KeyCode::Enter, &mut state); // move to value
        assert_eq!(screen.mode, Mode::AddValue);

        screen.handle_input(KeyCode::Esc, &mut state);
        assert_eq!(screen.mode, Mode::Navigate);
        assert_eq!(screen.param_count(), 3); // nothing added
    }

    #[test]
    fn test_add_empty_key_cancels() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        screen.handle_input(KeyCode::Char('+'), &mut state);
        screen.handle_input(KeyCode::Enter, &mut state); // empty key

        assert_eq!(screen.mode, Mode::Navigate);
        assert_eq!(screen.param_count(), 3);
    }

    // ── Confirm tests ────────────────────────────────────────────

    #[test]
    fn test_confirm_writes_to_state() {
        let mut state = test_state();
        let mut screen = EditIncarScreen::new(&state);

        for _ in 0..=screen.param_count() {
            screen.handle_input(KeyCode::Down, &mut state);
        }
        assert!(screen.on_confirm());

        let action = screen.handle_input(KeyCode::Enter, &mut state);
        assert_eq!(action, ScreenAction::Advance);
        assert_eq!(state.incar_params.len(), 3);
    }

    // ── Grid tests ───────────────────────────────────────────────

    #[test]
    fn test_grid_navigation() {
        let mut state = large_state();
        let mut screen = EditIncarScreen::new(&state);

        assert!(screen.use_grid);
        screen.handle_input(KeyCode::Right, &mut state);
        assert!(screen.grid_col > 0 || screen.focus > 0);
    }
}
