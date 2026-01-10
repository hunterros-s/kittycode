use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::Event;
use crossterm::terminal;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tokio::task::JoinHandle;
use tui_cells::{Cell, ErrorCell, MessageCell};
use tui_core::{InputHandler, InputResult};
use tui_input::Textarea;
use tui_render::{wrap_lines, WrapOptions};
use tui_terminal::{Frame, Tui};

pub mod protocol;

use protocol::{ChatMessage, OpenAIClient};

/// Content that can be rendered in the viewport
pub trait ViewportCell: Send {
    fn render_lines(&self, width: u16) -> Vec<Line<'static>>;
    fn height(&self, width: u16) -> u16 {
        self.render_lines(width).len() as u16
    }
    fn is_complete(&self) -> bool;
    fn history_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.render_lines(width)
    }
}

/// Animated spinner cell for showing progress
pub struct SpinnerCell {
    pub label: String,
    pub start_time: Instant,
    pub completed: Option<(bool, Duration)>,
}

impl SpinnerCell {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            start_time: Instant::now(),
            completed: None,
        }
    }

    pub fn complete(&mut self, success: bool) {
        self.completed = Some((success, self.start_time.elapsed()));
    }

    fn spinner_frame(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
        let idx = (self.start_time.elapsed().as_millis() / 200) % 4;
        FRAMES[idx as usize]
    }
}

impl ViewportCell for SpinnerCell {
    fn render_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let line = if let Some((success, duration)) = self.completed {
            let (symbol, color) = if success {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(symbol, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw(self.label.clone()),
                Span::styled(
                    format!(" ({:.1}s)", duration.as_secs_f64()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(self.spinner_frame(), Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::raw(self.label.clone()),
            ])
        };
        vec![line]
    }

    fn is_complete(&self) -> bool {
        self.completed.is_some()
    }
}

pub struct App {
    chat_history: Vec<ChatMessage>,
    input: Textarea,
    client: OpenAIClient,
    pending_response: Option<JoinHandle<Result<String>>>,
    active_spinner: Option<SpinnerCell>,
    should_exit: bool,
}

impl App {
    pub fn new(client: OpenAIClient) -> Self {
        Self {
            chat_history: Vec::new(),
            input: Textarea::new(),
            client,
            pending_response: None,
            active_spinner: None,
            should_exit: false,
        }
    }

    fn has_active_spinner(&self) -> bool {
        self.active_spinner
            .as_ref()
            .map(|s| !s.is_complete())
            .unwrap_or(false)
    }

    pub async fn run(mut self, mut tui: Tui) -> Result<()> {
        let (cols, _) = terminal::size()?;

        loop {
            // Flush completed spinner to history (queue for next draw)
            if let Some(ref spinner) = self.active_spinner {
                if spinner.is_complete() {
                    let spinner = self.active_spinner.take().unwrap();
                    let lines = spinner.history_lines(cols);
                    tui.queue_history(lines);
                }
            }

            // Calculate viewport height: spinner + input + border
            let spinner_height = self.active_spinner.as_ref().map(|s| s.height(cols)).unwrap_or(0);
            let input_height = self.input.line_count() as u16 + 1; // +1 border
            let needed_height = spinner_height + input_height;

            // Atomic: resize + flush history + draw
            tui.draw(needed_height, |frame| self.render(frame))?;

            if self.should_exit {
                break;
            }

            // Compute poll interval before borrowing pending_response
            let poll_interval = if self.has_active_spinner() {
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
                                // Flush spinner to history first (before agent message)
                                if let Some(spinner) = &mut self.active_spinner {
                                    spinner.complete(true);
                                }
                                if let Some(spinner) = self.active_spinner.take() {
                                    tui.queue_history(spinner.history_lines(cols));
                                }

                                // Show agent response
                                self.chat_history.push(ChatMessage::assistant(&response));
                                let cell = MessageCell::agent(&response);
                                let lines = self.cell_to_lines(&cell);
                                tui.queue_history(lines);
                            }
                            Ok(Err(e)) => {
                                // Flush spinner to history first
                                if let Some(spinner) = &mut self.active_spinner {
                                    spinner.complete(false);
                                }
                                if let Some(spinner) = self.active_spinner.take() {
                                    tui.queue_history(spinner.history_lines(cols));
                                }

                                // Show error cell
                                let error = ErrorCell::with_details("API request failed", e.to_string());
                                tui.queue_history(error.render_lines(cols));
                            }
                            Err(e) => {
                                // Flush spinner to history first
                                if let Some(spinner) = &mut self.active_spinner {
                                    spinner.complete(false);
                                }
                                if let Some(spinner) = self.active_spinner.take() {
                                    tui.queue_history(spinner.history_lines(cols));
                                }

                                // Show error cell
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

        // Layout: [spinner_area (if active)] [input_area]
        let spinner_height = self.active_spinner.as_ref().map(|s| s.height(area.width)).unwrap_or(0);
        let input_height = self.input.line_count() as u16 + 1;

        let [spinner_area, input_area] = Layout::vertical([
            Constraint::Length(spinner_height),
            Constraint::Length(input_height),
        ])
        .areas(area);

        // Render spinner if active
        if let Some(spinner) = &self.active_spinner {
            let lines = spinner.render_lines(area.width);
            frame.render_widget(Paragraph::new(lines), spinner_area);
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
                        // Queue user message for history (will be inserted on next draw)
                        let cell = MessageCell::user(&text);
                        let lines = self.cell_to_lines(&cell);
                        tui.queue_history(lines);

                        self.chat_history.push(ChatMessage::user(&text));

                        let client = self.client.clone();
                        let messages = self.chat_history.clone();

                        // Create spinner for this request
                        self.active_spinner = Some(SpinnerCell::new("chat completion"));
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
