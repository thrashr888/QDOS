//! Q-MAIL state and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// VIEWS
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QMailView {
    #[default]
    AccountSetup,
    FolderList,
    MessageList,
    MessageRead,
    Compose,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountSetupField {
    #[default]
    Name,
    Email,
    ImapServer,
    ImapPort,
    SmtpServer,
    SmtpPort,
    Username,
    Password,
    UseTls,
}

impl AccountSetupField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Email,
            Self::Email => Self::ImapServer,
            Self::ImapServer => Self::ImapPort,
            Self::ImapPort => Self::SmtpServer,
            Self::SmtpServer => Self::SmtpPort,
            Self::SmtpPort => Self::Username,
            Self::Username => Self::Password,
            Self::Password => Self::UseTls,
            Self::UseTls => Self::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::UseTls,
            Self::Email => Self::Name,
            Self::ImapServer => Self::Email,
            Self::ImapPort => Self::ImapServer,
            Self::SmtpServer => Self::ImapPort,
            Self::SmtpPort => Self::SmtpServer,
            Self::Username => Self::SmtpPort,
            Self::Password => Self::Username,
            Self::UseTls => Self::Password,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComposeField {
    #[default]
    To,
    Subject,
    Body,
}

impl ComposeField {
    pub fn next(self) -> Self {
        match self {
            Self::To => Self::Subject,
            Self::Subject => Self::Body,
            Self::Body => Self::To,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::To => Self::Body,
            Self::Subject => Self::To,
            Self::Body => Self::Subject,
        }
    }
}

// =============================================================================
// ACCOUNT
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub email: String,
    pub imap_server: String,
    pub imap_port: u16,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub use_tls: bool,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            name: String::new(),
            email: String::new(),
            // Gmail defaults
            imap_server: "imap.gmail.com".to_string(),
            imap_port: 993,
            smtp_server: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            username: String::new(),
            use_tls: true,
        }
    }
}

impl Account {
    pub fn is_configured(&self) -> bool {
        !self.name.is_empty()
            && !self.email.is_empty()
            && !self.imap_server.is_empty()
            && !self.smtp_server.is_empty()
            && !self.username.is_empty()
    }
}

// =============================================================================
// FOLDER
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderType {
    Inbox,
    Sent,
    Drafts,
    Archive,
    Trash,
    Spam,
}

impl FolderType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inbox => "INBOX",
            Self::Sent => "Sent",
            Self::Drafts => "Drafts",
            Self::Archive => "Archive",
            Self::Trash => "Trash",
            Self::Spam => "Spam",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Folder {
    pub name: String,
    pub folder_type: FolderType,
    pub unread: u32,
    pub total: u32,
}

impl Folder {
    pub fn new(folder_type: FolderType) -> Self {
        Self {
            name: folder_type.name().to_string(),
            folder_type,
            unread: 0,
            total: 0,
        }
    }
}

// =============================================================================
// MESSAGE
// =============================================================================

#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub date: DateTime<Utc>,
    pub is_read: bool,
}

impl MessageHeader {
    pub fn mock(uid: u32, from: &str, subject: &str, days_ago: i64, is_read: bool) -> Self {
        let date = Utc::now() - chrono::Duration::days(days_ago);
        Self {
            uid,
            subject: subject.to_string(),
            from: from.to_string(),
            date,
            is_read,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub body: String,
}

impl Message {
    pub fn mock(uid: u32, from: &str, subject: &str, body: &str, days_ago: i64) -> Self {
        Self {
            header: MessageHeader::mock(uid, from, subject, days_ago, false),
            body: body.to_string(),
        }
    }
}

// =============================================================================
// DRAFT
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl Draft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.to.clear();
        self.subject.clear();
        self.body.clear();
    }
}

// =============================================================================
// STATE
// =============================================================================

#[derive(Debug)]
pub struct QMailState {
    pub view: QMailView,

    // Account setup
    pub accounts: Vec<Account>,
    pub current_account: Option<usize>,
    pub setup_field: AccountSetupField,
    pub setup_account: Account,
    pub password_buffer: String,

    // Folders
    pub folders: Vec<Folder>,
    pub folder_cursor: usize,

