use std::io;
use std::time::Duration;

use crossterm::event::Event;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tui_cells::Cell;
use tui_core::{InputHandler, InputResult};
use tui_input::Textarea;
use tui_terminal::Terminal;

pub struct App {
    history: Vec<Cell>,
    input: Textarea,
    should_exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            input: Textarea::new(),
            should_exit: false,
        }
    }

    pub fn run(mut self, terminal: &mut Terminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(event) = tui_terminal::poll_event(Duration::from_millis(100))? {
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

        let total_lines = lines.len() as u16;
        let scroll_offset = total_lines.saturating_sub(area.height);

        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset, 0));

        frame.render_widget(para, area);
    }

    fn render_input(&self, frame: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::TOP)
            .title(" Message ");

        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(&self.input, inner);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match self.input.handle_key(key) {
                InputResult::Submit(text) => {
                    if !text.trim().is_empty() {
                        self.history.push(Cell::user(&text));
                        let response = generate_mock_response(&text);
                        self.history.push(Cell::agent(response));
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

fn generate_mock_response(input: &str) -> String {
    format!(
        "I received your message: **\"{}\"**\n\n\
         This is a *mock response* from the agent. \
         In the full implementation, this would come from an actual AI backend.\n\n\
         Some features:\n\
         - Markdown rendering\n\
         - `Code` formatting\n\
         - **Bold** and *italic* text",
        input
    )
}
