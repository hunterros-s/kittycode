use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Widget};
use tui_core::{FlexRenderable, InputHandler, InputResult, Renderable};
use tui_input::Textarea;

use crate::footer::{Footer, FooterMode, FooterProps};
use crate::popup::{ActivePopup, CommandPopup};
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
    active_popup: ActivePopup,
    footer_props: FooterProps,
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
            active_popup: ActivePopup::None,
            footer_props: FooterProps::new(),
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

    pub fn footer_props(&self) -> &FooterProps {
        &self.footer_props
    }

    pub fn footer_props_mut(&mut self) -> &mut FooterProps {
        &mut self.footer_props
    }

    pub fn set_context_info(&mut self, text: impl Into<String>) {
        self.footer_props.custom_left = Some(text.into());
    }

    pub fn set_model_info(&mut self, text: impl Into<String>) {
        self.footer_props.custom_right = Some(text.into());
    }

    fn effective_footer_mode(&self) -> FooterMode {
        // Any content dismisses overlay and shows context-only
        if !self.composer.content().is_empty() {
            return FooterMode::ContextOnly;
        }
        self.footer_props.mode
    }

    fn sync_popups(&mut self) {
        let text = self.composer.content();

        // Reset overlay mode when typing starts
        if !text.is_empty() && self.footer_props.mode == FooterMode::ShortcutOverlay {
            self.footer_props.mode = FooterMode::ShortcutSummary;
        }

        let should_show = text.starts_with('/')
            && !text.contains('\n')
            && self.cursor_in_command_token();

        if should_show {
            let filter = self.extract_command_filter();
            match &mut self.active_popup {
                ActivePopup::Command(popup) => popup.update_filter(&filter),
                ActivePopup::None => {
                    self.active_popup = ActivePopup::Command(CommandPopup::new(&filter));
                }
            }
        } else {
            self.active_popup = ActivePopup::None;
        }
    }

    fn cursor_in_command_token(&self) -> bool {
        let text = self.composer.content();
        let (row, _) = self.composer.cursor();

        if row != 0 {
            return false;
        }

        let first_line = text.lines().next().unwrap_or("");
        if !first_line.starts_with('/') {
            return false;
        }

        let space_pos = first_line.find(' ').unwrap_or(first_line.len());
        let (_, col) = self.composer.cursor();
        col <= space_pos
    }

    fn extract_command_filter(&self) -> String {
        let text = self.composer.content();
        let first_line = text.lines().next().unwrap_or("");

        if !first_line.starts_with('/') {
            return String::new();
        }

        let cmd_part = first_line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('/');

        cmd_part.to_string()
    }

    fn handle_with_command_popup(&mut self, key: KeyEvent) -> InputResult {
        match key.code {
            KeyCode::Up => {
                if let ActivePopup::Command(popup) = &mut self.active_popup {
                    popup.move_selection(-1);
                }
                InputResult::Consumed
            }
            KeyCode::Down => {
                if let ActivePopup::Command(popup) = &mut self.active_popup {
                    popup.move_selection(1);
                }
                InputResult::Consumed
            }
            KeyCode::Enter => {
                if let ActivePopup::Command(popup) = &self.active_popup {
                    if let Some(cmd) = popup.selected_command() {
                        let command_text = format!("/{}", cmd.name);
                        self.composer.clear();
                        self.active_popup = ActivePopup::None;
                        return InputResult::Submit(command_text);
                    }
                }
                self.active_popup = ActivePopup::None;
                InputResult::Consumed
            }
            KeyCode::Esc => {
                self.active_popup = ActivePopup::None;
                InputResult::Consumed
            }
            KeyCode::Tab => {
                if let ActivePopup::Command(popup) = &self.active_popup {
                    if let Some(cmd) = popup.selected_command() {
                        self.composer.clear();
                        self.composer.set_text(&format!("/{} ", cmd.name));
                        self.active_popup = ActivePopup::None;
                    }
                }
                InputResult::Consumed
            }
            _ => {
                let result = self.composer.handle_key(key);
                self.sync_popups();
                result
            }
        }
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
        // Modal takes precedence
        if let Some(view) = self.active_view_mut() {
            let result = view.handle_key_event(key);
            if view.is_complete() {
                self.pop_view();
            }
            return result;
        }

        // Then popup
        if matches!(self.active_popup, ActivePopup::Command(_)) {
            return self.handle_with_command_popup(key);
        }

        // Handle ? toggle when composer is empty
        if key.code == KeyCode::Char('?') && self.composer.content().is_empty() {
            self.footer_props.toggle_shortcuts();
            return InputResult::Consumed;
        }

        // Then textarea
        let result = self.composer.handle_key(key);
        self.sync_popups();
        result
    }
}

impl BottomPane {
    fn build_flex(&self) -> FlexRenderable<'_> {
        let waiting = self
            .status
            .as_ref()
            .map(|s| !s.is_complete())
            .unwrap_or(false);

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

        // Add popup or footer below composer
        match &self.active_popup {
            ActivePopup::Command(popup) => {
                flex.push(0, popup);
            }
            ActivePopup::None => {
                let effective_mode = self.effective_footer_mode();
                flex.push_owned(0, Footer::new(&self.footer_props, effective_mode));
            }
        }

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
