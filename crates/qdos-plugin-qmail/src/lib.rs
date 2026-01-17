//! Q-MAIL: Email Client Plugin for R-DOS
//!
//! A terminal email client with IMAP/SMTP support.
//! Inspired by mutt, alpine, and Mailspring.

mod modal;
mod state;

pub use state::{
    Account, Draft, Folder, FolderType, Message, MessageHeader, QMailState, QMailView,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::{
    AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory, ThemeColors,
};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::fs;
use std::path::PathBuf;

// =============================================================================
// PLUGIN
// =============================================================================

pub struct QMailPlugin {
    state: QMailState,
}

impl Default for QMailPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QMailPlugin {
    pub fn new() -> Self {
        let mut plugin = Self {
            state: QMailState::new(),
        };
        plugin.load_accounts();
        plugin
    }

    // =========================================================================
    // PERSISTENCE
    // =========================================================================

    fn ensure_dirs(&self) {
        if let Some(parent) = self.state.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
    }

    fn load_accounts(&mut self) {
        self.ensure_dirs();

        if self.state.config_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.state.config_path) {
                if let Ok(accounts) = serde_json::from_str(&content) {
                    self.state.accounts = accounts;
                    if !self.state.accounts.is_empty() {
                        self.state.current_account = Some(0);
                        self.state.connected = true;
                        self.state.load_mock_messages();
                        self.state.view = QMailView::FolderList;
                    }
                }
            }
        }
    }

    fn save_accounts(&self) -> Result<(), String> {
        self.ensure_dirs();

        let content =
            serde_json::to_string_pretty(&self.state.accounts).map_err(|e| e.to_string())?;
        fs::write(&self.state.config_path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    // =========================================================================
    // KEY HANDLERS
    // =========================================================================

    fn handle_account_setup_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        use state::AccountSetupField;

        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.state.setup_field = self.state.setup_field.prev();
                } else {
                    self.state.setup_field = self.state.setup_field.next();
                }
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.setup_field = self.state.setup_field.prev();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.save_setup_account();
                if let Err(e) = self.save_accounts() {
                    self.state.status_message = Some(format!("Save failed: {}", e));
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(' ') if self.state.setup_field == AccountSetupField::UseTls => {
                self.state.setup_account.use_tls = !self.state.setup_account.use_tls;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                match self.state.setup_field {
                    AccountSetupField::Name => {
                        self.state.setup_account.name.pop();
                    }
                    AccountSetupField::Email => {
                        self.state.setup_account.email.pop();
                    }
                    AccountSetupField::ImapServer => {
                        self.state.setup_account.imap_server.pop();
                    }
                    AccountSetupField::ImapPort => {
                        let s = self.state.setup_account.imap_port.to_string();
                        if !s.is_empty() {
                            let new_s: String = s.chars().take(s.len() - 1).collect();
                            self.state.setup_account.imap_port = new_s.parse().unwrap_or(993);
                        }
                    }
                    AccountSetupField::SmtpServer => {
                        self.state.setup_account.smtp_server.pop();
                    }
                    AccountSetupField::SmtpPort => {
                        let s = self.state.setup_account.smtp_port.to_string();
                        if !s.is_empty() {
                            let new_s: String = s.chars().take(s.len() - 1).collect();
                            self.state.setup_account.smtp_port = new_s.parse().unwrap_or(587);
                        }
                    }
                    AccountSetupField::Username => {
                        self.state.setup_account.username.pop();
                    }
                    AccountSetupField::Password => {
                        self.state.password_buffer.pop();
                    }
                    AccountSetupField::UseTls => {}
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                match self.state.setup_field {
                    AccountSetupField::Name => {
                        self.state.setup_account.name.push(c);
                    }
                    AccountSetupField::Email => {
                        self.state.setup_account.email.push(c);
                    }
                    AccountSetupField::ImapServer => {
                        self.state.setup_account.imap_server.push(c);
                    }
                    AccountSetupField::ImapPort => {
                        if c.is_ascii_digit() {
                            let mut s = self.state.setup_account.imap_port.to_string();
                            if s == "0" {
                                s.clear();
                            }
                            s.push(c);
                            self.state.setup_account.imap_port = s.parse().unwrap_or(993);
                        }
                    }
                    AccountSetupField::SmtpServer => {
                        self.state.setup_account.smtp_server.push(c);
                    }
                    AccountSetupField::SmtpPort => {
                        if c.is_ascii_digit() {
                            let mut s = self.state.setup_account.smtp_port.to_string();
                            if s == "0" {
                                s.clear();
                            }
                            s.push(c);
                            self.state.setup_account.smtp_port = s.parse().unwrap_or(587);
                        }
                    }
                    AccountSetupField::Username => {
                        self.state.setup_account.username.push(c);
                    }
                    AccountSetupField::Password => {
                        self.state.password_buffer.push(c);
                    }
                    AccountSetupField::UseTls => {
                        if c == ' ' {
                            self.state.setup_account.use_tls = !self.state.setup_account.use_tls;
                        }
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_folder_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => KeyHandleResult::CloseModal,
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.folder_cursor_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.folder_cursor_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.open_folder();
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.state.start_compose();
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QMailView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_message_list_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QMailView::FolderList;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.message_cursor_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.message_cursor_down();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                self.state.open_message();
                KeyHandleResult::Handled
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.state.start_compose();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.delete_message();
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.state.archive_message();
                KeyHandleResult::Handled
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.state.toggle_read();
                KeyHandleResult::Handled
            }
            KeyCode::Char('?') => {
                self.state.view = QMailView::Help;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_message_read_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.current_message = None;
                self.state.view = QMailView::MessageList;
                KeyHandleResult::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.scroll_message_up();
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.scroll_message_down();
                KeyHandleResult::Handled
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.state.start_reply();
                KeyHandleResult::Handled
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.delete_message();
                self.state.view = QMailView::MessageList;
                KeyHandleResult::Handled
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.state.archive_message();
                self.state.view = QMailView::MessageList;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_compose_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.draft.clear();
                self.state.view = QMailView::FolderList;
                KeyHandleResult::Handled
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.state.compose_field = self.state.compose_field.prev();
                } else {
                    self.state.compose_field = self.state.compose_field.next();
                }
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                self.state.compose_field = self.state.compose_field.prev();
                KeyHandleResult::Handled
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                match self.state.send_message() {
                    Ok(()) => KeyHandleResult::CloseWithSuccess("Message sent".to_string()),
                    Err(e) => {
                        self.state.status_message = Some(e);
                        KeyHandleResult::Handled
                    }
                }
            }
            KeyCode::Enter => {
                // In body field, add newline
                if self.state.compose_field == state::ComposeField::Body {
                    self.state.draft.body.push('\n');
                }
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                match self.state.compose_field {
                    state::ComposeField::To => {
                        self.state.draft.to.pop();
                    }
                    state::ComposeField::Subject => {
                        self.state.draft.subject.pop();
                    }
                    state::ComposeField::Body => {
                        self.state.draft.body.pop();
                    }
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                match self.state.compose_field {
                    state::ComposeField::To => {
                        self.state.draft.to.push(c);
                    }
                    state::ComposeField::Subject => {
                        self.state.draft.subject.push(c);
                    }
                    state::ComposeField::Body => {
                        self.state.draft.body.push(c);
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        match key.code {
            KeyCode::Esc => {
                self.state.view = QMailView::FolderList;
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for QMailPlugin {
    fn id(&self) -> &str {
        "qmail"
    }

    fn name(&self) -> &str {
        "Q-MAIL"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_menu: false,
            has_keys: true,
            has_modal: true,
            has_status: false,
            has_cli: false,
            has_help: true,
        }
    }

    fn is_available(&self, _cwd: &PathBuf) -> bool {
        true
    }

    fn app_entry(&self) -> Option<AppEntry> {
        Some(AppEntry {
            id: self.id().to_string(),
            name: "Q-MAIL".to_string(),
            description: "Email client with IMAP/SMTP".to_string(),
            category: PluginCategory::Tools,
            key: 'M',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        // If we have accounts, go to folder list; otherwise, show setup
        if self.state.accounts.is_empty() {
            self.state.view = QMailView::AccountSetup;
            self.state.setup_field = state::AccountSetupField::Name;
        } else {
            self.state.view = QMailView::FolderList;
            self.state.folder_cursor = 0;
            self.state.load_mock_messages();
        }
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        // Q-MAIL is launched via Apps menu (F12) which calls launch()
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        match self.state.view {
            QMailView::AccountSetup => self.handle_account_setup_key(key),
            QMailView::FolderList => self.handle_folder_list_key(key),
            QMailView::MessageList => self.handle_message_list_key(key),
            QMailView::MessageRead => self.handle_message_read_key(key),
            QMailView::Compose => self.handle_compose_key(key),
            QMailView::Help => self.handle_help_key(key),
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        modal::draw_qmail(&self.state, frame, area, colors);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-MAIL - Email Client".to_string(),
            "".to_string(),
            "A terminal email client with IMAP/SMTP support.".to_string(),
            "".to_string(),
            "Folder List:".to_string(),
            "  Up/Down    Navigate folders".to_string(),
            "  Enter      Open folder".to_string(),
            "  C          Compose new message".to_string(),
            "".to_string(),
            "Message List:".to_string(),
            "  Up/Down    Navigate messages".to_string(),
            "  Enter      Read message".to_string(),
            "  D          Delete message".to_string(),
            "  A          Archive message".to_string(),
            "  U          Toggle read/unread".to_string(),
            "".to_string(),
            "Compose:".to_string(),
            "  Tab        Next field".to_string(),
            "  Ctrl+Enter Send message".to_string(),
            "  Esc        Discard draft".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
