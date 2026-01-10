use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tui_core::Renderable;

#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "help",
        description: "Show help",
    },
    SlashCommand {
        name: "clear",
        description: "Clear conversation",
    },
    SlashCommand {
        name: "model",
        description: "Change the model",
    },
];

pub struct CommandPopup {
    filter: String,
    selected_index: usize,
    filtered_commands: Vec<&'static SlashCommand>,
}

impl CommandPopup {
    pub fn new(filter: &str) -> Self {
        let mut popup = Self {
            filter: String::new(),
            selected_index: 0,
            filtered_commands: Vec::new(),
        };
        popup.update_filter(filter);
        popup
    }

    pub fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.filtered_commands = COMMANDS
            .iter()
            .filter(|cmd| Self::matches(cmd.name, filter))
            .collect();

        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = self.filtered_commands.len().saturating_sub(1);
        }
    }

    fn matches(command_name: &str, filter: &str) -> bool {
        if filter.is_empty() {
            return true;
        }
        command_name
            .to_lowercase()
            .contains(&filter.to_lowercase())
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered_commands.is_empty() {
            return;
        }

        let len = self.filtered_commands.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
    }

    pub fn selected_command(&self) -> Option<&'static SlashCommand> {
        self.filtered_commands.get(self.selected_index).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.filtered_commands.is_empty()
    }
}

impl Renderable for CommandPopup {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 || self.filtered_commands.is_empty() {
            return;
        }

        let max_rows = area.height as usize;

        for (i, cmd) in self.filtered_commands.iter().take(max_rows).enumerate() {
            let is_selected = i == self.selected_index;
            let y = area.y + i as u16;

            let prefix = if is_selected { "> " } else { "  " };
            let cmd_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let desc_style = Style::default().fg(Color::DarkGray);

            let line = Line::from(vec![
                Span::styled(prefix, cmd_style),
                Span::styled(format!("/{}", cmd.name), cmd_style),
                Span::raw("  "),
                Span::styled(cmd.description, desc_style),
            ]);

            buf.set_line(area.x, y, &line, area.width);
        }
    }

    fn height(&self, _width: u16) -> u16 {
        self.filtered_commands.len().min(5) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_popup_shows_all_commands() {
        let popup = CommandPopup::new("");
        assert_eq!(popup.filtered_commands.len(), COMMANDS.len());
    }

    #[test]
    fn test_filter_narrows_commands() {
        let popup = CommandPopup::new("mo");
        assert_eq!(popup.filtered_commands.len(), 1);
        assert_eq!(popup.filtered_commands[0].name, "model");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let popup = CommandPopup::new("HE");
        assert_eq!(popup.filtered_commands.len(), 1);
        assert_eq!(popup.filtered_commands[0].name, "help");
    }

    #[test]
    fn test_selection_wraps() {
        let mut popup = CommandPopup::new("");
        popup.move_selection(-1);
        assert_eq!(popup.selected_index, COMMANDS.len() - 1);

        popup.move_selection(1);
        assert_eq!(popup.selected_index, 0);
    }

    #[test]
    fn test_selected_command() {
        let popup = CommandPopup::new("");
        assert!(popup.selected_command().is_some());
    }

    #[test]
    fn test_empty_filter_result() {
        let popup = CommandPopup::new("xyz");
        assert!(popup.is_empty());
        assert!(popup.selected_command().is_none());
    }
}
