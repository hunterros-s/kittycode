use std::time::Duration;

use anyhow::Result;
use crossterm::event::Event;
use crossterm::terminal;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tokio::task::JoinHandle;
use tui_cells::Cell;
use tui_core::{InputHandler, InputResult};
use tui_input::Textarea;
use tui_render::{wrap_lines, WrapOptions};
use tui_terminal::{Frame, Tui};

pub mod protocol;

use protocol::{ChatMessage, OpenAIClient};

pub struct App {
    chat_history: Vec<ChatMessage>,
    input: Textarea,
    client: OpenAIClient,
    pending_response: Option<JoinHandle<Result<String>>>,
    should_exit: bool,
}

impl App {
    pub fn new(client: OpenAIClient) -> Self {
        Self {
            chat_history: Vec::new(),
            input: Textarea::new(),
            client,
            pending_response: None,
            should_exit: false,
        }
    }

    pub async fn run(mut self, mut tui: Tui) -> Result<()> {
        loop {
            // Set viewport height for input + status line (grows and shrinks)
            let needed_height = self.input.line_count() as u16 + 2; // +1 border, +1 status
            tui.set_height(needed_height)?;

            tui.draw(|frame| self.render(frame))?;

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
                                let cell = Cell::agent(&response);
                                let lines = self.cell_to_lines(&cell);
                                tui.insert_history(lines)?;
                            }
                            Ok(Err(e)) => {
                                let cell = Cell::agent(format!("**Error:** {}", e));
                                let lines = self.cell_to_lines(&cell);
                                tui.insert_history(lines)?;
                            }
                            Err(e) => {
                                let cell = Cell::agent(format!("**Error:** {}", e));
                                let lines = self.cell_to_lines(&cell);
                                tui.insert_history(lines)?;
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
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

        // Dynamic input height: line count + 1 for border
        let input_height = self.input.line_count() as u16 + 1;

        let [status_area, input_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(input_height)]).areas(area);

        if self.pending_response.is_some() {
            let status = Paragraph::new(Line::styled(
                "Thinking...",
                Style::default().fg(Color::Yellow),
            ));
            frame.render_widget(status, status_area);
        }

        self.render_input(frame, input_area);
    }

    fn render_input(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let title = if self.pending_response.is_some() {
            " Message (waiting...) "
        } else {
            " Message "
        };

        let block = Block::default().borders(Borders::TOP).title(title);

        let inner = block.inner(area);
        frame.render_widget(block, area);
        // Clear the input area before rendering to remove leftover characters
        frame.render_widget(Clear, inner);
        frame.render_widget(&self.input, inner);
    }

    fn handle_event(&mut self, tui: &mut Tui, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match self.input.handle_key(key) {
                InputResult::Submit(text) => {
                    if !text.trim().is_empty() && self.pending_response.is_none() {
                        // Shrink viewport FIRST (textarea is now empty after submit)
                        let new_height = self.input.line_count() as u16 + 2;
                        tui.set_height(new_height)?;

                        // Now insert history with correct viewport dimensions
                        let cell = Cell::user(&text);
                        let lines = self.cell_to_lines(&cell);
                        tui.insert_history(lines)?;

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
        Ok(())
    }

    fn cell_to_lines(&self, cell: &Cell) -> Vec<Line<'static>> {
        let width = terminal::size().map(|(w, _)| w).unwrap_or(80) as usize;
        let mut lines = Vec::new();

        match cell {
            Cell::User(msg) => {
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
            Cell::Agent(msg) => {
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
