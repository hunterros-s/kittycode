use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::Cell;

/// A cell that displays an error message with optional details.
#[derive(Debug, Clone)]
pub struct ErrorCell {
    pub summary: String,
    pub details: Option<String>,
}

impl ErrorCell {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            details: None,
        }
    }

    pub fn with_details(summary: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            details: Some(details.into()),
        }
    }
}

impl Cell for ErrorCell {
    fn render_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let red_bold = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);
        let red_dim = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::DIM);

        let mut lines = vec![Line::from(vec![
            Span::styled("⚠ ", red_bold),
            Span::styled(self.summary.clone(), red_bold),
        ])];

        if let Some(details) = &self.details {
            for line in details.lines() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line.to_string(), red_dim),
                ]));
            }
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_cell_new() {
        let cell = ErrorCell::new("Connection failed");
        assert_eq!(cell.summary, "Connection failed");
        assert!(cell.details.is_none());
    }

    #[test]
    fn error_cell_with_details() {
        let cell = ErrorCell::with_details("Connection failed", "Timeout after 30s");
        assert_eq!(cell.summary, "Connection failed");
        assert_eq!(cell.details, Some("Timeout after 30s".to_string()));
    }

    #[test]
    fn error_cell_render_no_details() {
        let cell = ErrorCell::new("Test error");
        let lines = cell.render_lines(80);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].to_string().contains("⚠"));
        assert!(lines[0].to_string().contains("Test error"));
    }

    #[test]
    fn error_cell_render_with_details() {
        let cell = ErrorCell::with_details("Test error", "Some details");
        let lines = cell.render_lines(80);

        assert_eq!(lines.len(), 2);
        assert!(lines[0].to_string().contains("Test error"));
        assert!(lines[1].to_string().contains("Some details"));
    }

    #[test]
    fn error_cell_render_multiline_details() {
        let cell = ErrorCell::with_details("Test error", "Line 1\nLine 2\nLine 3");
        let lines = cell.render_lines(80);

        assert_eq!(lines.len(), 4); // 1 summary + 3 detail lines
        assert!(lines[1].to_string().contains("Line 1"));
        assert!(lines[2].to_string().contains("Line 2"));
        assert!(lines[3].to_string().contains("Line 3"));
    }

    #[test]
    fn error_cell_height() {
        let cell = ErrorCell::new("Error");
        assert_eq!(cell.height(80), 1);

        let cell_with_details = ErrorCell::with_details("Error", "Detail 1\nDetail 2");
        assert_eq!(cell_with_details.height(80), 3);
    }

    #[test]
    fn error_cell_has_red_styling() {
        let cell = ErrorCell::new("Error");
        let lines = cell.render_lines(80);

        // Check that spans have red color
        let first_span = &lines[0].spans[0];
        assert_eq!(first_span.style.fg, Some(Color::Red));
    }

    #[test]
    fn error_cell_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ErrorCell>();
    }
}
