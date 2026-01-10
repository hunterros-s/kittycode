use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default, Clone)]
pub struct TextBuffer {
    text: String,
    cursor: usize,
    preferred_col: Option<usize>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    // Content access

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        self.preferred_col = None;
    }

    pub fn take_text(&mut self) -> String {
        self.cursor = 0;
        self.preferred_col = None;
        std::mem::take(&mut self.text)
    }

    // Editing

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.preferred_col = None;
    }

    pub fn delete_backward(&mut self) {
        if self.cursor > 0 {
            let prev = self.prev_grapheme_boundary();
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
            self.preferred_col = None;
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor < self.text.len() {
            let next = self.next_grapheme_boundary();
            self.text.drain(self.cursor..next);
            self.preferred_col = None;
        }
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let before = &self.text[..self.cursor];
        let mut end = self.cursor;

        // Skip trailing whitespace
        for (i, c) in before.char_indices().rev() {
            if !c.is_whitespace() {
                end = i + c.len_utf8();
                break;
            }
            end = i;
        }

        // Skip word characters
        let mut start = 0;
        for (i, c) in self.text[..end].char_indices().rev() {
            if c.is_whitespace() {
                start = i + c.len_utf8();
                break;
            }
        }

        self.text.drain(start..self.cursor);
        self.cursor = start;
        self.preferred_col = None;
    }

    // Cursor movement

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_grapheme_boundary();
            self.preferred_col = None;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.next_grapheme_boundary();
            self.preferred_col = None;
        }
    }

    pub fn move_up(&mut self) -> bool {
        let (row, col) = self.cursor_position();
        if row == 0 {
            return false;
        }

        let target_col = self.preferred_col.unwrap_or(col);
        self.preferred_col = Some(target_col);

        let prev_line_start = self.line_start(row - 1);
        let prev_line_end = self.line_end(row - 1);
        let prev_line = &self.text[prev_line_start..prev_line_end];

        self.cursor = prev_line_start + byte_offset_for_display_col(prev_line, target_col);
        true
    }

    pub fn move_down(&mut self) -> bool {
        let (row, col) = self.cursor_position();
        let line_count = self.line_count();
        if row >= line_count.saturating_sub(1) {
            return false;
        }

        let target_col = self.preferred_col.unwrap_or(col);
        self.preferred_col = Some(target_col);

        let next_line_start = self.line_start(row + 1);
        let next_line_end = self.line_end(row + 1);
        let next_line = &self.text[next_line_start..next_line_end];

        self.cursor = next_line_start + byte_offset_for_display_col(next_line, target_col);
        true
    }

    pub fn move_to_line_start(&mut self) {
        let (row, _) = self.cursor_position();
        self.cursor = self.line_start(row);
        self.preferred_col = None;
    }

    pub fn move_to_line_end(&mut self) {
        let (row, _) = self.cursor_position();
        self.cursor = self.line_end(row);
        self.preferred_col = None;
    }

    // For rendering

    pub fn cursor_position(&self) -> (usize, usize) {
        let before_cursor = &self.text[..self.cursor];
        let row = before_cursor.matches('\n').count();

        let line_start = before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = before_cursor[line_start..].width();

        (row, col)
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        LineIterator::new(&self.text)
    }

    // Helpers

    pub fn cursor_at_start(&self) -> bool {
        self.cursor == 0
    }

    pub fn cursor_at_end(&self) -> bool {
        self.cursor == self.text.len()
    }

    pub fn current_line(&self) -> usize {
        self.cursor_position().0
    }

    // Internal helpers

    fn prev_grapheme_boundary(&self) -> usize {
        let before = &self.text[..self.cursor];
        before
            .grapheme_indices(true)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_grapheme_boundary(&self) -> usize {
        let after = &self.text[self.cursor..];
        after
            .grapheme_indices(true)
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.text.len())
    }

    fn line_start(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        self.text
            .match_indices('\n')
            .nth(line - 1)
            .map(|(i, _)| i + 1)
            .unwrap_or(self.text.len())
    }

    fn line_end(&self, line: usize) -> usize {
        self.text
            .match_indices('\n')
            .nth(line)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    fn line_count(&self) -> usize {
        self.text.matches('\n').count() + 1
    }
}

fn byte_offset_for_display_col(line: &str, target_col: usize) -> usize {
    let mut col = 0;
    for (i, grapheme) in line.grapheme_indices(true) {
        let w = grapheme.width();
        if col + w > target_col {
            return i;
        }
        col += w;
    }
    line.len()
}

struct LineIterator<'a> {
    text: &'a str,
    pos: usize,
    done: bool,
}

