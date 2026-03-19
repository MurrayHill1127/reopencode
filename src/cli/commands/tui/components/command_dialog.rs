//! Command Dialog - searchable command palette
//!
//! Displays a searchable list of available commands with categories.
//! Activated by Ctrl+P (command_list keybind).

use super::dialog::{SelectDialog, SelectOption};
use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use std::time::Duration;

pub type CommandAction = Box<dyn FnMut() + Send + Sync>;

pub struct CommandEntry {
    pub title: String,
    pub value: String,
    pub category: Option<String>,
    pub keybind: Option<String>,
    pub action: Option<CommandAction>,
}

impl CommandEntry {
    pub fn new(title: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            category: None,
            keybind: None,
            action: None,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_keybind(mut self, keybind: impl Into<String>) -> Self {
        self.keybind = Some(keybind.into());
        self
    }

    pub fn with_action(mut self, action: CommandAction) -> Self {
        self.action = Some(action);
        self
    }

    fn to_option(&self) -> SelectOption<String> {
        let mut opt = SelectOption::new(&self.title, self.value.clone());
        if let Some(cat) = &self.category {
            opt = opt.with_category(cat);
        }
        if let Some(kb) = &self.keybind {
            opt = opt.with_footer(kb);
        }
        opt
    }
}

pub struct CommandDialog {
    id: ComponentId,
    dialog: SelectDialog<String>,
    commands: Vec<CommandEntry>,
}

impl CommandDialog {
    pub fn new() -> Self {
        let mut dialog = SelectDialog::new("Commands");
        dialog.set_title(" Commands ");
        Self {
            id: ComponentId::new(),
            dialog,
            commands: Vec::new(),
        }
    }

    pub fn with_commands(commands: Vec<CommandEntry>) -> Self {
        let options: Vec<SelectOption<String>> = commands.iter().map(|c| c.to_option()).collect();
        let mut dialog = SelectDialog::with_options("Commands", options);
        dialog.set_title(" Commands ");
        Self {
            id: ComponentId::new(),
            dialog,
            commands,
        }
    }

    pub fn set_commands(&mut self, commands: Vec<CommandEntry>) {
        self.commands = commands;
        let options: Vec<SelectOption<String>> =
            self.commands.iter().map(|c| c.to_option()).collect();
        self.dialog = SelectDialog::with_options("Commands", options);
        self.dialog.set_title(" Commands ");
    }

    pub fn is_visible(&self) -> bool {
        self.dialog.is_visible()
    }

    pub fn show(&mut self) {
        self.dialog.show();
    }

    pub fn hide(&mut self) {
        self.dialog.hide();
    }

    pub fn get_selected(&self) -> Option<&CommandEntry> {
        self.dialog
            .get_selected_original_index()
            .and_then(|idx| self.commands.get(idx))
    }

    pub fn execute_selected(&mut self) -> bool {
        let idx = self.dialog.get_selected_original_index();
        if let Some(idx) = idx {
            if let Some(cmd) = self.commands.get_mut(idx) {
                if let Some(ref mut action) = cmd.action {
                    action();
                    return true;
                }
            }
        }
        false
    }

    pub fn clear_filter(&mut self) {
        self.dialog.clear_filter();
    }
}

impl Default for CommandDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CommandDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        self.dialog.render(frame, area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.is_visible() {
            return EventPropagation::Continue;
        }

        match event.code {
            KeyCode::Esc => {
                self.hide();
                EventPropagation::Stop
            }
            KeyCode::Enter => {
                self.execute_selected();
                self.hide();
                EventPropagation::Stop
            }
            _ => self.dialog.handle_input(event),
        }
    }

    fn update(&mut self, _delta: Duration) {}

    fn is_focusable(&self) -> bool {
        true
    }

    fn on_focus(&mut self) {
        self.dialog.on_focus();
    }

    fn on_blur(&mut self) {
        self.dialog.on_blur();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_dialog_new() {
        let dialog = CommandDialog::new();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_command_entry_to_option() {
        let entry = CommandEntry::new("New Session", "session.new")
            .with_category("Session")
            .with_keybind("ctrl+n");

        let opt = entry.to_option();
        assert_eq!(opt.title(), "New Session");
        assert_eq!(opt.value(), &"session.new".to_string());
        assert_eq!(opt.category(), Some("Session"));
        assert_eq!(opt.footer(), Some("ctrl+n"));
    }

    #[test]
    fn test_command_dialog_visibility() {
        let mut dialog = CommandDialog::new();
        assert!(!dialog.is_visible());
        dialog.show();
        assert!(dialog.is_visible());
        dialog.hide();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_command_dialog_with_commands() {
        let commands = vec![
            CommandEntry::new("New Session", "session.new"),
            CommandEntry::new("Exit", "app.exit"),
        ];
        let dialog = CommandDialog::with_commands(commands);
        assert_eq!(dialog.commands.len(), 2);
    }
}
