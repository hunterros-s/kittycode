use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Default)]
pub struct WrapOptions {
    pub initial_indent: String,
    pub subsequent_indent: String,
}

impl WrapOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial_indent(mut self, indent: impl Into<String>) -> Self {
        self.initial_indent = indent.into();
        self
    }

    pub fn with_subsequent_indent(mut self, indent: impl Into<String>) -> Self {
        self.subsequent_indent = indent.into();
        self
    }
}

pub fn word_wrap(line: Line<'static>, width: usize, opts: &WrapOptions) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![line];
    }

    let initial_indent_width = opts.initial_indent.width();
    let subsequent_indent_width = opts.subsequent_indent.width();

    let mut result: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_width: usize = 0;
    let mut is_first_line = true;

    let effective_width = |first: bool| -> usize {
        if first {
            width.saturating_sub(initial_indent_width)
        } else {
            width.saturating_sub(subsequent_indent_width)
        }
    };

    for span in line.spans {
        let style = span.style;
        let text = span.content.into_owned();

        for word in split_keeping_whitespace(&text) {
            let word_width = word.width();

            if current_width + word_width > effective_width(is_first_line) && current_width > 0 {
                let indent = if is_first_line {
                    &opts.initial_indent
                } else {
                    &opts.subsequent_indent
                };

                let mut line_spans = Vec::new();
                if !indent.is_empty() {
                    line_spans.push(Span::raw(indent.clone()));
                }
                line_spans.append(&mut current_spans);
                result.push(Line::from(line_spans));

                current_spans = Vec::new();
                current_width = 0;
                is_first_line = false;
            }

            if !word.is_empty() {
                current_spans.push(Span::styled(word.to_string(), style));
                current_width += word_width;
            }
        }
    }

    if !current_spans.is_empty() || result.is_empty() {
        let indent = if is_first_line {
            &opts.initial_indent
        } else {
            &opts.subsequent_indent
        };

        let mut line_spans = Vec::new();
        if !indent.is_empty() {
            line_spans.push(Span::raw(indent.clone()));
        }
        line_spans.append(&mut current_spans);
        result.push(Line::from(line_spans));
    }

    result
}

fn split_keeping_whitespace(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_whitespace = false;

    for (i, c) in s.char_indices() {
        let is_ws = c.is_whitespace();
        if is_ws != in_whitespace && i > start {
            result.push(&s[start..i]);
            start = i;
        }
        in_whitespace = is_ws;
    }

    if start < s.len() {
        result.push(&s[start..]);
    }

    result
}

pub fn wrap_lines(lines: Vec<Line<'static>>, width: usize, opts: &WrapOptions) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .flat_map(|line| word_wrap(line, width, opts))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Modifier, Style};

    #[test]
    fn wrap_short_line_no_change() {
        let line = Line::from("hello");
        let wrapped = word_wrap(line, 80, &WrapOptions::default());
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].to_string(), "hello");
    }

    #[test]
    fn wrap_long_line_breaks() {
        let line = Line::from("hello world foo bar");
        let wrapped = word_wrap(line, 10, &WrapOptions::default());
        assert!(wrapped.len() > 1);
    }

    #[test]
    fn wrap_preserves_style() {
        let line = Line::from(vec![
            Span::styled("bold", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" text"),
        ]);
        let wrapped = word_wrap(line, 80, &WrapOptions::default());
        assert_eq!(wrapped.len(), 1);
        assert!(wrapped[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier == Modifier::BOLD));
    }

    #[test]
    fn wrap_handles_emoji() {
        let line = Line::from("Hello 👋 World 🌍");
        let wrapped = word_wrap(line, 80, &WrapOptions::default());
        assert!(!wrapped.is_empty());
        let text = wrapped[0].to_string();
        assert!(text.contains("👋"));
        assert!(text.contains("🌍"));
    }

    #[test]
    fn wrap_handles_cjk() {
        let line = Line::from("日本語テスト");
        let wrapped = word_wrap(line, 80, &WrapOptions::default());
        assert!(!wrapped.is_empty());
        assert!(wrapped[0].to_string().contains("日本語"));
    }

    #[test]
    fn wrap_with_indent() {
        let line = Line::from("hello world foo bar baz");
        let opts = WrapOptions::new()
            .with_initial_indent("  ")
            .with_subsequent_indent("    ");
        let wrapped = word_wrap(line, 15, &opts);
        assert!(wrapped.len() > 1);
        assert!(wrapped[0].to_string().starts_with("  "));
        if wrapped.len() > 1 {
            assert!(wrapped[1].to_string().starts_with("    "));
        }
    }

    #[test]
    fn wrap_empty_line() {
        let line = Line::from("");
        let wrapped = word_wrap(line, 80, &WrapOptions::default());
        assert_eq!(wrapped.len(), 1);
    }

    #[test]
    fn wrap_zero_width() {
        let line = Line::from("hello");
        let wrapped = word_wrap(line, 0, &WrapOptions::default());
        assert_eq!(wrapped.len(), 1);
    }

    #[test]
    fn wrap_single_long_word() {
        let line = Line::from("superlongwordthatcannotbreak");
        let wrapped = word_wrap(line, 10, &WrapOptions::default());
        assert!(!wrapped.is_empty());
    }

    #[test]
    fn wrap_multiple_lines() {
        let lines = vec![
            Line::from("first line here"),
            Line::from("second line here"),
        ];
        let wrapped = wrap_lines(lines, 10, &WrapOptions::default());
        assert!(wrapped.len() >= 2);
    }
}
