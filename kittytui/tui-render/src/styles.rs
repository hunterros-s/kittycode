use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct MarkdownStyles {
    pub h1: Style,
    pub h2: Style,
    pub h3: Style,
    pub code_inline: Style,
    pub code_block: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub strikethrough: Style,
    pub list_marker: Style,
    pub blockquote: Style,
    pub link: Style,
}

impl Default for MarkdownStyles {
    fn default() -> Self {
        Self {
            h1: Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            h2: Style::default().add_modifier(Modifier::BOLD),
            h3: Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
            code_inline: Style::default().fg(Color::Cyan),
            code_block: Style::default().add_modifier(Modifier::DIM),
            emphasis: Style::default().add_modifier(Modifier::ITALIC),
            strong: Style::default().add_modifier(Modifier::BOLD),
            strikethrough: Style::default().add_modifier(Modifier::CROSSED_OUT),
            list_marker: Style::default().fg(Color::Blue),
            blockquote: Style::default().fg(Color::Green),
            link: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
        }
    }
}
