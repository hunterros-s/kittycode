# Skinny Implementation Guide

A minimal, testable TUI implementation. Follow this guide to build a working conversational interface with the smallest possible scope.

## What You'll Build

A terminal application that:
- Displays a text input at the bottom
- Shows conversation history above
- Accepts user messages (Enter to submit)
- Displays agent responses with markdown rendering
- Handles Ctrl+C to exit cleanly

## What You Won't Build (Yet)

- Streaming responses (responses appear all at once)
- Tool calls, approvals, exec cells
- File attachments, slash commands
- Modals, popups, overlays
- Onboarding, sessions, persistence
- Syntax highlighting, diff rendering
- Notifications, external editors

---

## Implementation Phases

```
Phase 1: Foundation     →  Terminal works, can exit cleanly
Phase 2: Display        →  Can render static text with markdown
Phase 3: Input          →  Can type and edit text
Phase 4: Integration    →  Full loop: type → submit → see response
```

---

## Phase 1: Foundation

### tui-core (minimal)

**Files to create:**
```
tui-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── traits.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-core"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
```

**src/lib.rs:**
```rust
mod traits;
pub use traits::*;
```

**src/traits.rs:**
```rust
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use std::borrow::Cow;

/// Category of conversation cell
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellCategory {
    UserMessage,
    AgentMessage,
}

/// Data for a conversation history cell
pub trait CellData: Send + Sync + std::fmt::Debug {
    fn category(&self) -> CellCategory;
    fn text_content(&self) -> Cow<'_, str>;
}

/// Result of handling keyboard input
pub enum InputResult {
    Consumed,
    Submit(String),
    Exit,
    Ignored,
}

/// Component that handles keyboard input
pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult;
}

/// Component that can be rendered
pub trait Renderable {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn height(&self, width: u16) -> u16;
}
```

**Test:** `cargo build -p tui-core`

---

### tui-terminal (minimal)

**Files to create:**
```
tui-terminal/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── terminal.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-terminal"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
```

**src/lib.rs:**
```rust
mod terminal;
pub use terminal::*;
```

**src/terminal.rs:**
```rust
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout, Stdout};
use std::time::Duration;

pub type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

/// Initialize terminal for TUI
pub fn init() -> io::Result<Terminal> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = restore();
        original_hook(panic);
    }));

    let backend = CrosstermBackend::new(stdout());
    Terminal::new(backend)
}

/// Restore terminal to normal state
pub fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Poll for next event with timeout
pub fn poll_event(timeout: Duration) -> io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Read next event (blocking)
pub fn read_event() -> io::Result<Event> {
    event::read()
}
```

**Test:** Create a minimal main.rs:
```rust
fn main() -> std::io::Result<()> {
    let mut terminal = tui_terminal::init()?;
    terminal.draw(|f| {
        f.render_widget(
            ratatui::widgets::Paragraph::new("Press 'q' to quit"),
            f.area(),
        );
    })?;

    loop {
        if let Some(crossterm::event::Event::Key(key)) = tui_terminal::poll_event(std::time::Duration::from_millis(100))? {
            if key.code == crossterm::event::KeyCode::Char('q') {
                break;
            }
        }
    }

    tui_terminal::restore()?;
    Ok(())
}
```

**Verify:** Run, see message, press 'q', terminal restored cleanly.

---

## Phase 2: Display

### tui-render (minimal)

**Files to create:**
```
tui-render/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── markdown.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-render"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
pulldown-cmark = "0.11"
```

**src/lib.rs:**
```rust
mod markdown;
pub use markdown::render_markdown;
```

**src/markdown.rs:**
```rust
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

/// Render markdown to styled lines
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let style = match tag {
                    Tag::Heading { level, .. } => {
                        Style::default().bold().fg(match level {
                            pulldown_cmark::HeadingLevel::H1 => Color::Cyan,
                            pulldown_cmark::HeadingLevel::H2 => Color::Green,
                            _ => Color::Yellow,
                        })
                    }
                    Tag::Strong => Style::default().bold(),
                    Tag::Emphasis => Style::default().italic(),
                    Tag::CodeBlock(_) => Style::default().fg(Color::Gray),
                    _ => *style_stack.last().unwrap_or(&Style::default()),
                };
                style_stack.push(style);
            }
            Event::End(tag) => {
                style_stack.pop();
                match tag {
                    TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::CodeBlock => {
                        if !current_spans.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_spans)));
                        }
                        if matches!(tag, TagEnd::Paragraph) {
                            lines.push(Line::default()); // blank line after paragraph
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                current_spans.push(Span::styled(text.to_string(), style));
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Magenta),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}
```

**Test:**
```rust
#[test]
fn test_render_heading() {
    let lines = render_markdown("# Hello");
    assert!(!lines.is_empty());
}

#[test]
fn test_render_bold() {
    let lines = render_markdown("**bold**");
    assert!(!lines.is_empty());
}
```

---

### tui-cells (minimal)

