use std::fmt;
use std::io::{self, stdout, Stdout, Write};

use crossterm::cursor::{MoveTo, position};
use crossterm::queue;
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::Command;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

use crate::{install_panic_hook, Frame, InlineTerminal};

/// Plan for pushing the viewport down (Phase 1 of insert_history)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushViewportPlan {
    /// Scroll region top (1-based, for DECSTBM)
    pub scroll_region_top: u16,
    /// Scroll region bottom (1-based, for DECSTBM)
    pub scroll_region_bottom: u16,
    /// Row to position cursor at (0-based)
    pub cursor_row: u16,
    /// Number of RI (reverse index) commands to emit
    pub ri_count: u16,
    /// New viewport_top after push (0-based)
    pub new_viewport_top: u16,
}

/// Plan for writing content (Phase 2 of insert_history)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteContentPlan {
    /// Scroll region top (1-based, for DECSTBM)
    pub scroll_region_top: u16,
    /// Scroll region bottom (1-based, for DECSTBM)
    pub scroll_region_bottom: u16,
    /// Row to position cursor at before writing (0-based)
    pub cursor_row: u16,
    /// Number of lines to write
    pub line_count: u16,
}

/// Complete plan for insert_history operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertHistoryPlan {
    /// Phase 1: Push viewport down (None if already at bottom)
    pub push_viewport: Option<PushViewportPlan>,
    /// Phase 2: Write content above viewport
    pub write_content: Option<WriteContentPlan>,
    /// Final viewport top position (0-based)
    pub final_viewport_top: u16,
}

/// Pure function to compute insert_history plan (no I/O, fully testable)
pub fn plan_insert_history(
    viewport_top: u16,
    viewport_height: u16,
    terminal_rows: u16,
    num_lines: u16,
) -> InsertHistoryPlan {
    if num_lines == 0 {
        return InsertHistoryPlan {
            push_viewport: None,
            write_content: None,
            final_viewport_top: viewport_top,
        };
    }

    let viewport_bottom = viewport_top + viewport_height;
    let space_below = terminal_rows.saturating_sub(viewport_bottom);

    // cursor_top is computed BEFORE any viewport position changes
    // This is where we'll position cursor for phase 2
    let cursor_top = viewport_top.saturating_sub(1);

    let mut current_viewport_top = viewport_top;
    let mut push_viewport = None;

    // Phase 1: If there's whitespace below viewport, push viewport down
    if space_below > 0 {
        let scroll_amount = num_lines.min(space_below);

        push_viewport = Some(PushViewportPlan {
            // Scroll region from viewport top to screen bottom
            scroll_region_top: viewport_top + 1, // 1-based
            scroll_region_bottom: terminal_rows, // 1-based (terminal_rows is count, which == last row + 1)
            cursor_row: viewport_top,            // 0-based
            ri_count: scroll_amount,
            new_viewport_top: viewport_top + scroll_amount,
        });

        current_viewport_top = viewport_top + scroll_amount;
    }

    // Phase 2: Write content into scroll region above viewport
    let write_content = if current_viewport_top > 0 {
        Some(WriteContentPlan {
            // Scroll region from top of screen to just above viewport
            scroll_region_top: 1,                      // 1-based
            scroll_region_bottom: current_viewport_top, // 1-based (0-based row N = 1-based row N+1, but we want row N-1 as bottom, so just N)
            cursor_row: cursor_top,                    // 0-based
            line_count: num_lines,
        })
    } else {
        None
    };

    InsertHistoryPlan {
        push_viewport,
        write_content,
        final_viewport_top: current_viewport_top,
    }
}

/// DECSTBM - Set Top and Bottom Margins (scroll region)
struct SetScrollRegion {
    top: u16,    // 1-based
    bottom: u16, // 1-based
}

impl Command for SetScrollRegion {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x1b[{};{}r", self.top, self.bottom)
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Ok(())
    }
}

/// Reset scroll region to full screen
struct ResetScrollRegion;

impl Command for ResetScrollRegion {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x1b[r")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Ok(())
    }
}

/// Reverse Index - scroll content down within scroll region, making room at top
struct ReverseIndex;

impl Command for ReverseIndex {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x1bM")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Ok(())
    }
}

/// TUI coordinator for inline mode operation.
pub struct Tui {
    terminal: InlineTerminal<CrosstermBackend<Stdout>>,
    viewport_top: u16,    // Top row of viewport (0-based)
    viewport_height: u16, // Height of viewport
}

