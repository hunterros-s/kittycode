use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use tui_core::Renderable;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FooterMode {
    #[default]
    ShortcutSummary,
    ShortcutOverlay,
    ContextOnly,
}

#[derive(Debug, Clone, Default)]
pub struct FooterProps {
    pub mode: FooterMode,
    pub custom_left: Option<String>,
    pub custom_right: Option<String>,
}

impl FooterProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_shortcuts(&mut self) {
        self.mode = match self.mode {
            FooterMode::ShortcutSummary => FooterMode::ShortcutOverlay,
            FooterMode::ShortcutOverlay => FooterMode::ShortcutSummary,
            FooterMode::ContextOnly => FooterMode::ShortcutOverlay,
        };
    }
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("/", "for commands"),
    ("?", "for shortcuts"),
    ("Shift+Enter", "for newline"),
    ("Ctrl+C", "to exit"),
];

pub struct Footer<'a> {
    props: &'a FooterProps,
    effective_mode: FooterMode,
}

impl<'a> Footer<'a> {
    pub fn new(props: &'a FooterProps, effective_mode: FooterMode) -> Self {
        Self {
            props,
            effective_mode,
        }
    }

    fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let hint_style = Style::default().fg(Color::DarkGray);
        let key_style = Style::default().fg(Color::Cyan);

        let mut spans = Vec::new();

        if let Some(ref left) = self.props.custom_left {
            spans.push(Span::styled(left.as_str(), hint_style));
            spans.push(Span::styled(" \u{00b7} ", hint_style));
        }

        spans.push(Span::styled("?", key_style));
        spans.push(Span::styled(" for shortcuts", hint_style));

        if let Some(ref right) = self.props.custom_right {
            spans.push(Span::styled(" \u{00b7} ", hint_style));
            spans.push(Span::styled(right.as_str(), hint_style));
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }

    fn render_context_only(&self, area: Rect, buf: &mut Buffer) {
        let hint_style = Style::default().fg(Color::DarkGray);

        if let Some(ref left) = self.props.custom_left {
            let line = Line::from(Span::styled(left.as_str(), hint_style));
            buf.set_line(area.x, area.y, &line, area.width);
        }
    }

    fn render_overlay(&self, area: Rect, buf: &mut Buffer) {
        let hint_style = Style::default().fg(Color::DarkGray);
        let key_style = Style::default().fg(Color::Cyan);

        let half = (SHORTCUTS.len() + 1) / 2;
        let col_width = (area.width / 2) as usize;

        for row in 0..half.min(area.height as usize) {
            let mut spans = Vec::new();

            if let Some((key, desc)) = SHORTCUTS.get(row) {
                spans.push(Span::styled(*key, key_style));
                spans.push(Span::styled(format!(" {}", desc), hint_style));
            }

            let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
            let padding = col_width.saturating_sub(left_len);
            spans.push(Span::raw(" ".repeat(padding)));

            if let Some((key, desc)) = SHORTCUTS.get(row + half) {
                spans.push(Span::styled(*key, key_style));
                spans.push(Span::styled(format!(" {}", desc), hint_style));
            }

            let line = Line::from(spans);
            buf.set_line(area.x, area.y + row as u16, &line, area.width);
        }
    }
}

impl Renderable for Footer<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        match self.effective_mode {
            FooterMode::ShortcutSummary => self.render_summary(area, buf),
            FooterMode::ContextOnly => self.render_context_only(area, buf),
            FooterMode::ShortcutOverlay => self.render_overlay(area, buf),
        }
    }

    fn height(&self, _width: u16) -> u16 {
        match self.effective_mode {
            FooterMode::ShortcutSummary | FooterMode::ContextOnly => 1,
            FooterMode::ShortcutOverlay => ((SHORTCUTS.len() + 1) / 2) as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_summary() {
        let props = FooterProps::new();
        assert_eq!(props.mode, FooterMode::ShortcutSummary);
    }

    #[test]
    fn test_toggle_shortcuts() {
        let mut props = FooterProps::new();
        props.toggle_shortcuts();
        assert_eq!(props.mode, FooterMode::ShortcutOverlay);
        props.toggle_shortcuts();
        assert_eq!(props.mode, FooterMode::ShortcutSummary);
    }

    #[test]
    fn test_footer_height_summary() {
        let props = FooterProps::new();
        let footer = Footer::new(&props, FooterMode::ShortcutSummary);
        assert_eq!(footer.height(80), 1);
    }

    #[test]
    fn test_footer_height_overlay() {
        let props = FooterProps::new();
        let footer = Footer::new(&props, FooterMode::ShortcutOverlay);
        assert_eq!(footer.height(80), 2);
    }
}