**Files to create:**
```
tui-cells/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── user.rs
    └── agent.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-cells"
version = "0.1.0"
edition = "2021"

[dependencies]
tui-core = { path = "../tui-core" }
tui-render = { path = "../tui-render" }
ratatui = "0.28"
```

**src/lib.rs:**
```rust
mod user;
mod agent;

pub use user::UserMessage;
pub use agent::AgentMessage;
```

**src/user.rs:**
```rust
use std::borrow::Cow;
use tui_core::{CellCategory, CellData};
use ratatui::prelude::*;

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
}

impl UserMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn render(&self, width: u16) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("You: ", Style::default().bold().fg(Color::Blue)),
                Span::raw(self.text.clone()),
            ])
        ]
    }

    pub fn height(&self, _width: u16) -> u16 {
        1
    }
}

impl CellData for UserMessage {
    fn category(&self) -> CellCategory {
        CellCategory::UserMessage
    }

    fn text_content(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
}
```

**src/agent.rs:**
```rust
use std::borrow::Cow;
use tui_core::{CellCategory, CellData};
use tui_render::render_markdown;
use ratatui::prelude::*;

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub text: String,
    rendered: Vec<Line<'static>>,
}

impl AgentMessage {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let rendered = render_markdown(&text);
        Self { text, rendered }
    }

    pub fn render(&self, _width: u16) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(Span::styled(
            "Assistant:",
            Style::default().bold().fg(Color::Green),
        ))];
        lines.extend(self.rendered.clone());
        lines
    }

    pub fn height(&self, _width: u16) -> u16 {
        (1 + self.rendered.len()) as u16
    }
}

impl CellData for AgentMessage {
    fn category(&self) -> CellCategory {
        CellCategory::AgentMessage
    }

    fn text_content(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
}
```

**Test:** `cargo build -p tui-cells`

---

## Phase 3: Input

### tui-input (minimal)

**Files to create:**
```
tui-input/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── textarea.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-input"
version = "0.1.0"
edition = "2021"

[dependencies]
tui-core = { path = "../tui-core" }
ratatui = "0.28"
crossterm = "0.28"
```

**src/lib.rs:**
```rust
mod textarea;
pub use textarea::Textarea;
```

**src/textarea.rs:**
```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use tui_core::{InputHandler, InputResult, Renderable};

#[derive(Debug, Default)]
pub struct Textarea {
    content: String,
    cursor: usize,
}

impl Textarea {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl InputHandler for Textarea {
    fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        match (key.code, key.modifiers) {
            // Exit
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => InputResult::Exit,

            // Submit
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.content.is_empty() {
                    InputResult::Consumed
                } else {
                    let text = std::mem::take(&mut self.content);
                    self.cursor = 0;
                    InputResult::Submit(text)
                }
            }

            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.content.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                InputResult::Consumed
            }

            // Backspace
            (KeyCode::Backspace, _) => {
                if self.cursor > 0 {
                    let prev = self.content[..self.cursor]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.cursor -= prev;
                    self.content.remove(self.cursor);
                }
                InputResult::Consumed
            }

            // Delete
            (KeyCode::Delete, _) => {
                if self.cursor < self.content.len() {
                    self.content.remove(self.cursor);
                }
                InputResult::Consumed
            }

            // Cursor movement
            (KeyCode::Left, _) => {
                if self.cursor > 0 {
                    let prev = self.content[..self.cursor]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.cursor -= prev;
                }
                InputResult::Consumed
            }
            (KeyCode::Right, _) => {
                if self.cursor < self.content.len() {
                    let next = self.content[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.cursor += next;
                }
                InputResult::Consumed
            }
            (KeyCode::Home, _) => {
                self.cursor = 0;
                InputResult::Consumed
            }
            (KeyCode::End, _) => {
                self.cursor = self.content.len();
                InputResult::Consumed
            }

            _ => InputResult::Ignored,
        }
    }
}

impl Renderable for Textarea {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Render prompt and content
        let prompt = "> ";
        let display = format!("{}{}", prompt, self.content);

        buf.set_string(area.x, area.y, &display, Style::default());

        // Render cursor
        let cursor_x = area.x + prompt.len() as u16 + self.cursor as u16;
        if cursor_x < area.right() {
            if let Some(cell) = buf.cell_mut((cursor_x, area.y)) {
                cell.set_style(Style::default().bg(Color::White).fg(Color::Black));
            }
        }
    }

    fn height(&self, _width: u16) -> u16 {
        1
    }
}
```

**Test:**
```rust
#[test]
fn test_textarea_input() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut ta = Textarea::new();
    ta.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    ta.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(ta.content(), "hi");
}

#[test]
fn test_textarea_submit() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tui_core::InputResult;

    let mut ta = Textarea::new();
    ta.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    match ta.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)) {
        InputResult::Submit(text) => assert_eq!(text, "x"),
        _ => panic!("Expected Submit"),
    }

    assert!(ta.is_empty());
}
```

---

## Phase 4: Integration

### tui-app (minimal)

**Files to create:**
```
tui-app/
├── Cargo.toml
└── src/
    ├── main.rs
    └── app.rs
```

