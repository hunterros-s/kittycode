use std::time::{Duration, Instant};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::spinner::spinner;
use crate::Cell;

const MAX_OUTPUT_LINES: usize = 5;

/// Status of a tool call execution.
#[derive(Debug, Clone)]
pub enum ToolStatus {
    Pending,
    Running { start_time: Instant },
    Complete { exit_code: i32, duration: Duration },
}

/// Output from a tool call.
#[derive(Debug, Clone, Default)]
pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
}

/// A cell that displays a tool/command execution with status.
#[derive(Debug, Clone)]
pub struct ToolCallCell {
    pub name: String,
    pub command: String,
    pub status: ToolStatus,
    pub output: Option<ToolOutput>,
}

impl ToolCallCell {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            status: ToolStatus::Pending,
            output: None,
        }
    }

    pub fn pending(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self::new(name, command)
    }

    pub fn running(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            status: ToolStatus::Running {
                start_time: Instant::now(),
            },
            output: None,
        }
    }

    fn render_output(&self, output: &ToolOutput) -> Vec<Line<'static>> {
        let dim = Style::default().add_modifier(Modifier::DIM);

        let text = if !output.stderr.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };

        if text.is_empty() {
            return vec![Line::from(Span::styled("  └ (no output)", dim))];
        }

        let all_lines: Vec<&str> = text.lines().collect();
        let mut result = Vec::new();

        // First line with └ prefix
        if let Some(first) = all_lines.first() {
            result.push(Line::from(vec![
                Span::styled("  └ ", dim),
                Span::styled((*first).to_string(), dim),
            ]));
        }

        // Middle lines with spaces prefix (up to MAX)
        let remaining = if all_lines.len() > 1 {
            &all_lines[1..]
        } else {
            &[]
        };
        let show_count = remaining.len().min(MAX_OUTPUT_LINES - 1);

        for line in &remaining[..show_count] {
            result.push(Line::from(vec![
                Span::raw("    "),
                Span::styled((*line).to_string(), dim),
            ]));
        }

        // Truncation indicator
        if remaining.len() > MAX_OUTPUT_LINES - 1 {
            let omitted = remaining.len() - (MAX_OUTPUT_LINES - 1);
            result.push(Line::from(Span::styled(
                format!("    … +{} lines", omitted),
                dim,
            )));
        }

        result
    }
}

