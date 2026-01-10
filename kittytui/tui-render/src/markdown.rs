use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::styles::MarkdownStyles;

#[derive(Clone)]
struct IndentContext {
    prefix: Vec<Span<'static>>,
    marker: Option<Vec<Span<'static>>>,
}

struct MarkdownRenderer {
    lines: Vec<Line<'static>>,
    styles: MarkdownStyles,

    style_stack: Vec<Style>,
    list_indices: Vec<Option<u64>>,

    in_code_block: bool,
    code_block_content: String,

    current_heading: Option<HeadingLevel>,
    pending_link_url: Option<String>,

    pending_marker_line: bool,

    indent_stack: Vec<IndentContext>,

    current_line_content: Option<Line<'static>>,
    current_initial_indent: Vec<Span<'static>>,
    current_subsequent_indent: Vec<Span<'static>>,

    // Table state
    in_table: bool,
    table_alignments: Vec<Alignment>,
    table_rows: Vec<Vec<String>>,
    current_cell: String,
}

impl MarkdownRenderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            styles: MarkdownStyles::default(),
            style_stack: vec![Style::default()],
            list_indices: Vec::new(),
            in_code_block: false,
            code_block_content: String::new(),
            current_heading: None,
            pending_link_url: None,
            pending_marker_line: false,
            indent_stack: Vec::new(),
            current_line_content: None,
            current_initial_indent: Vec::new(),
            current_subsequent_indent: Vec::new(),
            in_table: false,
            table_alignments: Vec::new(),
            table_rows: Vec::new(),
            current_cell: String::new(),
        }
    }

    fn push_span(&mut self, span: Span<'static>) {
        if let Some(line) = self.current_line_content.as_mut() {
            line.spans.push(span);
        } else {
            self.push_line(Line::from(vec![span]));
        }
    }

    fn push_line(&mut self, line: Line<'static>) {
        self.flush_current_line();
        let was_pending = self.pending_marker_line;
        self.current_initial_indent = self.prefix_spans(was_pending);
        self.current_subsequent_indent = self.prefix_spans(false);
        self.current_line_content = Some(line);
        self.pending_marker_line = false;
    }

    fn is_last_marker_index(&self, idx: usize) -> bool {
        for i in (idx + 1)..self.indent_stack.len() {
            if self.indent_stack[i].marker.is_some() {
                return false;
            }
        }
        true
    }

    fn prefix_spans(&self, include_marker: bool) -> Vec<Span<'static>> {
        let mut prefix = Vec::new();
        for (i, ctx) in self.indent_stack.iter().enumerate() {
            if include_marker && ctx.marker.is_some() && self.is_last_marker_index(i) {
                prefix.extend(ctx.marker.as_ref().unwrap().clone());
            } else {
                prefix.extend(ctx.prefix.clone());
            }
        }
        prefix
    }

    fn flush_current_line(&mut self) {
        if let Some(mut line) = self.current_line_content.take() {
            let mut spans = self.current_initial_indent.clone();
            spans.append(&mut line.spans);
            self.lines.push(Line::from(spans));
            self.current_initial_indent.clear();
            self.current_subsequent_indent.clear();
        }
    }

    fn push_inline_style(&mut self, style: Style) {
        let current = self.style_stack.last().copied().unwrap_or_default();
        let merged = current.patch(style);
        self.style_stack.push(merged);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            Event::Rule => self.horizontal_rule(),
            Event::TaskListMarker(checked) => self.task_list_marker(checked),
            _ => {}
        }
    }

    fn task_list_marker(&mut self, checked: bool) {
        let checkbox = if checked { "☑ " } else { "☐ " };
        if let Some(ctx) = self.indent_stack.last_mut() {
            ctx.marker = Some(vec![Span::styled(
                checkbox.to_string(),
                self.styles.list_marker,
            )]);
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.current_heading = Some(level);
                let style = match level {
                    HeadingLevel::H1 => self.styles.h1,
                    HeadingLevel::H2 => self.styles.h2,
                    HeadingLevel::H3 => self.styles.h3,
                    _ => self.styles.h2,
                };
                self.push_inline_style(style);
            }
            Tag::BlockQuote(_) => {
                self.indent_stack.push(IndentContext {
                    prefix: vec![Span::styled("▎ ", self.styles.blockquote)],
                    marker: None,
                });
                self.push_inline_style(self.styles.blockquote);
            }
            Tag::CodeBlock(_) => {
                self.in_code_block = true;
                self.code_block_content.clear();
            }
            Tag::List(ordered) => {
                if ordered.is_some() {
                    self.list_indices.push(Some(0));
                } else {
                    self.list_indices.push(None);
                }
            }
            Tag::Item => {
                self.start_item();
            }
            Tag::Emphasis => {
                self.push_inline_style(self.styles.emphasis);
            }
            Tag::Strong => {
                self.push_inline_style(self.styles.strong);
            }
            Tag::Strikethrough => {
                self.push_inline_style(self.styles.strikethrough);
            }
            Tag::Link { dest_url, .. } => {
                self.pending_link_url = Some(dest_url.to_string());
                self.push_inline_style(self.styles.link);
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments.to_vec();
                self.table_rows.clear();
                self.in_table = true;
            }
            Tag::TableHead | Tag::TableRow => {
                self.table_rows.push(Vec::new());
            }
            Tag::TableCell => {
                self.current_cell.clear();
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_current_line();
                self.lines.push(Line::default()); // blank line after paragraph
            }
            TagEnd::Heading(_) => {
                if let Some(level) = self.current_heading.take() {
                    let prefix = match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        HeadingLevel::H4 => "#### ",
                        HeadingLevel::H5 => "##### ",
                        HeadingLevel::H6 => "###### ",
                    };
                    let style = self.current_style();
                    if let Some(line) = self.current_line_content.as_mut() {
                        line.spans.insert(0, Span::styled(prefix.to_string(), style));
                    }
                }
                self.pop_style();
                self.flush_current_line();
                self.lines.push(Line::default()); // blank line after heading
            }
            TagEnd::BlockQuote => {
                self.indent_stack.pop();
                self.pop_style();
                self.flush_current_line();
                self.lines.push(Line::default()); // blank line after blockquote
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let content = std::mem::take(&mut self.code_block_content);
                let style = self.styles.code_block;
                for line in content.lines() {
                    self.push_line(Line::from(vec![Span::styled(line.to_string(), style)]));
                }
                self.flush_current_line();
                self.lines.push(Line::default()); // blank line after code block
            }
            TagEnd::List(_) => {
                self.list_indices.pop();
                self.flush_current_line();
                self.lines.push(Line::default()); // blank line after list
            }
            TagEnd::Item => {
                self.indent_stack.pop();
                self.flush_current_line();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.pop_style();
            }
            TagEnd::Link => {
                if let Some(url) = self.pending_link_url.take() {
                    let style = self.current_style();
                    self.push_span(Span::styled(format!(" ({})", url), style));
                }
                self.pop_style();
            }
            TagEnd::TableCell => {
                if let Some(row) = self.table_rows.last_mut() {
                    row.push(std::mem::take(&mut self.current_cell));
                }
            }
            TagEnd::Table => {
                self.render_table();
                self.in_table = false;
                self.lines.push(Line::default()); // blank line after table
            }
            _ => {}
        }
    }

    fn start_item(&mut self) {
        self.pending_marker_line = true;
        let depth = self.list_indices.len();
        let width = depth * 4 - 3;

        let marker = match self.list_indices.last_mut() {
            Some(None) => Some(vec![Span::styled(
                format!("{}- ", " ".repeat(width.saturating_sub(1))),
                self.styles.list_marker,
            )]),
            Some(Some(index)) => {
                *index += 1;
                let num = *index;
                Some(vec![Span::styled(
                    format!("{:>width$}. ", num, width = width),
                    self.styles.list_marker,
                )])
            }
            None => None,
        };

        let indent_prefix = vec![Span::from(" ".repeat(width + 2))];
        self.indent_stack.push(IndentContext {
            prefix: indent_prefix,
            marker,
        });
    }

    fn render_table(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }

        let col_count = self.table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return;
        }

        let mut widths = vec![0usize; col_count];
        for row in &self.table_rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.width());
            }
        }

        for (row_idx, row) in self.table_rows.iter().enumerate() {
            let cells: Vec<String> = (0..col_count)
                .map(|i| {
                    let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    let w = widths[i];
                    let cell_width = cell.width();
                    let padding = w.saturating_sub(cell_width);

                    match self.table_alignments.get(i) {
                        Some(Alignment::Center) => {
                            let left = padding / 2;
                            let right = padding - left;
                            format!("{}{}{}", " ".repeat(left), cell, " ".repeat(right))
                        }
                        Some(Alignment::Right) => {
                            format!("{}{}", " ".repeat(padding), cell)
                        }
                        _ => {
                            format!("{}{}", cell, " ".repeat(padding))
                        }
                    }
                })
                .collect();

            self.lines
                .push(Line::raw(format!("│ {} │", cells.join(" │ "))));

            if row_idx == 0 && self.table_rows.len() > 1 {
                let sep: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
                self.lines
                    .push(Line::raw(format!("├─{}─┤", sep.join("─┼─"))));
            }
        }
    }

    fn text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }

        if self.in_table {
            self.current_cell.push_str(text);
            return;
        }

        if self.pending_marker_line {
            self.push_line(Line::default());
        }
        self.pending_marker_line = false;

        for (i, line) in text.lines().enumerate() {
            if i > 0 {
                self.push_line(Line::default());
            }
            let style = self.current_style();
            self.push_span(Span::styled(line.to_string(), style));
        }
    }

    fn inline_code(&mut self, code: &str) {
        self.push_span(Span::styled(
            format!("`{}`", code),
            self.styles.code_inline,
        ));
    }

    fn soft_break(&mut self) {
        self.push_span(Span::raw(" "));
    }

    fn hard_break(&mut self) {
        self.flush_current_line();
    }

    fn horizontal_rule(&mut self) {
        self.flush_current_line();
        self.lines.push(Line::from(vec![Span::styled(
            "───────────────────────────────────────",
            self.styles.code_block,
        )]));
        self.lines.push(Line::default()); // blank line after hr
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_current_line();

        while self
            .lines
            .last()
            .map(|l| l.spans.is_empty())
            .unwrap_or(false)
        {
            self.lines.pop();
        }

        self.lines
    }
}

pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(text, options);
    let mut renderer = MarkdownRenderer::new();

    for event in parser {
        renderer.handle_event(event);
    }

    renderer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    fn line_to_string(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn test_plain_text() {
        let lines = render_markdown("Hello world");
        assert_eq!(lines.len(), 1);
        assert_eq!(line_to_string(&lines[0]), "Hello world");
    }

    #[test]
    fn test_bold() {
        let lines = render_markdown("Hello **bold** world");
        assert_eq!(lines.len(), 1);
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn test_italic() {
        let lines = render_markdown("Hello *italic* world");
        assert_eq!(lines.len(), 1);
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::ITALIC)));
    }

    #[test]
    fn test_bold_italic_nested() {
        let lines = render_markdown("***bold and italic***");
        assert!(!lines.is_empty());
        let has_both = lines[0].spans.iter().any(|s| {
            s.style.add_modifier.contains(Modifier::BOLD)
                && s.style.add_modifier.contains(Modifier::ITALIC)
        });
        assert!(has_both);
    }

    #[test]
    fn test_inline_code() {
        let lines = render_markdown("Use `code` here");
        assert_eq!(lines.len(), 1);
        let text = line_to_string(&lines[0]);
        assert!(text.contains("`code`"));
    }

    #[test]
    fn test_heading_h1() {
        let lines = render_markdown("# Heading");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.starts_with("# "));
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD)
                && s.style.add_modifier.contains(Modifier::UNDERLINED)));
    }

    #[test]
    fn test_heading_h2() {
        let lines = render_markdown("## Heading");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.starts_with("## "));
    }

    #[test]
    fn test_heading_h3() {
        let lines = render_markdown("### Heading");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.starts_with("### "));
    }

    #[test]
    fn test_unordered_list() {
        let lines = render_markdown("- Item 1\n- Item 2\n- Item 3");
        assert!(lines.len() >= 3);
        let text0 = line_to_string(&lines[0]);
        assert!(text0.contains("-"));
        assert!(text0.contains("Item 1"));
    }

    #[test]
    fn test_ordered_list() {
        let lines = render_markdown("1. First\n2. Second\n3. Third");
        assert!(lines.len() >= 3);
        let text0 = line_to_string(&lines[0]);
        assert!(text0.contains("1."));
        assert!(text0.contains("First"));
    }

    #[test]
    fn test_blockquote() {
        let lines = render_markdown("> quoted text");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.contains("quoted text"));
    }

    #[test]
    fn test_strikethrough() {
        let lines = render_markdown("~~struck~~");
        assert!(!lines.is_empty());
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::CROSSED_OUT)));
    }

    #[test]
    fn test_link() {
        let lines = render_markdown("[text](https://example.com)");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.contains("text"));
        assert!(text.contains("https://example.com"));
    }

    #[test]
    fn test_horizontal_rule() {
        let lines = render_markdown("---");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.contains("───"));
    }

    #[test]
    fn test_code_block() {
        let lines = render_markdown("```\nlet x = 1;\n```");
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| line_to_string(l).contains("let x = 1")));
    }

    #[test]
    fn test_empty_input() {
        let lines = render_markdown("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_multiple_paragraphs() {
        let lines = render_markdown("First paragraph.\n\nSecond paragraph.");
        assert!(lines.len() >= 2);
        assert!(lines.iter().any(|l| line_to_string(l).contains("First")));
        assert!(lines.iter().any(|l| line_to_string(l).contains("Second")));
    }

    #[test]
    fn test_task_list_unchecked() {
        let lines = render_markdown("- [ ] todo item");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.contains("☐"));
        assert!(text.contains("todo item"));
    }

    #[test]
    fn test_task_list_checked() {
        let lines = render_markdown("- [x] done item");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        assert!(text.contains("☑"));
        assert!(text.contains("done item"));
    }

    #[test]
    fn test_table() {
        let lines = render_markdown("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(lines.len() >= 2);
        let header = line_to_string(&lines[0]);
        assert!(header.contains("A"));
        assert!(header.contains("B"));
        assert!(header.contains("│"));
    }

    #[test]
    fn test_smart_punctuation() {
        let lines = render_markdown("\"quoted\" and 'single' and ...");
        assert!(!lines.is_empty());
        let text = line_to_string(&lines[0]);
        // Smart quotes: " " or ' '
        assert!(text.contains('\u{201C}') || text.contains('\u{201D}'));
        // Ellipsis: …
        assert!(text.contains('\u{2026}'));
    }
}
