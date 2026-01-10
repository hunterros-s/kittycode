use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use tui_core::{InputHandler, InputResult};

use crate::buffer::TextBuffer;
use crate::history::InputHistory;

#[derive(Debug, Clone)]
pub struct Textarea {
    buffer: TextBuffer,
    history: InputHistory,
}

impl Default for Textarea {
    fn default() -> Self {
        Self::new()
    }
}

impl Textarea {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            history: InputHistory::default(),
        }
    }

    pub fn content(&self) -> &str {
        self.buffer.text()
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.buffer.cursor_position()
    }

    pub fn clear(&mut self) {
        self.buffer.take_text();
        self.history.reset();
    }

    pub fn line_count(&self) -> usize {
        self.buffer.lines().count().max(1)
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert_char(c);
        self.history.reset();
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_text(text);
        self.history.reset();
    }

    fn should_browse_history_up(&self) -> bool {
        self.buffer.is_empty()
            || (self.buffer.current_line() == 0 && self.history.is_browsing())
    }

    fn should_browse_history_down(&self) -> bool {
        self.history.is_browsing()
    }
}

impl InputHandler for Textarea {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        // Ctrl+C exits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return InputResult::Exit;
        }

        // Enter submits, Shift+Enter for newline
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Enter inserts newline
                    self.buffer.insert_char('\n');
                    self.history.reset();
                    InputResult::Consumed
                } else {
                    // Plain Enter submits
                    let text = self.buffer.take_text();
                    if !text.is_empty() {
                        self.history.push(text.clone());
                    }
                    self.history.reset();
                    InputResult::Submit(text)
                }
            }

            // Alt+Enter as fallback for newline (works in more terminals)
            KeyCode::Char('\r') | KeyCode::Char('\n') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.buffer.insert_char('\n');
                self.history.reset();
                InputResult::Consumed
            }

            KeyCode::Up => {
                if self.should_browse_history_up() {
                    if let Some(text) = self.history.navigate_up(self.buffer.text()) {
                        self.buffer.set_text(text);
                    }
                } else {
                    self.buffer.move_up();
                }
                InputResult::Consumed
            }

            KeyCode::Down => {
                if self.should_browse_history_down() {
                    if let Some(text) = self.history.navigate_down() {
                        self.buffer.set_text(text);
                    }
                } else {
                    self.buffer.move_down();
                }
                InputResult::Consumed
            }

            KeyCode::Left => {
                self.buffer.move_left();
                InputResult::Consumed
            }

            KeyCode::Right => {
                self.buffer.move_right();
                InputResult::Consumed
            }

            KeyCode::Backspace => {
                self.buffer.delete_backward();
                self.history.reset();
                InputResult::Consumed
            }

            KeyCode::Delete => {
                self.buffer.delete_forward();
                self.history.reset();
                InputResult::Consumed
            }

            KeyCode::Home => {
                self.buffer.move_to_line_start();
                InputResult::Consumed
            }

            KeyCode::End => {
                self.buffer.move_to_line_end();
                InputResult::Consumed
            }

            KeyCode::Char(c) => {
                // Ctrl+A = line start, Ctrl+E = line end (emacs)
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'a' => self.buffer.move_to_line_start(),
                        'e' => self.buffer.move_to_line_end(),
                        'w' => self.buffer.delete_word_backward(),
                        'u' => {
                            self.buffer.take_text();
                        }
                        _ => return InputResult::Ignored,
                    }
                    return InputResult::Consumed;
                }

                self.buffer.insert_char(c);
                self.history.reset();
                InputResult::Consumed
            }

            _ => InputResult::Ignored,
        }
    }
}

impl Widget for &Textarea {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let (cursor_row, cursor_col) = self.buffer.cursor_position();
        let lines: Vec<&str> = self.buffer.lines().collect();

        for (row_idx, line) in lines.iter().enumerate().take(area.height as usize) {
            let y = area.y + row_idx as u16;

            if row_idx == cursor_row {
                let before_cursor = if cursor_col > 0 {
                    let mut col = 0;
                    let mut end_byte = 0;
                    for (i, c) in line.char_indices() {
                        if col >= cursor_col {
                            break;
                        }
                        col += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                        end_byte = i + c.len_utf8();
                    }
                    &line[..end_byte]
                } else {
                    ""
                };

                let cursor_char = line
                    .chars()
                    .skip(before_cursor.chars().count())
                    .next()
                    .unwrap_or(' ');

                let after_cursor = if cursor_col < unicode_width::UnicodeWidthStr::width(*line) {
                    let skip_bytes = before_cursor.len() + cursor_char.len_utf8();
                    if skip_bytes <= line.len() {
                        &line[skip_bytes..]
                    } else {
                        ""
                    }
                } else {
                    ""
                };

                let rendered = Line::from(vec![
                    Span::raw(before_cursor.to_string()),
                    Span::styled(
                        cursor_char.to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    ),
                    Span::raw(after_cursor.to_string()),
                ]);

                buf.set_line(area.x, y, &rendered, area.width);
            } else {
                buf.set_string(area.x, y, *line, Style::default());
            }
        }
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

