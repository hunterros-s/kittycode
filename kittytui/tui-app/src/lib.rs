use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use tokio::task::JoinHandle;
use tui_bottom_pane::{ApprovalView, BottomPane, StatusIndicator};
use tui_cells::{Cell, ErrorCell, MessageCell};
use tui_core::{InputHandler, InputResult, Renderable};
use tui_render::{wrap_lines, WrapOptions};
use tui_terminal::{Frame, Tui};

pub mod protocol;

use protocol::{ChatMessage, OpenAIClient};

pub struct App {
    chat_history: Vec<ChatMessage>,
    bottom_pane: BottomPane,
    client: OpenAIClient,
    pending_response: Option<JoinHandle<Result<String>>>,
    should_exit: bool,
}

impl App {
    pub fn new(client: OpenAIClient) -> Self {
        Self {
            chat_history: Vec::new(),
            bottom_pane: BottomPane::new(),
            client,
            pending_response: None,
            should_exit: false,
        }
    }

    fn has_active_status(&self) -> bool {
        self.bottom_pane
            .status()
            .map(|s| !s.is_complete())
            .unwrap_or(false)
    }

    pub async fn run(mut self, mut tui: Tui) -> Result<()> {
        let (cols, _) = terminal::size()?;

        loop {
            if let Some(status) = self.bottom_pane.status() {
                if status.is_complete() {
                    let status = self.bottom_pane.clear_status().unwrap();
                    let line = status.render_line();
                    tui.queue_history(vec![line]);
                }
            }

            let needed_height = self.bottom_pane.height(cols);

            tui.draw(needed_height, |frame| self.render(frame))?;

            if self.should_exit {
                break;
            }

            let poll_interval = if self.has_active_status() {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(50)
            };

            if let Some(handle) = self.pending_response.as_mut() {
                tokio::select! {
                    result = handle => {
                        self.pending_response = None;

                        match result {
                            Ok(Ok(response)) => {
                                if let Some(status) = self.bottom_pane.status_mut() {
                                    status.complete(true);
                                }
                                if let Some(status) = self.bottom_pane.clear_status() {
                                    tui.queue_history(vec![status.render_line(), Line::default()]);
                                }

                                self.chat_history.push(ChatMessage::assistant(&response));
                                let cell = MessageCell::agent(&response);
                                let lines = self.cell_to_lines(&cell);
                                tui.queue_history(lines);
                            }
                            Ok(Err(e)) => {
                                if let Some(status) = self.bottom_pane.status_mut() {
                                    status.complete(false);
                                }
                                if let Some(status) = self.bottom_pane.clear_status() {
                                    tui.queue_history(vec![status.render_line(), Line::default()]);
                                }

                                let error = ErrorCell::with_details("API request failed", e.to_string());
                                tui.queue_history(error.render_lines(cols));
                            }
                            Err(e) => {
                                if let Some(status) = self.bottom_pane.status_mut() {
                                    status.complete(false);
                                }
                                if let Some(status) = self.bottom_pane.clear_status() {
                                    tui.queue_history(vec![status.render_line(), Line::default()]);
                                }

                                let error = ErrorCell::with_details("Task failed", e.to_string());
                                tui.queue_history(error.render_lines(cols));
                            }
                        }
                    }
                    _ = tokio::time::sleep(poll_interval) => {
                        if let Some(event) = tui_terminal::poll_event(Duration::from_millis(1))? {
                            self.handle_event(&mut tui, event)?;
                        }
                    }
                }
            } else if let Some(event) = tui_terminal::poll_event(Duration::from_millis(100))? {
                self.handle_event(&mut tui, event)?;
            }
        }

        tui.restore()?;
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        self.bottom_pane.render(area, frame.buffer_mut());
    }

    fn handle_event(&mut self, tui: &mut Tui, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            // Ctrl+T = test approval view
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
                let approval = ApprovalView::new("Run command?", "echo 'hello world'");
                self.bottom_pane.push_view(Box::new(approval));
                return Ok(());
            }

            match self.bottom_pane.handle_key(key) {
                InputResult::Submit(text) => {
                    if !text.trim().is_empty() && self.pending_response.is_none() {
                        let cell = MessageCell::user(&text);
                        let lines = self.cell_to_lines(&cell);
                        tui.queue_history(lines);

                        self.chat_history.push(ChatMessage::user(&text));

                        let client = self.client.clone();
                        let messages = self.chat_history.clone();

                        self.bottom_pane.set_status(StatusIndicator::new("chat completion"));
                        self.pending_response = Some(tokio::spawn(async move {
                            client.chat(messages).await
                        }));
                    }
                }
                InputResult::Exit => {
                    self.should_exit = true;
                }
                InputResult::Consumed | InputResult::Ignored => {}
            }
        }
        Ok(())
    }

    fn cell_to_lines(&self, cell: &MessageCell) -> Vec<Line<'static>> {
        let width = terminal::size().map(|(w, _)| w).unwrap_or(80) as usize;
        let mut lines = Vec::new();

        match cell {
            MessageCell::User(msg) => {
                lines.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Cyan)),
                    Span::styled("You:", Style::default().fg(Color::Cyan)),
                ]));
                let opts = WrapOptions::new(width)
                    .initial_indent(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                    ]))
                    .subsequent_indent(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                    ]));
                let wrapped = wrap_lines(msg.render(), opts);
                lines.extend(wrapped);
                lines.push(Line::default());
            }
            MessageCell::Agent(msg) => {
                lines.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Green)),
                    Span::styled("Agent:", Style::default().fg(Color::Green)),
                ]));
                let opts = WrapOptions::new(width)
                    .initial_indent(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                    ]))
                    .subsequent_indent(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                    ]));
                let wrapped = wrap_lines(msg.render().to_vec(), opts);
                lines.extend(wrapped);
                lines.push(Line::default());
            }
        }

        lines
    }
}