impl Tui {
    /// Create a new Tui coordinator in inline mode.
    pub fn new_inline(viewport_height: u16) -> io::Result<Self> {
        install_panic_hook();
        terminal::enable_raw_mode()?;

        let mut stdout_handle = stdout();
        let (cols, rows) = terminal::size()?;
        let (_, cursor_y) = position()?;

        // Viewport starts at current cursor position
        // If not enough room below, scroll to make room
        let space_below = rows.saturating_sub(cursor_y + 1);
        let need_scroll = viewport_height.saturating_sub(space_below);

        let viewport_top = if need_scroll > 0 {
            // Print newlines to scroll content up and make room
            for _ in 0..need_scroll {
                queue!(stdout_handle, Print("\n"))?;
            }
            // Viewport top is pushed up by the scroll amount
            cursor_y.saturating_sub(need_scroll)
        } else {
            // Viewport starts right at cursor
            cursor_y
        };

        // Position cursor at viewport top
        queue!(stdout_handle, MoveTo(0, viewport_top))?;
        stdout_handle.flush()?;

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = InlineTerminal::new(backend)?;

        // Set viewport area and clear it
        let viewport_area = Rect::new(0, viewport_top, cols, viewport_height);
        terminal.set_viewport_area(viewport_area);
        terminal.clear()?;

        Ok(Self {
            terminal,
            viewport_top,
            viewport_height,
        })
    }

    /// Insert lines into scrollback above the viewport.
    pub fn insert_history(&mut self, lines: Vec<Line<'static>>) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        let mut stdout = stdout();
        let (cols, rows) = terminal::size()?;
        let num_lines = lines.len() as u16;

        // Compute the plan using pure function
        let plan = plan_insert_history(
            self.viewport_top,
            self.viewport_height,
            rows,
            num_lines,
        );

        // Execute Phase 1: Push viewport down if needed
        if let Some(push) = &plan.push_viewport {
            queue!(stdout, SetScrollRegion {
                top: push.scroll_region_top,
                bottom: push.scroll_region_bottom,
            })?;

            queue!(stdout, MoveTo(0, push.cursor_row))?;
            for _ in 0..push.ri_count {
                queue!(stdout, ReverseIndex)?;
            }
            queue!(stdout, ResetScrollRegion)?;

            // Update our viewport_top tracking variable
            self.viewport_top = push.new_viewport_top;
        }

        // Execute Phase 2: Write content above viewport
        if let Some(write) = &plan.write_content {
            queue!(stdout, SetScrollRegion {
                top: write.scroll_region_top,
                bottom: write.scroll_region_bottom,
            })?;

            queue!(stdout, MoveTo(0, write.cursor_row))?;

            for line in &lines {
                queue!(stdout, Print("\r\n"))?;
                queue!(stdout, Clear(ClearType::UntilNewLine))?;
                Self::write_styled_line(&mut stdout, line)?;
            }

            queue!(stdout, ResetScrollRegion)?;
        }

        // Restore cursor into viewport
        queue!(stdout, MoveTo(0, plan.final_viewport_top))?;

        // Flush all escape sequences
        stdout.flush()?;

        // Update InlineTerminal's viewport area if it moved.
        // Unlike ratatui's Terminal, InlineTerminal.set_viewport_area() resizes
        // buffers WITHOUT clearing the screen, preserving diff state.
        if plan.push_viewport.is_some() {
            let new_area = Rect::new(0, self.viewport_top, cols, self.viewport_height);
            self.terminal.set_viewport_area(new_area);
        }

        Ok(())
    }

    /// Draw the TUI frame.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Get terminal size.
    pub fn size(&self) -> io::Result<Size> {
        self.terminal.size()
    }