impl Cell for ToolCallCell {
    fn render_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let dim = Style::default().add_modifier(Modifier::DIM);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let green_bold = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);
        let red_bold = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);

        let (bullet, title) = match &self.status {
            ToolStatus::Pending => (Span::styled("○", dim), "Pending"),
            ToolStatus::Running { start_time } => (spinner(*start_time), "Running"),
            ToolStatus::Complete { exit_code: 0, .. } => (Span::styled("•", green_bold), "Ran"),
            ToolStatus::Complete { .. } => (Span::styled("•", red_bold), "Ran"),
        };

        let mut title_line = vec![
            bullet,
            Span::raw(" "),
            Span::styled(title, bold),
            Span::raw(" "),
            Span::raw(self.command.clone()),
        ];

        // Add duration if complete
        if let ToolStatus::Complete { duration, .. } = &self.status {
            title_line.push(Span::styled(
                format!(" ({:.1}s)", duration.as_secs_f32()),
                dim,
            ));
        }

        let mut lines = vec![Line::from(title_line)];

        // Add output if present
        if let Some(output) = &self.output {
            let output_lines = self.render_output(output);
            lines.extend(output_lines);
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_new() {
        let cell = ToolCallCell::new("bash", "npm install");
        assert_eq!(cell.name, "bash");
        assert_eq!(cell.command, "npm install");
        assert!(matches!(cell.status, ToolStatus::Pending));
    }

    #[test]
    fn tool_call_pending() {
        let cell = ToolCallCell::pending("bash", "echo hello");
        let lines = cell.render_lines(80);

        assert_eq!(lines.len(), 1);
        let line_str = lines[0].to_string();
        assert!(line_str.contains("○"));
        assert!(line_str.contains("Pending"));
        assert!(line_str.contains("echo hello"));
    }

    #[test]
    fn tool_call_running() {
        let cell = ToolCallCell::running("bash", "npm install");
        let lines = cell.render_lines(80);

        assert_eq!(lines.len(), 1);
        let line_str = lines[0].to_string();
        assert!(line_str.contains("Running"));
        assert!(line_str.contains("npm install"));
        // Spinner character should be one of the spinner chars
        assert!(
            line_str.contains("◐")
                || line_str.contains("◓")
                || line_str.contains("◑")
                || line_str.contains("◒")
        );
    }

    #[test]
    fn tool_call_complete_success() {
        let mut cell = ToolCallCell::new("bash", "npm install");
        cell.status = ToolStatus::Complete {
            exit_code: 0,
            duration: Duration::from_secs_f32(5.5),
        };
        cell.output = Some(ToolOutput {
            stdout: "added 150 packages".to_string(),
            stderr: String::new(),
        });

        let lines = cell.render_lines(80);

        assert!(lines.len() >= 2);
        let title = lines[0].to_string();
        assert!(title.contains("•"));
        assert!(title.contains("Ran"));
        assert!(title.contains("npm install"));
        assert!(title.contains("5.5s"));

        let output = lines[1].to_string();
        assert!(output.contains("└"));
        assert!(output.contains("added 150 packages"));
    }

    #[test]
    fn tool_call_complete_failed() {
        let mut cell = ToolCallCell::new("bash", "npm test");
        cell.status = ToolStatus::Complete {
            exit_code: 1,
            duration: Duration::from_secs(2),
        };
        cell.output = Some(ToolOutput {
            stdout: String::new(),
            stderr: "Error: 2 tests failed".to_string(),
        });

        let lines = cell.render_lines(80);

        assert!(lines.len() >= 2);
        let title = lines[0].to_string();
        assert!(title.contains("•"));
        assert!(title.contains("Ran"));

        // Check that stderr is shown (since it's non-empty)
        let output = lines[1].to_string();
        assert!(output.contains("Error: 2 tests failed"));
    }

    #[test]
    fn tool_call_output_truncation() {
        let mut cell = ToolCallCell::new("bash", "long output");
        cell.status = ToolStatus::Complete {
            exit_code: 0,
            duration: Duration::from_secs(1),
        };
        cell.output = Some(ToolOutput {
            stdout: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8".to_string(),
            stderr: String::new(),
        });

        let lines = cell.render_lines(80);

        // Should have: title + 5 output lines max + truncation indicator
        // First output line + 4 more = 5 lines, then truncation
        let last_line = lines.last().unwrap().to_string();
        assert!(last_line.contains("…"));
        assert!(last_line.contains("+"));
        assert!(last_line.contains("lines"));
    }

    #[test]
    fn tool_call_empty_output() {
        let mut cell = ToolCallCell::new("bash", "silent command");
        cell.status = ToolStatus::Complete {
            exit_code: 0,
            duration: Duration::from_secs(1),
        };
        cell.output = Some(ToolOutput {
            stdout: String::new(),
            stderr: String::new(),
        });

        let lines = cell.render_lines(80);

        assert!(lines.len() >= 2);
        let output = lines[1].to_string();
        assert!(output.contains("(no output)"));
    }

    #[test]
    fn tool_call_height() {
        let cell = ToolCallCell::pending("bash", "test");
        assert_eq!(cell.height(80), 1);

        let mut cell_with_output = ToolCallCell::new("bash", "test");
        cell_with_output.status = ToolStatus::Complete {
            exit_code: 0,
            duration: Duration::from_secs(1),
        };
        cell_with_output.output = Some(ToolOutput {
            stdout: "Line 1\nLine 2".to_string(),
            stderr: String::new(),
        });
        assert_eq!(cell_with_output.height(80), 3); // title + 2 output lines
    }

    #[test]
    fn tool_call_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ToolCallCell>();
    }

    #[test]
    fn tool_output_default() {
        let output = ToolOutput::default();
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
