use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let parser = Parser::new_ext(text, Options::all());
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let new_style = match &tag {
                    Tag::Emphasis => style_stack.last().copied().unwrap_or_default().add_modifier(Modifier::ITALIC),
                    Tag::Strong => style_stack.last().copied().unwrap_or_default().add_modifier(Modifier::BOLD),
                    Tag::Heading { .. } => Style::default().add_modifier(Modifier::BOLD),
                    Tag::CodeBlock(_) => Style::default().add_modifier(Modifier::DIM),
                    _ => style_stack.last().copied().unwrap_or_default(),
                };
                style_stack.push(new_style);
                if matches!(tag, Tag::Item) {
                    current_spans.push(Span::raw("• "));
                }
            }
            Event::End(tag_end) => {
                style_stack.pop();
                match tag_end {
                    TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock => {
                        if !current_spans.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_spans)));
                        }
                        lines.push(Line::default());
                    }
                    TagEnd::Item => {
                        if !current_spans.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_spans)));
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let style = style_stack.last().copied().unwrap_or_default();
                current_spans.push(Span::styled(text.to_string(), style));
            }
            Event::Code(code) => {
                let style = Style::default().add_modifier(Modifier::DIM);
                current_spans.push(Span::styled(format!("`{}`", code), style));
            }
            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    while lines.last().map(|l| l.spans.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let lines = render_markdown("Hello world");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "Hello world");
    }

    #[test]
    fn test_bold() {
        let lines = render_markdown("Hello **bold** world");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.iter().any(|s| s.style.add_modifier == Modifier::BOLD));
    }

    #[test]
    fn test_italic() {
        let lines = render_markdown("Hello *italic* world");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.iter().any(|s| s.style.add_modifier == Modifier::ITALIC));
    }

    #[test]
    fn test_inline_code() {
        let lines = render_markdown("Use `code` here");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].to_string().contains("`code`"));
    }

    #[test]
    fn test_heading() {
        let lines = render_markdown("# Heading");
        assert!(!lines.is_empty());
        assert!(lines[0].spans.iter().any(|s| s.style.add_modifier == Modifier::BOLD));
    }

    #[test]
    fn test_list() {
        let lines = render_markdown("- Item 1\n- Item 2\n- Item 3");
        assert!(lines.len() >= 3);
        assert!(lines[0].to_string().contains("•"));
        assert!(lines[0].to_string().contains("Item 1"));
        assert!(lines[1].to_string().contains("Item 2"));
    }
}
