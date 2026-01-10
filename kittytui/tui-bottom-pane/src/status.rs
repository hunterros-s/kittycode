use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tui_core::Renderable;

const SPINNER_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub struct StatusIndicator {
    label: String,
    start_time: Instant,
    paused_at: Option<Instant>,
    paused_duration: Duration,
    completed: Option<(bool, Duration)>,
}

impl StatusIndicator {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            start_time: Instant::now(),
            paused_at: None,
            paused_duration: Duration::ZERO,
            completed: None,
        }
    }

    pub fn complete(&mut self, success: bool) {
        self.completed = Some((success, self.elapsed()));
    }

    pub fn is_complete(&self) -> bool {
        self.completed.is_some()
    }

    pub fn pause(&mut self) {
        if self.paused_at.is_none() && self.completed.is_none() {
            self.paused_at = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            self.paused_duration += paused_at.elapsed();
        }
    }

    pub fn elapsed(&self) -> Duration {
        let raw_elapsed = if let Some(paused_at) = self.paused_at {
            paused_at.duration_since(self.start_time)
        } else {
            self.start_time.elapsed()
        };
        raw_elapsed.saturating_sub(self.paused_duration)
    }

    fn spinner_frame(&self) -> &'static str {
        let idx = (self.start_time.elapsed().as_millis() / 200) % 4;
        SPINNER_FRAMES[idx as usize]
    }

    pub fn render_line(&self) -> Line<'static> {
        if let Some((success, duration)) = self.completed {
            let (symbol, color) = if success {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            Line::from(vec![
                Span::styled(symbol, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw(self.label.clone()),
                Span::styled(
                    format!(" ({:.1}s)", duration.as_secs_f64()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            let elapsed = self.elapsed();
            Line::from(vec![
                Span::styled(self.spinner_frame(), Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::raw(self.label.clone()),
                Span::styled(
                    format!(" [{}s]", elapsed.as_secs()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
    }
}

impl Renderable for StatusIndicator {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let line = self.render_line();
        buf.set_line(area.x, area.y, &line, area.width);
    }

    fn height(&self, _width: u16) -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_status() {
        let status = StatusIndicator::new("test");
        assert!(!status.is_complete());
    }

    #[test]
    fn test_complete() {
        let mut status = StatusIndicator::new("test");
        status.complete(true);
        assert!(status.is_complete());
    }

    #[test]
    fn test_pause_resume() {
        let mut status = StatusIndicator::new("test");
        std::thread::sleep(Duration::from_millis(10));
        status.pause();
        let paused_elapsed = status.elapsed();
        std::thread::sleep(Duration::from_millis(50));
        let still_paused = status.elapsed();
        assert_eq!(paused_elapsed, still_paused);
        status.resume();
        std::thread::sleep(Duration::from_millis(10));
        let resumed_elapsed = status.elapsed();
        assert!(resumed_elapsed > still_paused);
    }
}