    // Messages
    pub messages: Vec<MessageHeader>,
    pub message_cursor: usize,
    pub message_scroll: usize,
    pub current_message: Option<Message>,
    pub message_scroll_offset: usize,

    // Compose
    pub draft: Draft,
    pub compose_field: ComposeField,

    // Connection
    pub connected: bool,
    pub status_message: Option<String>,

    // File paths
    pub config_path: PathBuf,
}

impl Default for QMailState {
    fn default() -> Self {
        Self::new()
    }
}

impl QMailState {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rdos")
            .join("qmail");

        let mut state = Self {
            view: QMailView::AccountSetup,
            accounts: Vec::new(),
            current_account: None,
            setup_field: AccountSetupField::Name,
            setup_account: Account::default(),
            password_buffer: String::new(),
            folders: Vec::new(),
            folder_cursor: 0,
            messages: Vec::new(),
            message_cursor: 0,
            message_scroll: 0,
            current_message: None,
            message_scroll_offset: 0,
            draft: Draft::new(),
            compose_field: ComposeField::To,
            connected: false,
            status_message: None,
            config_path: config_dir.join("accounts.json"),
        };

        state.init_folders();
        state
    }

    // =========================================================================
    // INITIALIZATION
    // =========================================================================

    fn init_folders(&mut self) {
        self.folders = vec![
            Folder::new(FolderType::Inbox),
            Folder::new(FolderType::Sent),
            Folder::new(FolderType::Drafts),
            Folder::new(FolderType::Archive),
            Folder::new(FolderType::Trash),
            Folder::new(FolderType::Spam),
        ];

        // Set some mock unread counts
        if let Some(inbox) = self.folders.first_mut() {
            inbox.unread = 3;
            inbox.total = 8;
        }
    }

    pub fn load_mock_messages(&mut self) {
        self.messages = vec![
            MessageHeader::mock(
                1,
                "John Smith <john@company.com>",
                "Re: Project Update",
                0,
                false,
            ),
            MessageHeader::mock(
                2,
                "Alice Johnson <alice@example.com>",
                "Meeting tomorrow",
                0,
                false,
            ),
            MessageHeader::mock(
                3,
                "GitHub <noreply@github.com>",
                "[repo] New PR: Fix bug",
                0,
                false,
            ),
            MessageHeader::mock(
                4,
                "Bob Wilson <bob@example.com>",
                "Thanks for your help!",
                1,
                true,
            ),
            MessageHeader::mock(
                5,
                "Newsletter <news@tech.com>",
                "Weekly tech digest",
                1,
                true,
            ),
            MessageHeader::mock(
                6,
                "Support <support@service.com>",
                "Ticket #123 resolved",
                2,
                true,
            ),
            MessageHeader::mock(
                7,
                "HR Dept <hr@company.com>",
                "Benefits enrollment reminder",
                3,
                true,
            ),
            MessageHeader::mock(
                8,
                "Alice Johnson <alice@example.com>",
                "Re: Budget discussion",
                4,
                true,
            ),
        ];
    }

    pub fn get_mock_message(&self, uid: u32) -> Option<Message> {
        match uid {
            1 => Some(Message::mock(
                1,
                "John Smith <john@company.com>",
                "Re: Project Update",
                "Hi,\n\nThanks for the update on the project. I've reviewed the latest\nchanges and they look great!\n\nA few notes:\n\n1. The new feature implementation is solid\n2. Tests are passing in CI\n3. Let's schedule a demo for next week\n\nBest regards,\nJohn",
                0,
            )),
            2 => Some(Message::mock(
                2,
                "Alice Johnson <alice@example.com>",
                "Meeting tomorrow",
                "Hi,\n\nJust a reminder that we have a meeting scheduled for tomorrow\nat 2 PM in Conference Room B.\n\nAgenda:\n- Q4 review\n- 2024 planning\n- Budget discussion\n\nSee you there!\n\nBest,\nAlice",
                0,
            )),
            3 => Some(Message::mock(
                3,
                "GitHub <noreply@github.com>",
                "[repo] New PR: Fix bug",
                "@username opened a new pull request:\n\n#42 Fix critical bug in authentication\n\nThis PR fixes the authentication issue reported in #41.\n\nChanges:\n- Updated auth middleware\n- Added rate limiting\n- Fixed token refresh logic\n\nPlease review at your earliest convenience.",
                0,
            )),
            _ => None,
        }
    }

    // =========================================================================
    // ACCOUNT OPERATIONS
    // =========================================================================

    pub fn current_account(&self) -> Option<&Account> {
        self.current_account.and_then(|i| self.accounts.get(i))
    }

    pub fn save_setup_account(&mut self) {
        if self.setup_account.is_configured() {
            self.accounts.push(self.setup_account.clone());
            self.current_account = Some(self.accounts.len() - 1);
            self.setup_account = Account::default();
            self.password_buffer.clear();
            self.connected = true;
            self.load_mock_messages();
            self.view = QMailView::FolderList;
            self.status_message = Some("Account configured (mock mode)".to_string());
        }
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    pub fn folder_cursor_up(&mut self) {
        if self.folder_cursor > 0 {
            self.folder_cursor -= 1;
        }
    }

    pub fn folder_cursor_down(&mut self) {
        if self.folder_cursor + 1 < self.folders.len() {
            self.folder_cursor += 1;
        }
    }

    pub fn message_cursor_up(&mut self) {
        if self.message_cursor > 0 {
            self.message_cursor -= 1;
        }
    }

    pub fn message_cursor_down(&mut self) {
        if self.message_cursor + 1 < self.messages.len() {
            self.message_cursor += 1;
        }
    }

    pub fn scroll_message_up(&mut self) {
        if self.message_scroll_offset > 0 {
            self.message_scroll_offset -= 1;
        }
    }

    pub fn scroll_message_down(&mut self) {
        self.message_scroll_offset += 1;
    }

    // =========================================================================
    // ACTIONS
    // =========================================================================

    pub fn open_folder(&mut self) {
        // For now, always shows inbox messages
        self.view = QMailView::MessageList;
        self.message_cursor = 0;
    }

    pub fn open_message(&mut self) {
        if let Some(header) = self.messages.get(self.message_cursor) {
            if let Some(message) = self.get_mock_message(header.uid) {
                self.current_message = Some(message);
                self.message_scroll_offset = 0;
                self.view = QMailView::MessageRead;

                // Mark as read
                if let Some(h) = self.messages.get_mut(self.message_cursor) {
                    h.is_read = true;
                }
            }
        }
    }

    pub fn start_compose(&mut self) {
        self.draft.clear();
        self.compose_field = ComposeField::To;
        self.view = QMailView::Compose;
    }

    pub fn start_reply(&mut self) {
        if let Some(msg) = &self.current_message {
            self.draft.to = msg.header.from.clone();
            self.draft.subject = format!("Re: {}", msg.header.subject);
            self.draft.body = format!(
                "\n\n--- Original Message ---\nFrom: {}\nDate: {}\n\n{}",
                msg.header.from,
                msg.header.date.format("%Y-%m-%d %H:%M"),
                msg.body
            );
            self.compose_field = ComposeField::Body;
            self.view = QMailView::Compose;
        }
    }

    pub fn send_message(&mut self) -> Result<(), String> {
        if self.draft.to.is_empty() {
            return Err("Recipient is required".to_string());
        }
        if self.draft.subject.is_empty() {
            return Err("Subject is required".to_string());
        }

        // In mock mode, just clear the draft and return to folder list
        self.draft.clear();
        self.view = QMailView::FolderList;
        self.status_message = Some("Message sent (mock mode)".to_string());
        Ok(())
    }

    pub fn delete_message(&mut self) {
        if !self.messages.is_empty() {
            self.messages.remove(self.message_cursor);
            if self.message_cursor >= self.messages.len() && self.message_cursor > 0 {
                self.message_cursor -= 1;
            }
            self.status_message = Some("Message deleted".to_string());
        }
    }

    pub fn archive_message(&mut self) {
        if !self.messages.is_empty() {
            self.messages.remove(self.message_cursor);
            if self.message_cursor >= self.messages.len() && self.message_cursor > 0 {
                self.message_cursor -= 1;
            }
            self.status_message = Some("Message archived".to_string());
        }
    }

    pub fn toggle_read(&mut self) {
        if let Some(msg) = self.messages.get_mut(self.message_cursor) {
            msg.is_read = !msg.is_read;
        }
    }
}
