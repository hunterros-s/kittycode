mod inline_terminal;
mod tui;

pub use inline_terminal::{Frame, InlineTerminal};
pub use tui::Tui;

use std::io::{self, stdout, Stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;

pub type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

static PANIC_HOOK_INSTALLED: Once = Once::new();
static IN_ALT_SCREEN: AtomicBool = AtomicBool::new(false);

pub(crate) fn install_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = restore();
            original_hook(panic_info);
        }));
    });
}

/// Initialize terminal in alternate screen mode.
/// For inline mode, use `Tui::new_inline()` instead.
pub fn init() -> io::Result<Terminal> {
    install_panic_hook();
    terminal::enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    IN_ALT_SCREEN.store(true, Ordering::SeqCst);
    let backend = CrosstermBackend::new(stdout());
    Terminal::new(backend)
}

/// Restore terminal from alternate screen mode.
/// Also disables raw mode. Safe to call even if not in alt screen.
pub fn restore() -> io::Result<()> {
    if IN_ALT_SCREEN.load(Ordering::SeqCst) {
        stdout().execute(LeaveAlternateScreen)?;
        IN_ALT_SCREEN.store(false, Ordering::SeqCst);
    }
    terminal::disable_raw_mode()?;
    Ok(())
}

pub fn poll_event(timeout: Duration) -> io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn read_event() -> io::Result<Event> {
    event::read()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_hook_can_be_installed_multiple_times() {
        install_panic_hook();
        install_panic_hook();
    }
}
