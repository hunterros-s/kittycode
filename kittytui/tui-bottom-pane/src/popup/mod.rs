mod command;

pub use command::{CommandPopup, SlashCommand, COMMANDS};

pub enum ActivePopup {
    None,
    Command(CommandPopup),
}

impl Default for ActivePopup {
    fn default() -> Self {
        Self::None
    }
}
