use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Widget};
use tui_core::{FlexRenderable, InputHandler, InputResult, Renderable};
use tui_input::Textarea;

use crate::status::StatusIndicator;
use crate::view::{ApprovalRequest, BottomPaneView};

const PROMPT_WIDTH: u16 = 2;
const PROMPT: &str = "> ";

/// Wrapper for rendering the composer with its border and prompt
struct ComposerView<'a> {
    composer: &'a Textarea,
    waiting: bool,
}

impl Renderable for ComposerView<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let title_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);

        let title = if self.waiting {
            Span::styled(" Message (waiting...) ", title_style)
        } else {
            Span::styled(" Message ", title_style)
        };

        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title);

        let inner = block.inner(area);
        block.render(area, buf);
        Clear.render(inner, buf);

        // Pre-calculate textarea rect with inset for prompt
        let textarea_rect = Rect {
            x: inner.x + PROMPT_WIDTH,
            y: inner.y,
            width: inner.width.saturating_sub(PROMPT_WIDTH),
            height: inner.height,
        };

        // Render prompt in the margin (left of textarea)
        let prompt_style = Style::default().fg(Color::DarkGray);
        buf.set_string(inner.x, inner.y, PROMPT, prompt_style);

        // Render textarea in its pre-calculated spot
        self.composer.render(textarea_rect, buf);
    }

    fn height(&self, _width: u16) -> u16 {
        self.composer.line_count() as u16 + 2 // +2 for borders
    }
}

pub struct BottomPane {
    composer: Textarea,
    view_stack: Vec<Box<dyn BottomPaneView>>,
    status: Option<StatusIndicator>,
}

impl Default for BottomPane {
    fn default() -> Self {
        Self::new()
    }
}

impl BottomPane {
    pub fn new() -> Self {
        Self {
            composer: Textarea::new(),
            view_stack: Vec::new(),
            status: None,
        }
    }

    pub fn push_view(&mut self, view: Box<dyn BottomPaneView>) {
        if let Some(status) = &mut self.status {
            status.pause();
        }
        self.view_stack.push(view);
    }

    pub fn pop_view(&mut self) -> Option<Box<dyn BottomPaneView>> {
        let view = self.view_stack.pop();
        if self.view_stack.is_empty() {
            if let Some(status) = &mut self.status {
                status.resume();
            }
        }
        view
    }

    pub fn active_view(&self) -> Option<&dyn BottomPaneView> {
        self.view_stack.last().map(|v| v.as_ref())
    }

    pub fn active_view_mut(&mut self) -> Option<&mut Box<dyn BottomPaneView>> {
        self.view_stack.last_mut()
    }

    pub fn set_status(&mut self, status: StatusIndicator) {
        self.status = Some(status);
    }

    pub fn clear_status(&mut self) -> Option<StatusIndicator> {
        self.status.take()
    }

    pub fn status(&self) -> Option<&StatusIndicator> {
        self.status.as_ref()
    }

    pub fn status_mut(&mut self) -> Option<&mut StatusIndicator> {
        self.status.as_mut()
    }

    pub fn composer(&self) -> &Textarea {
        &self.composer
    }

    pub fn composer_mut(&mut self) -> &mut Textarea {
        &mut self.composer
    }

    pub fn try_consume_approval(&mut self, request: &ApprovalRequest) -> bool {
        if let Some(view) = self.active_view_mut() {
            return view.try_consume_approval_request(request);
        }
        false
    }

    pub fn has_modal(&self) -> bool {
        !self.view_stack.is_empty()
    }
}

impl InputHandler for BottomPane {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        if let Some(view) = self.active_view_mut() {
            let result = view.handle_key_event(key);
            if view.is_complete() {
                self.pop_view();
            }
            return result;
        }
        self.composer.handle_key(key)
    }
}

impl BottomPane {
    fn build_flex(&self) -> FlexRenderable<'_> {
        let waiting = self.status.as_ref().map(|s| !s.is_complete()).unwrap_or(false);

        let mut flex = FlexRenderable::vertical();

        if let Some(status) = &self.status {
            flex.push(0, status);
            flex.push_spacer(1);
        }

        flex.push_owned(
            0,
            ComposerView {
                composer: &self.composer,
                waiting,
            },
        );

        flex
    }
}

impl Renderable for BottomPane {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Modal takes over entire area
        if let Some(view) = self.active_view() {
            view.render(area, buf);
            return;
        }

        self.build_flex().render(area, buf);
    }

    fn height(&self, width: u16) -> u16 {
        if self.has_modal() {
            if let Some(view) = self.active_view() {
                return view.height(width);
            }
        }

        self.build_flex().height(width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bottom_pane() {
        let pane = BottomPane::new();
        assert!(!pane.has_modal());
        assert!(pane.status().is_none());
    }

    #[test]
    fn test_status_lifecycle() {
        let mut pane = BottomPane::new();
        pane.set_status(StatusIndicator::new("test"));
        assert!(pane.status().is_some());
        pane.clear_status();
        assert!(pane.status().is_none());
    }
}