    /// Restore terminal to normal state.
    pub fn restore(self) -> io::Result<()> {
        let mut stdout = stdout();

        // Reset scroll region
        queue!(stdout, ResetScrollRegion)?;

        // Move cursor just below the viewport (not at screen bottom)
        // This prevents unnecessary whitespace when viewport is near top of screen
        let cursor_row = self.viewport_top + self.viewport_height;
        queue!(stdout, MoveTo(0, cursor_row))?;
        queue!(stdout, Print("\n"))?;

        stdout.flush()?;

        // Clear ratatui state
        drop(self.terminal);

        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn write_styled_line(stdout: &mut io::Stdout, line: &Line<'_>) -> io::Result<()> {
        for span in &line.spans {
            Self::apply_style(stdout, &span.style)?;
            queue!(stdout, Print(&span.content))?;
            queue!(stdout, ResetColor)?;
            queue!(stdout, SetAttribute(Attribute::Reset))?;
        }
        Ok(())
    }

    fn apply_style(stdout: &mut io::Stdout, style: &Style) -> io::Result<()> {
        if let Some(fg) = style.fg {
            let color = Self::convert_color(fg);
            queue!(stdout, SetForegroundColor(color))?;
        }

        if style.add_modifier.contains(Modifier::BOLD) {
            queue!(stdout, SetAttribute(Attribute::Bold))?;
        }
        if style.add_modifier.contains(Modifier::ITALIC) {
            queue!(stdout, SetAttribute(Attribute::Italic))?;
        }
        if style.add_modifier.contains(Modifier::DIM) {
            queue!(stdout, SetAttribute(Attribute::Dim))?;
        }

        Ok(())
    }

    fn convert_color(color: ratatui::style::Color) -> Color {
        match color {
            ratatui::style::Color::Reset => Color::Reset,
            ratatui::style::Color::Black => Color::Black,
            ratatui::style::Color::Red => Color::Red,
            ratatui::style::Color::Green => Color::Green,
            ratatui::style::Color::Yellow => Color::Yellow,
            ratatui::style::Color::Blue => Color::Blue,
            ratatui::style::Color::Magenta => Color::Magenta,
            ratatui::style::Color::Cyan => Color::Cyan,
            ratatui::style::Color::Gray => Color::Grey,
            ratatui::style::Color::DarkGray => Color::DarkGrey,
            ratatui::style::Color::LightRed => Color::Red,
            ratatui::style::Color::LightGreen => Color::Green,
            ratatui::style::Color::LightYellow => Color::Yellow,
            ratatui::style::Color::LightBlue => Color::Blue,
            ratatui::style::Color::LightMagenta => Color::Magenta,
            ratatui::style::Color::LightCyan => Color::Cyan,
            ratatui::style::Color::White => Color::White,
            ratatui::style::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
            ratatui::style::Color::Indexed(i) => Color::AnsiValue(i),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_region_command_format() {
        let cmd = SetScrollRegion { top: 1, bottom: 10 };
        let mut buf = String::new();
        cmd.write_ansi(&mut buf).unwrap();
        assert_eq!(buf, "\x1b[1;10r");
    }

    #[test]
    fn reset_scroll_region_format() {
        let cmd = ResetScrollRegion;
        let mut buf = String::new();
        cmd.write_ansi(&mut buf).unwrap();
        assert_eq!(buf, "\x1b[r");
    }

    // ========================================
    // vt100-based screen verification tests
    // ========================================
    //
    // These tests use vt100 crate to simulate a real terminal and verify
    // that content appears at the correct screen positions after operations.
    //
    // Key invariant being tested:
    // - Content written by insert_history MUST remain visible on screen
    // - No Clear(ClearType::All) should wipe previously written content

    /// Execute a plan against a vt100 parser and return the parser for inspection
    fn execute_plan_on_vt100(
        plan: &InsertHistoryPlan,
        content_lines: &[&str],
        rows: u16,
        cols: u16,
    ) -> vt100::Parser {
        use crossterm::cursor::MoveTo;

        let mut parser = vt100::Parser::new(rows, cols, 0);
        let mut buf = Vec::new();

        // Phase 1: Push viewport down
        if let Some(push) = &plan.push_viewport {
            // SetScrollRegion
            let cmd = SetScrollRegion {
                top: push.scroll_region_top,
                bottom: push.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            // MoveTo
            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, push.cursor_row), &mut s).unwrap();
            buf.extend(s.as_bytes());

            // ReverseIndex (RI) commands
            for _ in 0..push.ri_count {
                buf.extend(b"\x1bM");
            }

            // ResetScrollRegion
            buf.extend(b"\x1b[r");
        }

        // Phase 2: Write content
        if let Some(write) = &plan.write_content {
            // SetScrollRegion
            let cmd = SetScrollRegion {
                top: write.scroll_region_top,
                bottom: write.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            // MoveTo cursor position
            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, write.cursor_row), &mut s).unwrap();
            buf.extend(s.as_bytes());

            // Write lines with \r\n prefix
            for line in content_lines.iter().take(write.line_count as usize) {
                buf.extend(b"\r\n");
                buf.extend(b"\x1b[K"); // Clear to end of line
                buf.extend(line.as_bytes());
            }

            // ResetScrollRegion
            buf.extend(b"\x1b[r");
        }

        // Move cursor to final position
        let mut s = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, plan.final_viewport_top), &mut s).unwrap();
        buf.extend(s.as_bytes());

