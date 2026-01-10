mod flex;

use std::borrow::Cow;
use std::fmt::Debug;

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub use flex::{FlexItem, FlexRenderable, RenderableItem, Spacer};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationEvent {
    Handled,
    NotHandled,
}

pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
}

pub trait Renderable {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn height(&self, width: u16) -> u16;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_result_consumed() {
        let result = InputResult::Consumed;
        assert_eq!(result, InputResult::Consumed);
    }

    #[test]
    fn input_result_submit() {
        let result = InputResult::Submit("hello".to_string());
        assert_eq!(result, InputResult::Submit("hello".to_string()));
        assert_ne!(result, InputResult::Submit("world".to_string()));
    }

    #[test]
    fn input_result_exit() {
        let result = InputResult::Exit;
        assert_eq!(result, InputResult::Exit);
    }

    #[test]
    fn input_result_ignored() {
        let result = InputResult::Ignored;
        assert_eq!(result, InputResult::Ignored);
    }

    #[test]
    fn cell_category_variants() {
        assert_eq!(CellCategory::UserMessage, CellCategory::UserMessage);
        assert_eq!(CellCategory::AgentMessage, CellCategory::AgentMessage);
        assert_ne!(CellCategory::UserMessage, CellCategory::AgentMessage);
    }

    #[derive(Debug)]
    struct TestCell {
        text: String,
    }

    impl CellData for TestCell {
        fn category(&self) -> CellCategory {
            CellCategory::UserMessage
        }

        fn text_content(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.text)
        }
    }

    #[test]
    fn cell_data_trait_object_safety() {
        let cell = TestCell {
            text: "test".to_string(),
        };
        let boxed: Box<dyn CellData> = Box::new(cell);
        assert_eq!(boxed.category(), CellCategory::UserMessage);
        assert_eq!(boxed.text_content(), "test");
    }

    #[test]
    fn cell_data_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestCell>();
    }
}
