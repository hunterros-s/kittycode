use std::time::Duration;

use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tokio::task::JoinHandle;
use tui_cells::Cell;
use tui_core::{InputHandler, InputResult};
use tui_input::Textarea;
use tui_terminal::Terminal;

pub mod protocol;

use protocol::{ChatMessage, OpenAIClient};

pub struct App {
    history: Vec<Cell>,
    chat_history: Vec<ChatMessage>,
    input: Textarea,
    client: OpenAIClient,
    pending_response: Option<JoinHandle<Result<String>>>,
    should_exit: bool,
}

impl App {
    pub fn new(client: OpenAIClient) -> Self {
        Self {
            history: Vec::new(),
            chat_history: Vec::new(),
            input: Textarea::new(),
            client,
            pending_response: None,
            should_exit: false,
        }
    }

    pub async fn run(mut self, terminal: &mut Terminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if self.should_exit {
                break;
            }

            if let Some(handle) = self.pending_response.as_mut() {
                tokio::select! {
                    result = handle => {
                        self.pending_response = None;
                        match result {
                            Ok(Ok(response)) => {
                                self.chat_history.push(ChatMessage::assistant(&response));
                                self.history.push(Cell::agent(response));
                            }
                            Ok(Err(e)) => {
                                self.history.push(Cell::agent(format!("**Error:** {}", e)));
                            }
                            Err(e) => {
                                self.history.push(Cell::agent(format!("**Error:** {}", e)));
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        if let Some(event) = tui_terminal::poll_event(Duration::from_millis(1))? {
                            self.handle_event(event);
                        }
                    }
                }
            } else if let Some(event) = tui_terminal::poll_event(Duration::from_millis(100))? {
                self.handle_event(event);
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let [history_area, input_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(area);

        self.render_history(frame, history_area);
        self.render_input(frame, input_area);
    }

    fn render_history(&self, frame: &mut ratatui::Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for cell in &self.history {
            match cell {
                Cell::User(msg) => {
                    lines.push(Line::from(vec![
                        Span::styled("You: ", Style::default().fg(Color::Cyan)),
                        Span::raw(&msg.text),
                    ]));
                    lines.push(Line::default());
                }
                Cell::Agent(msg) => {
                    lines.push(Line::styled(
                        "Agent:",
                        Style::default().fg(Color::Green),
                    ));
                    for line in msg.render() {
                        lines.push(line.clone());
                    }
                    lines.push(Line::default());
                }
            }
        }

        if self.pending_response.is_some() {
            lines.push(Line::styled(
                "Thinking...",
                Style::default().fg(Color::Yellow),
            ));
        }

        let total_lines = lines.len() as u16;
        let scroll_offset = total_lines.saturating_sub(area.height);

        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset, 0));

        frame.render_widget(para, area);
    }

    fn render_input(&self, frame: &mut ratatui::Frame, area: Rect) {
        let title = if self.pending_response.is_some() {
            " Message (waiting...) "
        } else {
            " Message "
        };

        let block = Block::default().borders(Borders::TOP).title(title);

        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(&self.input, inner);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match self.input.handle_key(key) {
                InputResult::Submit(text) => {
                    if !text.trim().is_empty() && self.pending_response.is_none() {
                        self.history.push(Cell::user(&text));
                        self.chat_history.push(ChatMessage::user(&text));

                        let client = self.client.clone();
                        let messages = self.chat_history.clone();

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
    }
}
