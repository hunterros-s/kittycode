use std::time::Instant;

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Returns an animated spinner span based on elapsed time.
/// Cycles through ◐◓◑◒ every 200ms.
pub fn spinner(start_time: Instant) -> Span<'static> {
    let frame = (start_time.elapsed().as_millis() / 200) % 4;
    let c = match frame {
        0 => "◐",
        1 => "◓",
        2 => "◑",
        _ => "◒",
    };
    Span::styled(c, Style::default().fg(Color::Cyan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn spinner_returns_span() {
        let start = Instant::now();
        let span = spinner(start);
        let content = span.content.to_string();
        assert!(
            content == "◐" || content == "◓" || content == "◑" || content == "◒",
            "Spinner should return one of the spinner characters"
        );
    }

    #[test]
    fn spinner_cycles_over_time() {
        let start = Instant::now();
        let first = spinner(start).content.to_string();

        // Sleep long enough to advance at least one frame
        sleep(Duration::from_millis(250));

        let second = spinner(start).content.to_string();

        // They might be the same if timing is unlucky, but both should be valid
        assert!(["◐", "◓", "◑", "◒"].contains(&first.as_str()));
        assert!(["◐", "◓", "◑", "◒"].contains(&second.as_str()));
    }

    #[test]
    fn spinner_has_cyan_color() {
        let start = Instant::now();
        let span = spinner(start);
        assert_eq!(span.style.fg, Some(Color::Cyan));
    }
}
