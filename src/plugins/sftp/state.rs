//! SFTP Plugin State Types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Current view in the SFTP plugin
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SftpView {
    #[default]
    /// Connection list (saved connections)
    Connections,
    /// Connection form (new/edit connection)
    Connect,
    /// Remote file browser
    Browser,
    /// Transfer progress
    Transfer,
    /// Save connection as profile
    SaveProfile,
    /// Error state
    Error,
}

/// Authentication method for SFTP
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AuthMethod {
    #[default]
    /// Use SSH key from default location (~/.ssh/id_rsa)
    DefaultKey,
    /// Use specific SSH key file
    KeyFile(String),
    /// Use password authentication
    Password,
    /// Use SSH agent
    Agent,
}

/// SFTP connection configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SftpConnection {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    pub auth_method: AuthMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
    /// Default remote directory to open
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub default_path: String,
}

impl SftpConnection {
    pub fn new() -> Self {
        Self {
            host: String::new(),
            port: 22,
            username: whoami::username(),
            password: String::new(),
            auth_method: AuthMethod::DefaultKey,
            key_file: None,
            default_path: String::new(),
        }
    }

    /// Get display string for connection
    pub fn display(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
}

/// Saved connection profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SftpProfile {
    pub name: String,
    pub connection: SftpConnection,
}

/// SFTP plugin configuration (saved to config file)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SftpPluginConfig {
    #[serde(default)]
    pub profiles: Vec<SftpProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connection: Option<SftpConnection>,
}

/// Remote file entry
#[derive(Debug, Clone)]
pub struct SftpFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<i64>,
    pub permissions: u32,
}

impl SftpFileEntry {
    /// Format permissions as rwxrwxrwx string
    pub fn permissions_string(&self) -> String {
        let perms = self.permissions;
        let mut s = String::with_capacity(9);

        // Owner
        s.push(if perms & 0o400 != 0 { 'r' } else { '-' });
        s.push(if perms & 0o200 != 0 { 'w' } else { '-' });
        s.push(if perms & 0o100 != 0 { 'x' } else { '-' });

        // Group
        s.push(if perms & 0o040 != 0 { 'r' } else { '-' });
        s.push(if perms & 0o020 != 0 { 'w' } else { '-' });
        s.push(if perms & 0o010 != 0 { 'x' } else { '-' });

        // Other
        s.push(if perms & 0o004 != 0 { 'r' } else { '-' });
        s.push(if perms & 0o002 != 0 { 'w' } else { '-' });
        s.push(if perms & 0o001 != 0 { 'x' } else { '-' });

        s
    }
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Download,
    Upload,
}

/// Transfer status
#[derive(Debug, Clone)]
pub struct TransferStatus {
    pub direction: TransferDirection,
    pub filename: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub complete: bool,
    pub error: Option<String>,
}

impl TransferStatus {
    pub fn progress_percent(&self) -> u8 {
        if self.total_bytes == 0 {
            return 0;
        }
        ((self.transferred_bytes * 100) / self.total_bytes) as u8
    }
}

/// Connection form field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectField {
    #[default]
    Host,
    Port,
    Username,
    Password,
    AuthMethod,
    KeyFile,
    DefaultPath,
}

/// SFTP plugin state
#[derive(Debug, Default)]
pub struct SftpState {
    /// Current view
    pub view: SftpView,
    /// Saved connection profiles
    pub profiles: Vec<SftpProfile>,
    /// Selected profile index
    pub selected_profile: usize,
    /// Current connection config (for editing/connecting)
    pub connection: SftpConnection,
    /// Currently selected connection field
    pub connect_field: ConnectField,
    /// Profile name being entered (for save)
    pub profile_name: String,
    /// Is connected
    pub connected: bool,
    /// Current remote directory
    pub current_dir: String,
    /// Remote files in current directory
    pub files: Vec<SftpFileEntry>,
    /// Selected file index
    pub selected_file: usize,
    /// Scroll offset for file list
    pub scroll_offset: usize,
    /// Local directory for transfers
    pub local_dir: PathBuf,
    /// Current transfer status
    pub transfer: Option<TransferStatus>,
    /// Error message
    pub error: Option<String>,
    /// Success message
    pub message: Option<String>,
}

impl SftpState {
    pub fn new() -> Self {
        Self {
            connection: SftpConnection::new(),
            current_dir: "/".to_string(),
            local_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ..Default::default()
        }
    }

    /// Reset state for new connection
    pub fn reset(&mut self) {
        self.view = SftpView::Connections;
        self.connected = false;
        self.current_dir = "/".to_string();
        self.files.clear();
        self.selected_file = 0;
        self.scroll_offset = 0;
        self.transfer = None;
        self.error = None;
        self.message = None;
    }