impl<'a> LineIterator<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            pos: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let remaining = &self.text[self.pos..];
        if remaining.is_empty() {
            self.done = true;
            return Some("");
        }

        match remaining.find('\n') {
            Some(idx) => {
                let line = &remaining[..idx];
                self.pos += idx + 1;
                Some(line)
            }
            None => {
                self.done = true;
                Some(remaining)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_textarea_is_empty() {
        let ta = TextBuffer::new();
        assert!(ta.is_empty());
        assert_eq!(ta.text(), "");
        assert!(ta.cursor_at_start());
        assert!(ta.cursor_at_end());
    }

    #[test]
    fn insert_char() {
        let mut ta = TextBuffer::new();
        ta.insert_char('h');
        ta.insert_char('i');
        assert_eq!(ta.text(), "hi");
    }

    #[test]
    fn insert_newline() {
        let mut ta = TextBuffer::new();
        ta.insert_char('a');
        ta.insert_char('\n');
        ta.insert_char('b');
        assert_eq!(ta.text(), "a\nb");
    }

    #[test]
    fn delete_backward() {
        let mut ta = TextBuffer::new();
        ta.insert_char('a');
        ta.insert_char('b');
        ta.delete_backward();
        assert_eq!(ta.text(), "a");
    }

    #[test]
    fn delete_backward_at_start() {
        let mut ta = TextBuffer::new();
        ta.delete_backward();
        assert_eq!(ta.text(), "");
    }

    #[test]
    fn delete_forward() {
        let mut ta = TextBuffer::new();
        ta.insert_char('a');
        ta.insert_char('b');
        ta.move_left();
        ta.move_left();
        ta.delete_forward();
        assert_eq!(ta.text(), "b");
    }

    #[test]
    fn delete_word_backward() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello world");
        ta.delete_word_backward();
        assert_eq!(ta.text(), "hello ");
    }

    #[test]
    fn delete_word_backward_with_trailing_space() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello world  ");
        ta.delete_word_backward();
        assert_eq!(ta.text(), "hello ");
    }

    #[test]
    fn move_left_right() {
        let mut ta = TextBuffer::new();
        ta.set_text("abc");
        ta.move_left();
        assert_eq!(ta.cursor_position(), (0, 2));
        ta.move_right();
        assert_eq!(ta.cursor_position(), (0, 3));
    }

    #[test]
    fn move_up_down() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello\nworld");
        assert_eq!(ta.cursor_position(), (1, 5));

        assert!(ta.move_up());
        assert_eq!(ta.cursor_position(), (0, 5));

        assert!(ta.move_down());
        assert_eq!(ta.cursor_position(), (1, 5));
    }

    #[test]
    fn move_up_at_first_line_returns_false() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello");
        assert!(!ta.move_up());
    }

    #[test]
    fn move_down_at_last_line_returns_false() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello");
        assert!(!ta.move_down());
    }

    #[test]
    fn preferred_col_preserved() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello\nhi\nworld");
        // Cursor at end of "world" (col 5)
        ta.move_up(); // to "hi", col clamped to 2
        assert_eq!(ta.cursor_position(), (1, 2));

        ta.move_up(); // to "hello", should restore to col 5
        assert_eq!(ta.cursor_position(), (0, 5));
    }

    #[test]
    fn move_to_line_start_end() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello\nworld");
        ta.move_to_line_start();
        assert_eq!(ta.cursor_position(), (1, 0));
        ta.move_to_line_end();
        assert_eq!(ta.cursor_position(), (1, 5));
    }

    #[test]
    fn cursor_position_multiline() {
        let mut ta = TextBuffer::new();
        ta.set_text("ab\ncd\nef");
        assert_eq!(ta.cursor_position(), (2, 2));
        assert_eq!(ta.current_line(), 2);
    }

    #[test]
    fn lines_iterator() {
        let ta = TextBuffer {
            text: "a\nb\nc".to_string(),
            cursor: 0,
            preferred_col: None,
        };
        let lines: Vec<_> = ta.lines().collect();
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn lines_iterator_empty() {
        let ta = TextBuffer::new();
        let lines: Vec<_> = ta.lines().collect();
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn lines_iterator_trailing_newline() {
        let ta = TextBuffer {
            text: "a\n".to_string(),
            cursor: 0,
            preferred_col: None,
        };
        let lines: Vec<_> = ta.lines().collect();
        assert_eq!(lines, vec!["a", ""]);
    }

    #[test]
    fn set_text_moves_cursor_to_end() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello");
        assert_eq!(ta.cursor_position(), (0, 5));
        assert!(ta.cursor_at_end());
    }

    #[test]
    fn take_text_clears() {
        let mut ta = TextBuffer::new();
        ta.set_text("hello");
        let text = ta.take_text();
        assert_eq!(text, "hello");
        assert!(ta.is_empty());
        assert!(ta.cursor_at_start());
    }

    #[test]
    fn unicode_grapheme_handling() {
        let mut ta = TextBuffer::new();
        ta.insert_char('日');
        ta.insert_char('本');
        assert_eq!(ta.text(), "日本");
        // Each CJK char is 2 display columns
        assert_eq!(ta.cursor_position(), (0, 4));

        ta.move_left();
        assert_eq!(ta.cursor_position(), (0, 2));

        ta.delete_backward();
        assert_eq!(ta.text(), "本");
    }

    #[test]
    fn emoji_handling() {
        let mut ta = TextBuffer::new();
        ta.set_text("👋🏽");
        // Emoji with skin tone is a grapheme cluster
        ta.move_left();
        ta.delete_forward();
        assert_eq!(ta.text(), "");
    }
}
