//! File list widget: displays files in ls-style horizontal format.
//! Wraps to multiple lines based on available width.

use ratatui::text::{Line, Span, Text};

use super::STYLE_HIGHLIGHT;

/// Format a list of filenames in ls-style horizontal layout.
///
/// Files are displayed side by side with 2-space gaps, wrapping to
/// new lines when the width would be exceeded. Filenames longer than
/// the available width are truncated with "…".
///
/// Returns a `Text` that can be rendered as a `Paragraph`.
pub fn format_horizontal(files: &[String], max_width: usize) -> Text<'static> {
    if files.is_empty() {
        return Text::raw("  (no files found)");
    }

    let gap = 2;
    let indent = 2;
    let usable = max_width.saturating_sub(indent);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_width: usize = indent;

    current_spans.push(Span::raw("  "));

    for file in files {
        // Truncate filenames that exceed available width (char-safe)
        let char_count = file.chars().count();
        let display_name = if char_count > usable && usable > 1 {
            let truncated: String = file.chars().take(usable - 1).collect();
            format!("{truncated}…")
        } else {
            file.clone()
        };
        let name_len = display_name.chars().count();

        // Check if we need to wrap to a new line
        let needed = if current_width > indent {
            gap + name_len
        } else {
            name_len
        };

        if current_width + needed > max_width && current_width > indent {
            lines.push(Line::from(current_spans));
            current_spans = vec![Span::raw("  ")];
            current_width = indent;
        }

        if current_width > indent {
            current_spans.push(Span::raw("  "));
            current_width += gap;
        }

        current_spans.push(Span::styled(display_name, STYLE_HIGHLIGHT));
        current_width += name_len;
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_files() {
        let text = format_horizontal(&[], 70);
        let content = text.to_string();
        assert!(content.contains("no files found"));
    }

    #[test]
    fn test_single_file() {
        let files = vec!["POSCAR".to_string()];
        let text = format_horizontal(&files, 70);
        let content = text.to_string();
        assert!(content.contains("POSCAR"));
    }

    #[test]
    fn test_multiple_files_fit_one_line() {
        let files = vec!["POSCAR".to_string(), "CONTCAR".to_string()];
        let text = format_horizontal(&files, 70);
        assert_eq!(text.lines.len(), 1);
    }

    #[test]
    fn test_wrapping() {
        let files: Vec<String> = (0..20)
            .map(|i| format!("structure_{:02}.vasp", i))
            .collect();
        let text = format_horizontal(&files, 40);
        assert!(text.lines.len() > 1);
    }

    #[test]
    fn test_long_filename_truncated() {
        let files = vec!["a".repeat(200)];
        let text = format_horizontal(&files, 40);
        let content = text.to_string();
        // Should be truncated, not 200 chars wide
        assert!(content.len() < 200);
    }
}