    /// Select next profile
    pub fn select_next_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = (self.selected_profile + 1) % self.profiles.len();
        }
    }

    /// Select previous profile
    pub fn select_prev_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile = self
                .selected_profile
                .checked_sub(1)
                .unwrap_or(self.profiles.len() - 1);
        }
    }

    /// Get selected profile
    pub fn selected_profile(&self) -> Option<&SftpProfile> {
        self.profiles.get(self.selected_profile)
    }

    /// Delete selected profile
    pub fn delete_selected_profile(&mut self) {
        if !self.profiles.is_empty() && self.selected_profile < self.profiles.len() {
            self.profiles.remove(self.selected_profile);
            if self.selected_profile >= self.profiles.len() && self.selected_profile > 0 {
                self.selected_profile -= 1;
            }
        }
    }

    /// Select next file
    pub fn select_next_file(&mut self) {
        if !self.files.is_empty() {
            self.selected_file = (self.selected_file + 1) % self.files.len();
        }
    }

    /// Select previous file
    pub fn select_prev_file(&mut self) {
        if !self.files.is_empty() {
            self.selected_file = self
                .selected_file
                .checked_sub(1)
                .unwrap_or(self.files.len() - 1);
        }
    }

    /// Get selected file
    pub fn selected_file(&self) -> Option<&SftpFileEntry> {
        self.files.get(self.selected_file)
    }

    /// Move to next connection field
    pub fn next_connect_field(&mut self) {
        self.connect_field = match self.connect_field {
            ConnectField::Host => ConnectField::Port,
            ConnectField::Port => ConnectField::Username,
            ConnectField::Username => ConnectField::AuthMethod,
            ConnectField::AuthMethod => match self.connection.auth_method {
                AuthMethod::Password => ConnectField::Password,
                AuthMethod::KeyFile(_) => ConnectField::KeyFile,
                _ => ConnectField::DefaultPath,
            },
            ConnectField::Password => ConnectField::DefaultPath,
            ConnectField::KeyFile => ConnectField::DefaultPath,
            ConnectField::DefaultPath => ConnectField::Host,
        };
    }

    /// Move to previous connection field
    pub fn prev_connect_field(&mut self) {
        self.connect_field = match self.connect_field {
            ConnectField::Host => ConnectField::DefaultPath,
            ConnectField::Port => ConnectField::Host,
            ConnectField::Username => ConnectField::Port,
            ConnectField::AuthMethod => ConnectField::Username,
            ConnectField::Password => ConnectField::AuthMethod,
            ConnectField::KeyFile => ConnectField::AuthMethod,
            ConnectField::DefaultPath => match self.connection.auth_method {
                AuthMethod::Password => ConnectField::Password,
                AuthMethod::KeyFile(_) => ConnectField::KeyFile,
                _ => ConnectField::AuthMethod,
            },
        };
    }

    /// Cycle auth method
    pub fn cycle_auth_method(&mut self) {
        self.connection.auth_method = match self.connection.auth_method {
            AuthMethod::DefaultKey => AuthMethod::Agent,
            AuthMethod::Agent => AuthMethod::Password,
            AuthMethod::Password => AuthMethod::KeyFile(String::new()),
            AuthMethod::KeyFile(_) => AuthMethod::DefaultKey,
        };
    }

    /// Insert character into current connection field
    pub fn insert_connect_char(&mut self, c: char) {
        match self.connect_field {
            ConnectField::Host => self.connection.host.push(c),
            ConnectField::Port => {
                if c.is_ascii_digit() {
                    let mut s = self.connection.port.to_string();
                    s.push(c);
                    if let Ok(p) = s.parse::<u16>() {
                        self.connection.port = p;
                    }
                }
            }
            ConnectField::Username => self.connection.username.push(c),
            ConnectField::Password => self.connection.password.push(c),
            ConnectField::KeyFile => {
                if let AuthMethod::KeyFile(ref mut path) = self.connection.auth_method {
                    path.push(c);
                }
            }
            ConnectField::DefaultPath => self.connection.default_path.push(c),
            ConnectField::AuthMethod => {} // Handled by cycle
        }
    }

    /// Backspace on current connection field
    pub fn backspace_connect(&mut self) {
        match self.connect_field {
            ConnectField::Host => {
                self.connection.host.pop();
            }
            ConnectField::Port => {
                let mut s = self.connection.port.to_string();
                s.pop();
                self.connection.port = s.parse().unwrap_or(22);
            }
            ConnectField::Username => {
                self.connection.username.pop();
            }
            ConnectField::Password => {
                self.connection.password.pop();
            }
            ConnectField::KeyFile => {
                if let AuthMethod::KeyFile(ref mut path) = self.connection.auth_method {
                    path.pop();
                }
            }
            ConnectField::DefaultPath => {
                self.connection.default_path.pop();
            }
            ConnectField::AuthMethod => {}
        }
    }
}
