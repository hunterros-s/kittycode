use std::borrow::Cow;

use ratatui::text::Line;
use tui_core::{CellCategory, CellData};
use tui_render::render_markdown;

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
}

impl UserMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl CellData for UserMessage {
    fn category(&self) -> CellCategory {
        CellCategory::UserMessage
    }

    fn text_content(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    text: String,
    rendered: Vec<Line<'static>>,
}

impl AgentMessage {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let rendered = render_markdown(&text);
        Self { text, rendered }
    }

    pub fn render(&self) -> &[Line<'static>] {
        &self.rendered
    }

    pub fn height(&self, _width: u16) -> u16 {
        self.rendered.len() as u16
    }
}

impl CellData for AgentMessage {
    fn category(&self) -> CellCategory {
        CellCategory::AgentMessage
    }

    fn text_content(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
}

#[derive(Debug, Clone)]
pub enum Cell {
    User(UserMessage),
    Agent(AgentMessage),
}

impl Cell {
    pub fn user(text: impl Into<String>) -> Self {
        Cell::User(UserMessage::new(text))
    }

    pub fn agent(text: impl Into<String>) -> Self {
        Cell::Agent(AgentMessage::new(text))
    }
}
