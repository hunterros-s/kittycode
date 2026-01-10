use std::borrow::Cow;
use std::fmt::Debug;

use ratatui::text::Line;
use tui_core::{CellCategory, CellData};
use tui_render::{render_markdown, wrap_lines, WrapOptions};

mod error;
mod spinner;
mod tool_call;

pub use error::ErrorCell;
pub use spinner::spinner;
pub use tool_call::{ToolCallCell, ToolOutput, ToolStatus};

/// Trait for renderable cell content.
pub trait Cell: Debug + Send + Sync {
    /// Render cell content as styled lines.
    fn render_lines(&self, width: u16) -> Vec<Line<'static>>;

    /// Height in lines for layout.
    fn height(&self, width: u16) -> u16 {
        self.render_lines(width).len() as u16
    }
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
}

impl UserMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn render(&self) -> Vec<Line<'static>> {
        self.text
            .lines()
            .map(|line| Line::raw(line.to_string()))
            .collect()
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

    pub fn render_wrapped(&self, width: u16) -> Vec<Line<'static>> {
        if width == 0 {
            return self.rendered.clone();
        }
        wrap_lines(self.rendered.clone(), WrapOptions::new(width as usize))
    }

    pub fn height(&self, width: u16) -> u16 {
        if width == 0 {
            self.rendered.len() as u16
        } else {
            self.render_wrapped(width).len() as u16
        }
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
pub enum MessageCell {
    User(UserMessage),
    Agent(AgentMessage),
}

impl MessageCell {
    pub fn user(text: impl Into<String>) -> Self {
        MessageCell::User(UserMessage::new(text))
    }

    pub fn agent(text: impl Into<String>) -> Self {
        MessageCell::Agent(AgentMessage::new(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_new() {
        let msg = UserMessage::new("hello");
        assert_eq!(msg.text, "hello");
    }

    #[test]
    fn user_message_multiline_renders_separate_lines() {
        let msg = UserMessage::new("line one\nline two");
        let rendered = msg.render();
        assert_eq!(rendered.len(), 2, "Multiline message should render as separate lines");
        assert_eq!(rendered[0].to_string(), "line one");
        assert_eq!(rendered[1].to_string(), "line two");
    }

    #[test]
    fn user_message_cell_data() {
        let msg = UserMessage::new("test");
        assert_eq!(msg.category(), CellCategory::UserMessage);
        assert_eq!(msg.text_content(), "test");
    }

    #[test]
    fn agent_message_new() {
        let msg = AgentMessage::new("hello world");
        assert_eq!(msg.text_content(), "hello world");
    }

    #[test]
    fn agent_message_cell_data() {
        let msg = AgentMessage::new("test");
        assert_eq!(msg.category(), CellCategory::AgentMessage);
        assert_eq!(msg.text_content(), "test");
    }

    #[test]
    fn agent_message_renders_markdown() {
        let msg = AgentMessage::new("**bold**");
        let rendered = msg.render();
        assert!(!rendered.is_empty());
    }

    #[test]
    fn agent_message_render_wrapped() {
        let msg = AgentMessage::new("This is a very long line that should wrap when rendered with a small width");
        let wrapped = msg.render_wrapped(20);
        assert!(wrapped.len() > 1);
    }

    #[test]
    fn agent_message_height_with_wrap() {
        let msg = AgentMessage::new("Short");
        assert_eq!(msg.height(0), 1);
        assert_eq!(msg.height(80), 1);
    }

    #[test]
    fn message_cell_user_constructor() {
        let cell = MessageCell::user("hello");
        match cell {
            MessageCell::User(msg) => assert_eq!(msg.text, "hello"),
            MessageCell::Agent(_) => panic!("Expected User variant"),
        }
    }

    #[test]
    fn message_cell_agent_constructor() {
        let cell = MessageCell::agent("world");
        match cell {
            MessageCell::Agent(msg) => assert_eq!(msg.text_content(), "world"),
            MessageCell::User(_) => panic!("Expected Agent variant"),
        }
    }

    #[test]
    fn user_message_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UserMessage>();
    }

    #[test]
    fn agent_message_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AgentMessage>();
    }
}
