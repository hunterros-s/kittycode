use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Widget};
use tui_core::{CancellationEvent, InputResult, Renderable};

use crate::view::BottomPaneView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResult {
    Approved,
    Denied,
    Always,
}

pub struct ApprovalView {
    title: String,
    command: String,
    result: Option<ApprovalResult>,
}

impl ApprovalView {
    pub fn new(title: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            command: command.into(),
            result: None,
        }
    }

    pub fn result(&self) -> Option<ApprovalResult> {
        self.result
    }
}

impl BottomPaneView for ApprovalView {
    fn handle_key_event(&mut self, key: KeyEvent) -> InputResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.result = Some(ApprovalResult::Approved);
                InputResult::Consumed
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.result = Some(ApprovalResult::Denied);
                InputResult::Consumed
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.result = Some(ApprovalResult::Always);
                InputResult::Consumed
            }
            KeyCode::Esc => {
                self.result = Some(ApprovalResult::Denied);
                InputResult::Consumed
            }
            _ => InputResult::Ignored,
        }
    }

    fn is_complete(&self) -> bool {
        self.result.is_some()
    }

    fn on_cancel(&mut self) -> CancellationEvent {
        self.result = Some(ApprovalResult::Denied);
        CancellationEvent::Handled
    }
}

impl Renderable for ApprovalView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                format!(" {} ", self.title),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let command_line = Line::from(vec![
            Span::styled("$ ", Style::default().fg(Color::DarkGray)),
            Span::styled(&self.command, Style::default().fg(Color::White)),
        ]);

        let options_line = Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" Yes", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" No", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("a", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Always", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("esc", Style::default().fg(Color::DarkGray)),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]);

        let mut y = inner.y;

        if y < inner.y + inner.height {
            buf.set_line(inner.x, y, &command_line, inner.width);
            y += 1;
        }

        if y < inner.y + inner.height {
            y += 1;
        }

        if y < inner.y + inner.height {
            buf.set_line(inner.x, y, &options_line, inner.width);
        }
    }

    fn height(&self, _width: u16) -> u16 {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::empty())
    }

    #[test]
    fn test_approve() {
        let mut view = ApprovalView::new("Run command?", "echo hello");
        assert!(!view.is_complete());

        view.handle_key_event(key(KeyCode::Char('y')));
        assert!(view.is_complete());
        assert_eq!(view.result(), Some(ApprovalResult::Approved));
    }

    #[test]
    fn test_deny() {
        let mut view = ApprovalView::new("Run command?", "echo hello");
        view.handle_key_event(key(KeyCode::Char('n')));
        assert_eq!(view.result(), Some(ApprovalResult::Denied));
    }

    #[test]
    fn test_always() {
        let mut view = ApprovalView::new("Run command?", "echo hello");
        view.handle_key_event(key(KeyCode::Char('a')));
        assert_eq!(view.result(), Some(ApprovalResult::Always));
    }

    #[test]
    fn test_escape_denies() {
        let mut view = ApprovalView::new("Run command?", "echo hello");
        view.handle_key_event(key(KeyCode::Esc));
        assert_eq!(view.result(), Some(ApprovalResult::Denied));
    }

    #[test]
    fn test_cancel_denies() {
        let mut view = ApprovalView::new("Run command?", "echo hello");
        let event = view.on_cancel();
        assert_eq!(event, CancellationEvent::Handled);
        assert_eq!(view.result(), Some(ApprovalResult::Denied));
    }
}