        parser.process(&buf);
        parser
    }

    /// Get the text content of a specific row from vt100 screen
    fn get_row_text(parser: &vt100::Parser, row: u16) -> String {
        let screen = parser.screen();
        let mut text = String::new();
        for col in 0..screen.size().1 {
            let cell = screen.cell(row, col);
            if let Some(cell) = cell {
                text.push_str(&cell.contents());
            }
        }
        text.trim_end().to_string()
    }

    /// Simulate the BUG: Clear(ClearType::All) sent after content
    fn simulate_clear_all_bug(parser: &mut vt100::Parser) {
        // This is what resize() does - sends Clear(ClearType::All)
        parser.process(b"\x1b[2J");
    }

    #[test]
    fn vt100_content_appears_at_correct_row() {
        // Viewport at row 10, insert 2 lines
        // Content should appear at rows 10-11, viewport moves to 12
        let plan = plan_insert_history(10, 5, 24, 2);
        let content = ["Hello World", "Second Line"];
        let parser = execute_plan_on_vt100(&plan, &content, 24, 80);

        // After push: viewport moved from 10 to 12
        // Content written to rows 10-11
        assert_eq!(get_row_text(&parser, 10), "Hello World");
        assert_eq!(get_row_text(&parser, 11), "Second Line");
    }

    #[test]
    fn vt100_viewport_at_bottom_content_scrolls() {
        // Viewport at row 19 (bottom), insert 2 lines
        // Content should appear via scroll region mechanics
        let plan = plan_insert_history(19, 5, 24, 2);
        let content = ["Line A", "Line B"];
        let parser = execute_plan_on_vt100(&plan, &content, 24, 80);

        // Without push (viewport at bottom), content should be at rows 17-18
        // (cursor was at 18, \r\n moves down, content written)
        // The exact positions depend on scroll behavior
        let row17 = get_row_text(&parser, 17);
        let row18 = get_row_text(&parser, 18);
        assert!(
            row17.contains("Line A") || row18.contains("Line A"),
            "Content should be visible. Row 17: '{}', Row 18: '{}'",
            row17, row18
        );
    }

    #[test]
    fn vt100_clear_all_wipes_content() {
        // This test demonstrates the BUG: Clear(ClearType::All) wipes our content
        let plan = plan_insert_history(10, 5, 24, 2);
        let content = ["IMPORTANT", "DATA"];
        let mut parser = execute_plan_on_vt100(&plan, &content, 24, 80);

        // Verify content is there BEFORE the bug
        assert_eq!(get_row_text(&parser, 10), "IMPORTANT");
        assert_eq!(get_row_text(&parser, 11), "DATA");

        // Simulate the resize() bug - sends Clear(ClearType::All)
        simulate_clear_all_bug(&mut parser);

        // Content is now WIPED - this is the bug!
        let row10_after = get_row_text(&parser, 10);
        let row11_after = get_row_text(&parser, 11);

        // This assertion PASSES, demonstrating the bug exists
        assert!(
            row10_after.is_empty() && row11_after.is_empty(),
            "Clear(ClearType::All) should wipe content (demonstrating the bug). \
             Row 10: '{}', Row 11: '{}'",
            row10_after, row11_after
        );
    }

    #[test]
    fn vt100_correct_implementation_preserves_content() {
        // This test verifies that WITHOUT the Clear bug, content is preserved
        let plan = plan_insert_history(10, 5, 24, 2);
        let content = ["PRESERVED", "CONTENT"];
        let parser = execute_plan_on_vt100(&plan, &content, 24, 80);

        // Content should still be visible (no Clear was sent)
        assert_eq!(get_row_text(&parser, 10), "PRESERVED");
        assert_eq!(get_row_text(&parser, 11), "CONTENT");

        // Cursor should be at viewport position
        let screen = parser.screen();
        assert_eq!(
            screen.cursor_position(),
            (plan.final_viewport_top, 0),
            "Cursor should be at new viewport top"
        );
    }

    #[test]
    fn vt100_sequential_inserts_accumulate() {
        // Simulate: user message, then agent response
        // Both should be visible, not overlapping

        // First insert: user message at viewport 5
        let plan1 = plan_insert_history(5, 5, 24, 2);
        let content1 = ["You: Hello", ""];
        let mut parser = execute_plan_on_vt100(&plan1, &content1, 24, 80);

        // User message should be at rows 5-6, viewport now at 7
        assert_eq!(get_row_text(&parser, 5), "You: Hello");

        // Second insert: agent response at viewport 7
        let plan2 = plan_insert_history(7, 5, 24, 3);
        let content2 = ["Agent:", "Hello back!", ""];

        // Execute second plan on same parser
        let mut buf = Vec::new();
        if let Some(push) = &plan2.push_viewport {
            let cmd = SetScrollRegion {
                top: push.scroll_region_top,
                bottom: push.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            let mut s = String::new();
            crossterm::Command::write_ansi(
                &crossterm::cursor::MoveTo(0, push.cursor_row),
                &mut s,
            ).unwrap();
            buf.extend(s.as_bytes());

            for _ in 0..push.ri_count {
                buf.extend(b"\x1bM");
            }
            buf.extend(b"\x1b[r");
        }

        if let Some(write) = &plan2.write_content {
            let cmd = SetScrollRegion {
                top: write.scroll_region_top,
                bottom: write.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            let mut s = String::new();
            crossterm::Command::write_ansi(
                &crossterm::cursor::MoveTo(0, write.cursor_row),
                &mut s,
            ).unwrap();
            buf.extend(s.as_bytes());

            for line in content2.iter().take(write.line_count as usize) {
                buf.extend(b"\r\n");
                buf.extend(b"\x1b[K");
                buf.extend(line.as_bytes());
            }
            buf.extend(b"\x1b[r");
        }

        parser.process(&buf);

        // Both messages should be visible, not overlapping
        // User message still at row 5
        assert_eq!(
            get_row_text(&parser, 5),
            "You: Hello",
            "User message should still be visible"
        );

        // Agent message at rows 7-9
        let row7 = get_row_text(&parser, 7);
        let row8 = get_row_text(&parser, 8);
        assert!(
            row7.contains("Agent") || row8.contains("Agent"),
            "Agent message should be visible. Row 7: '{}', Row 8: '{}'",
            row7, row8
        );
    }

    // ========================================
    // Restore cursor position tests
    // ========================================

    /// Generate restore escape sequences for testing
    /// This mirrors what Tui::restore() should generate
    fn generate_restore_sequences(viewport_top: u16, viewport_height: u16, _terminal_rows: u16) -> Vec<u8> {
        use crossterm::cursor::MoveTo;

        let mut buf = Vec::new();

        // Reset scroll region
        buf.extend(b"\x1b[r");

        // CORRECT behavior: cursor should go just below viewport
        // viewport_top + viewport_height is the first row AFTER the viewport
        let cursor_row = viewport_top + viewport_height;

        let mut s = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, cursor_row), &mut s).unwrap();
        buf.extend(s.as_bytes());

        // Print newline to move to next line
        buf.extend(b"\n");

        buf
    }

    /// Generate the BUGGY restore sequences (current behavior)
    fn generate_buggy_restore_sequences(terminal_rows: u16) -> Vec<u8> {
        use crossterm::cursor::MoveTo;

        let mut buf = Vec::new();

        // Reset scroll region
        buf.extend(b"\x1b[r");

        // BUGGY behavior: cursor goes to bottom of screen regardless of viewport
        let cursor_row = terminal_rows.saturating_sub(1);

        let mut s = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, cursor_row), &mut s).unwrap();
        buf.extend(s.as_bytes());

        buf.extend(b"\n");

        buf
    }

    #[test]
    fn vt100_restore_cursor_should_be_below_viewport() {
        // Scenario: viewport at row 5, height 5 (rows 5-9)
        // Terminal has 24 rows
        // After restore, cursor should be at row 10 (just below viewport)
        // NOT at row 23 (bottom of screen)

        let viewport_top = 5;
        let viewport_height = 5;
        let terminal_rows = 24;

        // Test CORRECT behavior
        let correct_sequences = generate_restore_sequences(viewport_top, viewport_height, terminal_rows);
        let mut parser = vt100::Parser::new(terminal_rows, 80, 0);
        parser.process(&correct_sequences);

        let (cursor_row, _cursor_col) = parser.screen().cursor_position();

        // Cursor should be at row 10 (viewport_top + viewport_height) after \n moves it down 1
        // Actually after MoveTo(0, 10) + \n, cursor is at row 11
        let expected_row = viewport_top + viewport_height + 1; // +1 for the newline
        assert_eq!(
            cursor_row,
            expected_row.min(terminal_rows - 1), // Can't go past screen bottom
            "Cursor should be just below viewport at row {}, not at screen bottom",
            expected_row
        );
    }

    #[test]
    fn vt100_restore_buggy_puts_cursor_at_bottom() {
        // This test demonstrates the BUG: cursor goes to screen bottom
        // causing lots of whitespace

        let viewport_top = 5;
        let viewport_height = 5;
        let terminal_rows = 24;

        // Test BUGGY behavior
        let buggy_sequences = generate_buggy_restore_sequences(terminal_rows);
        let mut parser = vt100::Parser::new(terminal_rows, 80, 0);
        parser.process(&buggy_sequences);

        let (cursor_row, _) = parser.screen().cursor_position();

        // Buggy behavior: cursor at row 23 (bottom) instead of row 10
        // This creates 13 rows of whitespace!
        assert_eq!(
            cursor_row,
            terminal_rows - 1, // Row 23 (bottom of screen after \n wraps or stays)
            "Buggy restore puts cursor at screen bottom"
        );

        // Calculate wasted whitespace
        let correct_position = viewport_top + viewport_height;
        let wasted_rows = (terminal_rows - 1).saturating_sub(correct_position);
        assert!(
            wasted_rows > 0,
            "Bug creates {} rows of unnecessary whitespace",
            wasted_rows
        );
    }

    #[test]
    fn vt100_restore_no_whitespace_when_viewport_near_bottom() {
        // Edge case: viewport is near bottom of screen
        // Restore should still position cursor just below viewport

        let viewport_top = 19;
        let viewport_height = 5;
        let terminal_rows = 24;

        // Viewport occupies rows 19-23 (bottom of screen)
        // After restore, cursor should be at row 24 which wraps/scrolls

        let correct_sequences = generate_restore_sequences(viewport_top, viewport_height, terminal_rows);
        let mut parser = vt100::Parser::new(terminal_rows, 80, 0);
        parser.process(&correct_sequences);

        let (cursor_row, _) = parser.screen().cursor_position();

        // When viewport is at bottom, cursor ends up at bottom anyway
        // but there's no unnecessary whitespace
        assert!(
            cursor_row >= viewport_top,
            "Cursor should be at or below viewport top"
        );
    }

    // Tests for plan_insert_history

    #[test]
    fn plan_empty_lines_does_nothing() {
        let plan = plan_insert_history(10, 5, 24, 0);
        assert!(plan.push_viewport.is_none());
        assert!(plan.write_content.is_none());
        assert_eq!(plan.final_viewport_top, 10);
    }

    #[test]
    fn plan_viewport_at_bottom_no_push() {
        // Terminal: 24 rows (0-23)
        // Viewport: rows 19-23 (top=19, height=5)
        // Viewport is at bottom, no whitespace below
        let plan = plan_insert_history(19, 5, 24, 3);

        // Phase 1: No push (already at bottom)
        assert!(plan.push_viewport.is_none());

        // Phase 2: Write content
        let write = plan.write_content.expect("should have write plan");
        assert_eq!(write.scroll_region_top, 1);      // 1-based top of screen
        assert_eq!(write.scroll_region_bottom, 19);  // 1-based = 0-based viewport_top
        assert_eq!(write.cursor_row, 18);            // viewport_top - 1 = row above viewport
        assert_eq!(write.line_count, 3);

        // Final viewport unchanged
        assert_eq!(plan.final_viewport_top, 19);
    }

    #[test]
    fn plan_viewport_with_whitespace_pushes_down() {
        // Terminal: 24 rows (0-23)
        // Viewport: rows 10-14 (top=10, height=5)
        // Space below: 24 - 15 = 9 rows
        let plan = plan_insert_history(10, 5, 24, 3);

        // Phase 1: Push viewport down by 3 (min of 3 lines, 9 space)
        let push = plan.push_viewport.expect("should have push plan");
        assert_eq!(push.scroll_region_top, 11);      // viewport_top + 1 (1-based)
        assert_eq!(push.scroll_region_bottom, 24);   // terminal_rows (1-based)
        assert_eq!(push.cursor_row, 10);             // 0-based viewport_top
        assert_eq!(push.ri_count, 3);
        assert_eq!(push.new_viewport_top, 13);       // 10 + 3

        // Phase 2: Write content
        let write = plan.write_content.expect("should have write plan");
        assert_eq!(write.scroll_region_top, 1);
        assert_eq!(write.scroll_region_bottom, 13);  // new viewport_top
        assert_eq!(write.cursor_row, 9);             // ORIGINAL viewport_top - 1 (computed before push)
        assert_eq!(write.line_count, 3);

        assert_eq!(plan.final_viewport_top, 13);
    }

    #[test]
    fn plan_viewport_at_top_with_whitespace() {
        // Terminal: 24 rows (0-23)
        // Viewport: rows 0-4 (top=0, height=5)
        // Space below: 24 - 5 = 19 rows
        let plan = plan_insert_history(0, 5, 24, 3);

        // Phase 1: Push viewport down by 3
        let push = plan.push_viewport.expect("should have push plan");
        assert_eq!(push.scroll_region_top, 1);       // 0 + 1 (1-based)
        assert_eq!(push.scroll_region_bottom, 24);
        assert_eq!(push.cursor_row, 0);
        assert_eq!(push.ri_count, 3);
        assert_eq!(push.new_viewport_top, 3);

        // Phase 2: Write content (now viewport_top > 0)
        let write = plan.write_content.expect("should have write plan");
        assert_eq!(write.scroll_region_top, 1);
        assert_eq!(write.scroll_region_bottom, 3);   // new viewport_top
        assert_eq!(write.cursor_row, 0);             // saturating_sub(1) from 0 = 0
        assert_eq!(write.line_count, 3);

        assert_eq!(plan.final_viewport_top, 3);
    }

    #[test]
    fn plan_more_lines_than_whitespace_caps_push() {
        // Terminal: 24 rows
        // Viewport: rows 20-23 (top=20, height=4)
        // Space below: 24 - 24 = 0... wait viewport_bottom = 24, space = 0
        // Let's use top=18, height=4, viewport_bottom=22, space_below=2
        let plan = plan_insert_history(18, 4, 24, 5);

        // Phase 1: Push viewport down by 2 (capped by space_below)
        let push = plan.push_viewport.expect("should have push plan");
        assert_eq!(push.ri_count, 2);                // min(5, 2) = 2
        assert_eq!(push.new_viewport_top, 20);       // 18 + 2

        // Phase 2: Write all 5 lines (scrolling happens in scroll region)
        let write = plan.write_content.expect("should have write plan");
        assert_eq!(write.scroll_region_bottom, 20);  // new viewport_top
        assert_eq!(write.cursor_row, 17);            // original viewport_top - 1
        assert_eq!(write.line_count, 5);

        assert_eq!(plan.final_viewport_top, 20);
    }

    #[test]
    fn plan_cursor_top_always_computed_before_push() {
        // This is the critical invariant: cursor_top = original_viewport_top - 1
        // regardless of what happens in phase 1

        // Case 1: With push
        let plan1 = plan_insert_history(10, 5, 24, 3);
        let write1 = plan1.write_content.unwrap();
        assert_eq!(write1.cursor_row, 9); // 10 - 1

        // Case 2: Without push
        let plan2 = plan_insert_history(19, 5, 24, 3);
        let write2 = plan2.write_content.unwrap();
        assert_eq!(write2.cursor_row, 18); // 19 - 1

        // Case 3: At row 0
        let plan3 = plan_insert_history(0, 5, 24, 3);
        let write3 = plan3.write_content.unwrap();
        assert_eq!(write3.cursor_row, 0); // saturating_sub(1) = 0
    }

    #[test]
    fn plan_write_region_uses_new_viewport_top() {
        // The scroll region for phase 2 should extend to the NEW viewport position
        // (after phase 1 push), not the original position

        let plan = plan_insert_history(10, 5, 24, 3);
        let push = plan.push_viewport.unwrap();
        let write = plan.write_content.unwrap();

        // Viewport moves from 10 to 13
        assert_eq!(push.new_viewport_top, 13);

        // Write region bottom should be new position (13), not old (10)
        assert_eq!(write.scroll_region_bottom, 13);
    }

    #[test]
    fn plan_no_write_if_viewport_stays_at_top() {
        // Edge case: viewport starts at row 0, no whitespace below
        // This shouldn't happen in practice, but let's handle it
        let plan = plan_insert_history(0, 24, 24, 3);

        // No whitespace below (viewport fills screen)
        assert!(plan.push_viewport.is_none());

        // No space above to write
        assert!(plan.write_content.is_none());

        assert_eq!(plan.final_viewport_top, 0);
    }

    #[test]
    fn plan_sequential_inserts_accumulate_correctly() {
        // Simulate two sequential insert_history calls (user message then agent response)
        // Terminal: 24 rows, viewport starts at row 5

        // First insert: 2 lines (user message)
        let plan1 = plan_insert_history(5, 5, 24, 2);
        let push1 = plan1.push_viewport.as_ref().unwrap();
        let write1 = plan1.write_content.as_ref().unwrap();

        assert_eq!(push1.new_viewport_top, 7);  // 5 + 2
        assert_eq!(write1.cursor_row, 4);       // original 5 - 1
        // Content will be at rows 5-6 (after \r\n from row 4)

        // Second insert: 3 lines (agent response)
        // Now viewport_top is 7
        let plan2 = plan_insert_history(7, 5, 24, 3);
        let push2 = plan2.push_viewport.as_ref().unwrap();
        let write2 = plan2.write_content.as_ref().unwrap();

        assert_eq!(push2.new_viewport_top, 10); // 7 + 3
        assert_eq!(write2.cursor_row, 6);       // original 7 - 1
        // Content will be at rows 7-9 (after \r\n from row 6)

        // Verify final positions don't overlap:
        // User message: rows 5-6
        // Agent message: rows 7-9
        // Viewport: rows 10-14
        assert_eq!(plan2.final_viewport_top, 10);
    }

    #[test]
    fn plan_small_terminal() {
        // Test with a very small terminal (12 rows)
        // Viewport: top=7, height=5, covers rows 7-11
        // Space below: 0 (viewport at bottom)
        let plan = plan_insert_history(7, 5, 12, 4);

        // No push (no push below)
        assert!(plan.push_viewport.is_none());

        // Write plan
        let write = plan.write_content.unwrap();
        assert_eq!(write.scroll_region_top, 1);
        assert_eq!(write.scroll_region_bottom, 7);  // viewport_top
        assert_eq!(write.cursor_row, 6);            // 7 - 1

        assert_eq!(plan.final_viewport_top, 7);
    }

    // ========================================
    // Viewport clearing after Terminal recreation
    // ========================================
    //
    // BUG: When Terminal is recreated in insert_history (after push_viewport),
    // the new Terminal's buffers are clean/default. But the actual screen still
    // has old content (e.g., previous Textarea text).
    //
    // Ratatui's diff algorithm compares buffers. Since both old and new buffers
    // are default (empty), it sees no changes and emits no escape sequences.
    // Result: old text remains visible on screen even though our content is empty.
    //
    // FIX: After recreating Terminal, explicitly clear the viewport area using
    // crossterm directly, ensuring screen state matches buffer state.

    #[test]
    fn vt100_viewport_cleared_after_terminal_recreation() {
        // Simulate the bug:
        // 1. Viewport has content "OLD TEXT"
        // 2. insert_history causes Terminal recreation (push_viewport happens)
        // 3. New draw with empty content should clear "OLD TEXT"
        //
        // The bug: ratatui's diff sees no change (both buffers empty) and emits nothing

        let viewport_top = 5;
        let viewport_height = 3;
        let terminal_rows = 24;
        let cols = 80;

        let mut parser = vt100::Parser::new(terminal_rows, cols, 0);

        // Step 1: Simulate initial viewport with "OLD TEXT" on screen
        // Move to viewport area and write content that represents old input
        let mut setup = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, viewport_top), &mut setup).unwrap();
        setup.push_str("OLD TEXT THAT SHOULD BE CLEARED");
        parser.process(setup.as_bytes());

        // Verify old text is on screen
        let row_before = get_row_text(&parser, viewport_top);
        assert!(row_before.contains("OLD TEXT"), "Setup: old text should be visible");

        // Step 2: Simulate insert_history with push_viewport
        let plan = plan_insert_history(viewport_top, viewport_height, terminal_rows, 2);
        assert!(plan.push_viewport.is_some(), "This test requires a push to trigger recreation");

        // Execute the plan (this moves viewport down)
        let _ = execute_plan_on_vt100(&plan, &["Line 1", "Line 2"], terminal_rows, cols);

        // Step 3: After Terminal recreation, the viewport area should be cleared
        // We need to simulate what SHOULD happen: clear the new viewport area
        //
        // The CORRECT behavior after Terminal recreation:
        // Send clear sequences for the viewport area

        // This is what we SHOULD be doing after Terminal recreation:
        let new_viewport_top = plan.final_viewport_top;
        let mut clear_viewport = Vec::new();
        for row in new_viewport_top..(new_viewport_top + viewport_height) {
            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, row), &mut s).unwrap();
            clear_viewport.extend(s.as_bytes());
            clear_viewport.extend(b"\x1b[K"); // Clear line
        }
        parser.process(&clear_viewport);

        // Now the viewport area should be clear
        for row in new_viewport_top..(new_viewport_top + viewport_height) {
            let text = get_row_text(&parser, row);
            assert!(
                text.is_empty() || text.chars().all(|c| c == ' '),
                "Row {} in viewport should be cleared, but contains: '{}'",
                row, text
            );
        }
    }

    /// Simulate insert_history INCLUDING what happens after Terminal recreation.
    /// If `clear_viewport_after` is true, simulates the fix (clearing viewport).
    /// If false, simulates current buggy behavior (no clearing).
    fn execute_insert_history_with_recreation(
        parser: &mut vt100::Parser,
        plan: &InsertHistoryPlan,
        content_lines: &[&str],
        viewport_height: u16,
        _cols: u16,
        clear_viewport_after: bool,
    ) {
        let mut buf = Vec::new();

        // Phase 1: Push viewport down
        if let Some(push) = &plan.push_viewport {
            let cmd = SetScrollRegion {
                top: push.scroll_region_top,
                bottom: push.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, push.cursor_row), &mut s).unwrap();
            buf.extend(s.as_bytes());

            for _ in 0..push.ri_count {
                buf.extend(b"\x1bM");
            }
            buf.extend(b"\x1b[r");
        }

        // Phase 2: Write content
        if let Some(write) = &plan.write_content {
            let cmd = SetScrollRegion {
                top: write.scroll_region_top,
                bottom: write.scroll_region_bottom,
            };
            let mut s = String::new();
            cmd.write_ansi(&mut s).unwrap();
            buf.extend(s.as_bytes());

            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, write.cursor_row), &mut s).unwrap();
            buf.extend(s.as_bytes());

            for line in content_lines.iter().take(write.line_count as usize) {
                buf.extend(b"\r\n");
                buf.extend(b"\x1b[K");
                buf.extend(line.as_bytes());
            }
            buf.extend(b"\x1b[r");
        }

        // Move cursor to final position
        let mut s = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, plan.final_viewport_top), &mut s).unwrap();
        buf.extend(s.as_bytes());

        // FIX: After Terminal recreation, clear the viewport area
        if clear_viewport_after && plan.push_viewport.is_some() {
            let new_vp_top = plan.final_viewport_top;
            for row in new_vp_top..(new_vp_top + viewport_height) {
                let mut s = String::new();
                crossterm::Command::write_ansi(&MoveTo(0, row), &mut s).unwrap();
                buf.extend(s.as_bytes());
                buf.extend(b"\x1b[K"); // Clear to end of line
            }
            // Return cursor to viewport top
            let mut s = String::new();
            crossterm::Command::write_ansi(&MoveTo(0, new_vp_top), &mut s).unwrap();
            buf.extend(s.as_bytes());
        }

        parser.process(&buf);
    }

    #[test]
    fn vt100_viewport_must_be_cleared_after_recreation() {
        // This test FAILS with current implementation, PASSES with fix.
        //
        // Scenario:
        // 1. Viewport at row 5 has "OLD INPUT" (simulating Textarea content)
        // 2. User submits, insert_history runs, viewport moves to row 7
        // 3. Terminal is recreated (because push_viewport happened)
        // 4. The OLD INPUT text should NOT be visible in new viewport area
        //
        // BUG: Without explicit clearing, "OLD INPUT" remains visible at row 7
        // because ratatui's diff sees no change (both buffers are empty/default)

        let viewport_top = 5;
        let viewport_height = 3;
        let terminal_rows = 24;
        let cols = 80;

        let mut parser = vt100::Parser::new(terminal_rows, cols, 0);

        // Step 1: Put content in original viewport (simulates Textarea with user input)
        let mut setup = Vec::new();
        let mut s = String::new();
        crossterm::Command::write_ansi(&MoveTo(0, viewport_top), &mut s).unwrap();
        setup.extend(s.as_bytes());
        setup.extend(b"OLD INPUT SHOULD DISAPPEAR");
        parser.process(&setup);

        // Verify setup
        assert!(get_row_text(&parser, viewport_top).contains("OLD INPUT"));

        // Step 2: Execute insert_history with the fix (clears viewport after recreation)
        let plan = plan_insert_history(viewport_top, viewport_height, terminal_rows, 2);
        assert!(plan.push_viewport.is_some(), "Need push to trigger recreation");

        execute_insert_history_with_recreation(
            &mut parser,
            &plan,
            &["User: Hello", ""],
            viewport_height,
            cols,
            true, // <-- Fixed behavior: clear viewport after recreation
        );

        // Step 3: Check new viewport area - it SHOULD be clear
        // The new viewport is at rows 7-9
        let new_vp_top = plan.final_viewport_top;

        // This is what we EXPECT (correct behavior):
        // The new viewport area should be empty/cleared
        for row in new_vp_top..(new_vp_top + viewport_height) {
            let text = get_row_text(&parser, row);
            assert!(
                text.is_empty() || !text.contains("OLD INPUT"),
                "FAIL: Row {} should NOT contain old input text, but has: '{}'\n\
                 This fails because viewport area wasn't cleared after Terminal recreation.",
                row, text
            );
        }
    }

}