    fn shift_enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)
    }

    #[test]
    fn new_composer_is_empty() {
        let c = Textarea::new();
        assert!(c.content().is_empty());
    }

    #[test]
    fn shift_enter_inserts_newline() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        c.handle_key(key(KeyCode::Char('b')));
        assert_eq!(c.content(), "a\nb");
    }

    #[test]
    fn enter_submits() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('h')));
        c.handle_key(key(KeyCode::Char('i')));

        let result = c.handle_key(key(KeyCode::Enter));
        assert_eq!(result, InputResult::Submit("hi".to_string()));
        assert!(c.content().is_empty());
    }

    #[test]
    fn ctrl_c_exits() {
        let mut c = Textarea::new();
        let result = c.handle_key(ctrl('c'));
        assert_eq!(result, InputResult::Exit);
    }

    #[test]
    fn up_on_empty_browses_history() {
        let mut c = Textarea::new();

        // Add history
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Enter));

        // Now empty, up should browse history
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "a");
    }

    #[test]
    fn up_moves_cursor_in_multiline() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        c.handle_key(key(KeyCode::Char('b')));

        assert_eq!(c.cursor(), (1, 1));
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.cursor(), (0, 1));
    }

    #[test]
    fn down_while_browsing_restores_draft() {
        let mut c = Textarea::new();

        // Add multiple history entries
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Enter));
        c.handle_key(key(KeyCode::Char('b')));
        c.handle_key(key(KeyCode::Enter));

        // Type a draft, then clear to start browsing
        c.handle_key(key(KeyCode::Char('d')));
        c.handle_key(key(KeyCode::Char('r')));
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Char('f')));
        c.handle_key(key(KeyCode::Char('t')));
        // Note: With current spec, must be empty to start browsing
        // Draft is preserved from when browsing STARTS
        c.handle_key(ctrl('u')); // Clear line

        // Now empty, browse up (draft is now "")
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "b"); // Most recent entry

        // Browse further up
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "a");

        // Navigate back down through history
        c.handle_key(key(KeyCode::Down));
        assert_eq!(c.content(), "b");

        // Navigate to draft (empty since we cleared before browsing)
        c.handle_key(key(KeyCode::Down));
        assert_eq!(c.content(), "");
    }

    #[test]
    fn cursor_movement() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Char('b')));
        c.handle_key(key(KeyCode::Char('c')));

        c.handle_key(key(KeyCode::Left));
        assert_eq!(c.cursor(), (0, 2));

        c.handle_key(key(KeyCode::Right));
        assert_eq!(c.cursor(), (0, 3));

        c.handle_key(key(KeyCode::Home));
        assert_eq!(c.cursor(), (0, 0));

        c.handle_key(key(KeyCode::End));
        assert_eq!(c.cursor(), (0, 3));
    }

    #[test]
    fn emacs_keybindings() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('t')));
        c.handle_key(key(KeyCode::Char('e')));
        c.handle_key(key(KeyCode::Char('s')));
        c.handle_key(key(KeyCode::Char('t')));

        c.handle_key(ctrl('a')); // start
        assert_eq!(c.cursor(), (0, 0));

        c.handle_key(ctrl('e')); // end
        assert_eq!(c.cursor(), (0, 4));

        c.handle_key(ctrl('u')); // clear line
        assert!(c.content().is_empty());
    }

    #[test]
    fn ctrl_w_deletes_word() {
        let mut c = Textarea::new();
        for ch in "hello world".chars() {
            c.handle_key(key(KeyCode::Char(ch)));
        }

        c.handle_key(ctrl('w'));
        assert_eq!(c.content(), "hello ");
    }

    #[test]
    fn backspace_and_delete() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Char('b')));
        c.handle_key(key(KeyCode::Char('c')));

        c.handle_key(key(KeyCode::Backspace));
        assert_eq!(c.content(), "ab");

        c.handle_key(key(KeyCode::Left));
        c.handle_key(key(KeyCode::Delete));
        assert_eq!(c.content(), "a");
    }

    #[test]
    fn history_not_triggered_on_second_line() {
        let mut c = Textarea::new();

        // Add history
        c.handle_key(key(KeyCode::Char('x')));
        c.handle_key(key(KeyCode::Enter)); // submit

        // Type multiline using Shift+Enter
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        c.handle_key(key(KeyCode::Char('b')));

        // On second line, Up should move cursor, not browse history
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "a\nb"); // content unchanged
        assert_eq!(c.cursor(), (0, 1)); // moved to first line
    }

    #[test]
    fn submit_adds_to_history() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(key(KeyCode::Enter));

        c.handle_key(key(KeyCode::Char('b')));
        c.handle_key(key(KeyCode::Enter));

        // Empty, browse history
        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "b");

        c.handle_key(key(KeyCode::Up));
        assert_eq!(c.content(), "a");
    }

    #[test]
    fn line_count_empty() {
        let c = Textarea::new();
        assert_eq!(c.line_count(), 1);
    }

    #[test]
    fn line_count_single_line() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('h')));
        c.handle_key(key(KeyCode::Char('i')));
        assert_eq!(c.line_count(), 1);
    }

    #[test]
    fn line_count_two_lines() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        c.handle_key(key(KeyCode::Char('b')));
        assert_eq!(c.line_count(), 2);
    }

    #[test]
    fn line_count_multiple_lines() {
        let mut c = Textarea::new();
        // Line 1
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        // Line 2 (empty)
        c.handle_key(shift_enter());
        // Line 3
        c.handle_key(key(KeyCode::Char('b')));
        c.handle_key(shift_enter());
        // Line 4
        c.handle_key(key(KeyCode::Char('c')));
        assert_eq!(c.line_count(), 4);
    }

    #[test]
    fn line_count_trailing_newline() {
        let mut c = Textarea::new();
        c.handle_key(key(KeyCode::Char('a')));
        c.handle_key(shift_enter());
        // Trailing newline means cursor is on a new empty line
        assert_eq!(c.line_count(), 2);
    }

    #[test]
    fn line_count_five_lines() {
        let mut c = Textarea::new();
        for i in 0..5 {
            if i > 0 {
                c.handle_key(shift_enter());
            }
            c.handle_key(key(KeyCode::Char('x')));
        }
        assert_eq!(c.content(), "x\nx\nx\nx\nx");
        assert_eq!(c.line_count(), 5);
    }
}
