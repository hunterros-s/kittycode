use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use tui_core::{InputHandler, InputResult};

#[derive(Debug, Default, Clone)]
pub struct Textarea {
    content: String,
    cursor: usize,
}

impl Textarea {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor > 0 {
            let prev_char_boundary = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.remove(prev_char_boundary);
            self.cursor = prev_char_boundary;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = self.content[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.content.len());
        }
    }
}

impl InputHandler for Textarea {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return InputResult::Exit,
                _ => return InputResult::Ignored,
            }
        }

        match key.code {
            KeyCode::Char(c) => {
                self.insert_char(c);
                InputResult::Consumed
            }
            KeyCode::Backspace => {
                self.delete_char_before_cursor();
                InputResult::Consumed
            }
            KeyCode::Left => {
                self.move_cursor_left();
                InputResult::Consumed
            }
            KeyCode::Right => {
                self.move_cursor_right();
                InputResult::Consumed
            }
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.content);
                self.cursor = 0;
                InputResult::Submit(text)
            }
            _ => InputResult::Ignored,
        }
    }
}

impl Widget for &Textarea {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let before_cursor = &self.content[..self.cursor];
        let after_cursor = &self.content[self.cursor..];

        let cursor_char = after_cursor.chars().next().unwrap_or(' ');
        let after_cursor_rest = if after_cursor.is_empty() {
            ""
        } else {
            &after_cursor[cursor_char.len_utf8()..]
        };

        let line = Line::from(vec![
            Span::raw(before_cursor.to_string()),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after_cursor_rest.to_string()),
        ]);

        buf.set_line(area.x, area.y, &line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_insert_chars() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('h')));
        ta.handle_key(key(KeyCode::Char('i')));
        assert_eq!(ta.content(), "hi");
        assert_eq!(ta.cursor(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('a')));
        ta.handle_key(key(KeyCode::Char('b')));
        ta.handle_key(key(KeyCode::Backspace));
        assert_eq!(ta.content(), "a");
        assert_eq!(ta.cursor(), 1);
    }

    #[test]
    fn test_backspace_empty() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Backspace));
        assert_eq!(ta.content(), "");
        assert_eq!(ta.cursor(), 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('a')));
        ta.handle_key(key(KeyCode::Char('b')));
        ta.handle_key(key(KeyCode::Char('c')));
        assert_eq!(ta.cursor(), 3);

        ta.handle_key(key(KeyCode::Left));
        assert_eq!(ta.cursor(), 2);

        ta.handle_key(key(KeyCode::Left));
        assert_eq!(ta.cursor(), 1);

        ta.handle_key(key(KeyCode::Right));
        assert_eq!(ta.cursor(), 2);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Left));
        assert_eq!(ta.cursor(), 0);

        ta.handle_key(key(KeyCode::Char('x')));
        ta.handle_key(key(KeyCode::Right));
        ta.handle_key(key(KeyCode::Right));
        assert_eq!(ta.cursor(), 1);
    }

    #[test]
    fn test_submit() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('t')));
        ta.handle_key(key(KeyCode::Char('e')));
        ta.handle_key(key(KeyCode::Char('s')));
        ta.handle_key(key(KeyCode::Char('t')));

        let result = ta.handle_key(key(KeyCode::Enter));
        assert_eq!(result, InputResult::Submit("test".to_string()));
        assert_eq!(ta.content(), "");
        assert_eq!(ta.cursor(), 0);
    }

    #[test]
    fn test_ctrl_c_exit() {
        let mut ta = Textarea::new();
        let result = ta.handle_key(ctrl('c'));
        assert_eq!(result, InputResult::Exit);
    }

    #[test]
    fn test_insert_at_cursor() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('a')));
        ta.handle_key(key(KeyCode::Char('c')));
        ta.handle_key(key(KeyCode::Left));
        ta.handle_key(key(KeyCode::Char('b')));
        assert_eq!(ta.content(), "abc");
    }

    #[test]
    fn test_unicode() {
        let mut ta = Textarea::new();
        ta.handle_key(key(KeyCode::Char('日')));
        ta.handle_key(key(KeyCode::Char('本')));
        assert_eq!(ta.content(), "日本");

        ta.handle_key(key(KeyCode::Left));
        ta.handle_key(key(KeyCode::Backspace));
        assert_eq!(ta.content(), "本");
    }
}
