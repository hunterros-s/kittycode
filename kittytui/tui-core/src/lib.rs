use std::borrow::Cow;
use std::fmt::Debug;

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellCategory {
    UserMessage,
    AgentMessage,
}

pub trait CellData: Send + Sync + Debug {
    fn category(&self) -> CellCategory;
    fn text_content(&self) -> Cow<'_, str>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    Consumed,
    Submit(String),
    Exit,
    Ignored,
}

pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
}

pub trait Renderable {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn height(&self, width: u16) -> u16;
}