**Cargo.toml:**
```toml
[package]
name = "tui-app"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "tui"
path = "src/main.rs"

[dependencies]
tui-core = { path = "../tui-core" }
tui-terminal = { path = "../tui-terminal" }
tui-render = { path = "../tui-render" }
tui-cells = { path = "../tui-cells" }
tui-input = { path = "../tui-input" }
ratatui = "0.28"
crossterm = "0.28"
```

**src/main.rs:**
```rust
mod app;

use app::App;
use std::io;

fn main() -> io::Result<()> {
    let mut terminal = tui_terminal::init()?;
    let result = App::new().run(&mut terminal);
    tui_terminal::restore()?;
    result
}
```

**src/app.rs:**
```rust
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;
use tui_cells::{AgentMessage, UserMessage};
use tui_core::{InputHandler, InputResult, Renderable};
use tui_input::Textarea;

enum Cell {
    User(UserMessage),
    Agent(AgentMessage),
}

pub struct App {
    history: Vec<Cell>,
    input: Textarea,
    scroll: u16,
    should_exit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            input: Textarea::new(),
            scroll: 0,
            should_exit: false,
        }
    }

    pub fn run(mut self, terminal: &mut tui_terminal::Terminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|f| self.render(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: history takes all space except bottom 3 lines
        let input_height = 3;
        let history_height = area.height.saturating_sub(input_height);

        let history_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: history_height,
        };

        let input_area = Rect {
            x: area.x,
            y: area.y + history_height,
            width: area.width,
            height: input_height,
        };

        // Render history
        self.render_history(frame, history_area);

        // Render separator
        let separator = "─".repeat(area.width as usize);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(separator)
                .style(Style::default().fg(Color::DarkGray)),
            Rect { y: input_area.y, height: 1, ..input_area },
        );

        // Render input
        self.input.render(
            Rect { y: input_area.y + 1, height: 2, ..input_area },
            frame.buffer_mut(),
        );
    }

    fn render_history(&self, frame: &mut Frame, area: Rect) {
        let mut y = area.y;

        for cell in &self.history {
            let lines = match cell {
                Cell::User(msg) => msg.render(area.width),
                Cell::Agent(msg) => msg.render(area.width),
            };

            for line in lines {
                if y >= area.bottom() {
                    break;
                }
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(line),
                    Rect { x: area.x, y, width: area.width, height: 1 },
                );
                y += 1;
            }

            // Add spacing between messages
            y += 1;
        }

        // Show hint if empty
        if self.history.is_empty() {
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Type a message and press Enter...")
                    .style(Style::default().fg(Color::DarkGray)),
                area,
            );
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Some(Event::Key(key)) = tui_terminal::poll_event(Duration::from_millis(50))? {
            match self.input.handle_key(key) {
                InputResult::Submit(text) => {
                    self.submit_message(text);
                }
                InputResult::Exit => {
                    self.should_exit = true;
                }
                InputResult::Consumed | InputResult::Ignored => {}
            }
        }
        Ok(())
    }

    fn submit_message(&mut self, text: String) {
        // Add user message
        self.history.push(Cell::User(UserMessage::new(&text)));

        // Generate mock response
        let response = self.mock_response(&text);
        self.history.push(Cell::Agent(AgentMessage::new(response)));
    }

    fn mock_response(&self, input: &str) -> String {
        // Simple echo for testing - replace with real protocol
        format!(
            "You said: **{}**\n\nThis is a mock response. \
            In a real implementation, this would come from your backend protocol.",
            input
        )
    }
}
```

---

## Workspace Setup

Create a workspace `Cargo.toml` at the root:

```toml
[workspace]
members = [
    "tui-core",
    "tui-terminal",
    "tui-render",
    "tui-cells",
    "tui-input",
    "tui-app",
]
resolver = "2"
```

---

## Test Checklist

After completing each phase:

### Phase 1
- [ ] `cargo build -p tui-core` succeeds
- [ ] `cargo build -p tui-terminal` succeeds
- [ ] Test app starts and exits cleanly with 'q'

### Phase 2
- [ ] `cargo test -p tui-render` passes
- [ ] `cargo build -p tui-cells` succeeds

### Phase 3
- [ ] `cargo test -p tui-input` passes
- [ ] Textarea handles typing, backspace, cursor movement

### Phase 4
- [ ] `cargo run -p tui-app` starts
- [ ] Can type text in input area
- [ ] Enter submits message to history
- [ ] Agent response appears with markdown formatting
- [ ] Ctrl+C exits cleanly
- [ ] Terminal restored after exit

---

## Next Steps (After Skinny Works)

Priority order for enhancements:

1. **Real Protocol** - Replace `mock_response()` with actual backend
2. **Scrolling** - Add scroll support for long history
3. **Streaming** - Show responses as they arrive
4. **Word Wrap** - Wrap long lines properly
5. **More Cells** - Add error cells, info cells
6. **Approvals** - Add approval flow for tool calls

See the full specifications in `crates/*.md` for complete feature requirements.
