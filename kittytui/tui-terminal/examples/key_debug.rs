use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::io::{stdout, Write};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;

    // Enable keyboard enhancement with all flags
    use crossterm::event::{PushKeyboardEnhancementFlags, KeyboardEnhancementFlags};
    use crossterm::execute;
    let enhanced = execute!(
        stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    );
    println!("Keyboard enhancement: {:?}\r", enhanced);

    println!("Press keys to see events. Ctrl+C to exit.\r");
    println!("Try: Enter, Shift+Enter, Alt+Enter\r");
    println!("---\r");

    loop {
        if event::poll(Duration::from_millis(100))? {
            let evt = event::read()?;
            match &evt {
                Event::Key(KeyEvent { code, modifiers, kind, state }) => {
                    if modifiers.contains(KeyModifiers::CONTROL) && *code == KeyCode::Char('c') {
                        break;
                    }
                    println!("Key: {:?}, Mods: {:?}, Kind: {:?}, State: {:?}\r", code, modifiers, kind, state);
                }
                other => {
                    println!("Other event: {:?}\r", other);
                }
            }
        }
    }

    use crossterm::event::PopKeyboardEnhancementFlags;
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    terminal::disable_raw_mode()?;
    Ok(())
}
