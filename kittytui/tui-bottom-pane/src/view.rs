use crossterm::event::KeyEvent;
use tui_core::{CancellationEvent, InputResult, Renderable};

pub struct ApprovalRequest {
    pub title: String,
    pub command: String,
    pub context: Option<String>,
}

impl ApprovalRequest {
    pub fn new(title: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            command: command.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

pub trait BottomPaneView: Renderable + Send {
    fn handle_key_event(&mut self, key: KeyEvent) -> InputResult;

    fn is_complete(&self) -> bool {
        false
    }

    fn on_cancel(&mut self) -> CancellationEvent {
        CancellationEvent::NotHandled
    }

    fn handle_paste(&mut self, _text: String) -> bool {
        false
    }

    fn try_consume_approval_request(&mut self, _request: &ApprovalRequest) -> bool {
        false
    }
}
