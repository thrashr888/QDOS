//! Redis plugin state

/// Main view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RedisView {
    #[default]
    Connect,
    Profiles,
    SaveProfile,
    KeyBrowser,
    KeyDetail,
    ServerInfo,
    Confirm,
    Error,
}

/// Connection profile for Redis
#[derive(Debug, Clone, Default)]
pub struct RedisConnection {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub database: u8,
    pub tls: bool,
}

impl RedisConnection {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            database: 0,
            tls: false,
        }
    }

    /// Build a connection URL
    pub fn to_url(&self) -> String {
        let scheme = if self.tls { "rediss" } else { "redis" };
        if let Some(ref pass) = self.password {
            format!(
                "{}://:{}@{}:{}/{}",
                scheme, pass, self.host, self.port, self.database
            )
        } else {
            format!("{}://{}:{}/{}", scheme, self.host, self.port, self.database)
        }
    }

    /// Display name or host:port
    pub fn display_name(&self) -> String {
        if self.name.is_empty() {
            format!("{}:{}", self.host, self.port)
        } else {
            self.name.clone()
        }
    }
}

/// Redis key type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RedisKeyType {
    #[default]
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    Unknown,
}

impl RedisKeyType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "string" => RedisKeyType::String,
            "list" => RedisKeyType::List,
            "set" => RedisKeyType::Set,
            "zset" => RedisKeyType::ZSet,
            "hash" => RedisKeyType::Hash,
            "stream" => RedisKeyType::Stream,
            _ => RedisKeyType::Unknown,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            RedisKeyType::String => "S",
            RedisKeyType::List => "L",
            RedisKeyType::Set => "s",
            RedisKeyType::ZSet => "Z",
            RedisKeyType::Hash => "H",
            RedisKeyType::Stream => "X",
            RedisKeyType::Unknown => "?",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RedisKeyType::String => "string",
            RedisKeyType::List => "list",
            RedisKeyType::Set => "set",
            RedisKeyType::ZSet => "zset",
            RedisKeyType::Hash => "hash",
            RedisKeyType::Stream => "stream",
            RedisKeyType::Unknown => "unknown",
        }
    }
}

/// Redis key entry
#[derive(Debug, Clone, Default)]
pub struct RedisKey {
    pub name: String,
    pub key_type: RedisKeyType,
    pub ttl: Option<i64>,
    pub memory_bytes: Option<usize>,
}

/// Redis value representation
#[derive(Debug, Clone, Default)]
pub enum RedisValue {
    String(String),
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(Vec<(String, String)>),
    Stream(Vec<String>),
    #[default]
    None,
}

/// Connect form field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectField {
    #[default]
    Host,
    Port,
    Password,
    Database,
    Tls,
    Name,
}

impl ConnectField {
    pub fn next(&self) -> Self {
        match self {
            ConnectField::Host => ConnectField::Port,
            ConnectField::Port => ConnectField::Password,
            ConnectField::Password => ConnectField::Database,
            ConnectField::Database => ConnectField::Tls,
            ConnectField::Tls => ConnectField::Name,
            ConnectField::Name => ConnectField::Host,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ConnectField::Host => ConnectField::Name,
            ConnectField::Port => ConnectField::Host,
            ConnectField::Password => ConnectField::Port,
            ConnectField::Database => ConnectField::Password,
            ConnectField::Tls => ConnectField::Database,
            ConnectField::Name => ConnectField::Tls,
        }
    }
}

/// Key type filter for browsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyTypeFilter {
    #[default]
    All,
    String,
    List,
    Set,
    ZSet,
    Hash,
}

impl KeyTypeFilter {
    pub fn next(&self) -> Self {
        match self {
            KeyTypeFilter::All => KeyTypeFilter::String,
            KeyTypeFilter::String => KeyTypeFilter::List,
            KeyTypeFilter::List => KeyTypeFilter::Set,
            KeyTypeFilter::Set => KeyTypeFilter::ZSet,
            KeyTypeFilter::ZSet => KeyTypeFilter::Hash,
            KeyTypeFilter::Hash => KeyTypeFilter::All,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            KeyTypeFilter::All => "All",
            KeyTypeFilter::String => "String",
            KeyTypeFilter::List => "List",
            KeyTypeFilter::Set => "Set",
            KeyTypeFilter::ZSet => "ZSet",
            KeyTypeFilter::Hash => "Hash",
        }
    }

    pub fn matches(&self, key_type: RedisKeyType) -> bool {
        match self {
            KeyTypeFilter::All => true,
            KeyTypeFilter::String => key_type == RedisKeyType::String,
            KeyTypeFilter::List => key_type == RedisKeyType::List,
            KeyTypeFilter::Set => key_type == RedisKeyType::Set,
            KeyTypeFilter::ZSet => key_type == RedisKeyType::ZSet,
            KeyTypeFilter::Hash => key_type == RedisKeyType::Hash,
        }
    }
}

/// Action requiring confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteKey(String),
    Disconnect,
}

impl std::fmt::Display for ConfirmAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmAction::DeleteKey(name) => write!(f, "Delete key '{}'?", name),
            ConfirmAction::Disconnect => write!(f, "Disconnect from Redis?"),
        }
    }
}

/// Main state container
#[derive(Debug, Clone, Default)]
pub struct RedisState {
    pub view: RedisView,
    pub loading: bool,
    pub loading_message: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,

    // Connection
    pub connected: bool,
    pub connection: RedisConnection,
    pub profiles: Vec<RedisConnection>,
    pub selected_profile: usize,

    // Connect form
    pub connect_field: ConnectField,
    pub connect_cursor: usize,

    // Key browser
    pub keys: Vec<RedisKey>,
    pub filtered_keys: Vec<usize>,
    pub selected_key: usize,
    pub key_scroll: usize,
    pub key_filter: String,
    pub filter_cursor: usize,
    pub type_filter: KeyTypeFilter,

    // Key detail
    pub current_key: Option<RedisKey>,
    pub current_value: RedisValue,
    pub detail_scroll: usize,

    // Server info
    pub server_info: Vec<String>,
    pub info_scroll: usize,

    // SCAN cursor for pagination
    pub scan_cursor: u64,
    pub scan_complete: bool,

    // Confirmation
    pub confirm_action: Option<ConfirmAction>,
}

impl RedisState {
    pub fn new() -> Self {
        Self {
            connection: RedisConnection::new(),
            ..Default::default()
        }
    }

    pub fn set_loading(&mut self, msg: &str) {
        self.loading = true;
        self.loading_message = Some(msg.to_string());
    }

    pub fn clear_loading(&mut self) {
        self.loading = false;
        self.loading_message = None;
    }

    pub fn reset(&mut self) {
        self.view = RedisView::Connect;
        self.loading = false;
        self.loading_message = None;
        self.error = None;
        self.message = None;
        self.connected = false;
        self.keys.clear();
        self.filtered_keys.clear();
        self.current_key = None;
        self.current_value = RedisValue::None;
        self.server_info.clear();
        self.scan_cursor = 0;
        self.scan_complete = false;
        self.confirm_action = None;
    }

    /// Apply key filter and update filtered_keys
    pub fn apply_filter(&mut self) {
        if self.key_filter.is_empty() && self.type_filter == KeyTypeFilter::All {
            self.filtered_keys = (0..self.keys.len()).collect();
        } else {
            self.filtered_keys = self
                .keys
                .iter()
                .enumerate()
                .filter(|(_, k)| {
                    let name_match = self.key_filter.is_empty()
                        || k.name
                            .to_lowercase()
                            .contains(&self.key_filter.to_lowercase());
                    let type_match = self.type_filter.matches(k.key_type);
                    name_match && type_match
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected_key = 0;
        self.key_scroll = 0;
    }

    /// Get visible keys based on filter
    pub fn visible_keys(&self) -> Vec<&RedisKey> {
        self.filtered_keys
            .iter()
            .filter_map(|&i| self.keys.get(i))
            .collect()
    }

    /// Get selected key
    pub fn selected_key(&self) -> Option<&RedisKey> {
        self.filtered_keys
            .get(self.selected_key)
            .and_then(|&i| self.keys.get(i))
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_key > 0 {
            self.selected_key -= 1;
            self.ensure_visible();
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let max = self.filtered_keys.len().saturating_sub(1);
        if self.selected_key < max {
            self.selected_key += 1;
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        let visible_lines = 16;
        if self.selected_key < self.key_scroll {
            self.key_scroll = self.selected_key;
        } else if self.selected_key >= self.key_scroll + visible_lines {
            self.key_scroll = self.selected_key - visible_lines + 1;
        }
    }

    /// Insert character in filter
    pub fn insert_filter_char(&mut self, c: char) {
        self.key_filter.insert(self.filter_cursor, c);
        self.filter_cursor += 1;
        self.apply_filter();
    }

    /// Backspace in filter
    pub fn backspace_filter(&mut self) {
        if self.filter_cursor > 0 {
            self.filter_cursor -= 1;
            self.key_filter.remove(self.filter_cursor);
            self.apply_filter();
        }
    }

    /// Get current field value for connect form
    pub fn get_connect_field_value(&self) -> String {
        match self.connect_field {
            ConnectField::Host => self.connection.host.clone(),
            ConnectField::Port => self.connection.port.to_string(),
            ConnectField::Password => self
                .connection
                .password
                .clone()
                .map(|_| "********".to_string())
                .unwrap_or_default(),
            ConnectField::Database => self.connection.database.to_string(),
            ConnectField::Tls => if self.connection.tls { "Yes" } else { "No" }.to_string(),
            ConnectField::Name => self.connection.name.clone(),
        }
    }

    /// Insert character in current connect field
    pub fn insert_connect_char(&mut self, c: char) {
        match self.connect_field {
            ConnectField::Host => {
                self.connection.host.insert(self.connect_cursor, c);
                self.connect_cursor += 1;
            }
            ConnectField::Port => {
                if c.is_ascii_digit() {
                    let s = self.connection.port.to_string();
                    let mut chars: Vec<char> = s.chars().collect();
                    if self.connect_cursor <= chars.len() {
                        chars.insert(self.connect_cursor, c);
                        if let Ok(port) = chars.iter().collect::<String>().parse::<u16>() {
                            self.connection.port = port;
                            self.connect_cursor += 1;
                        }
                    }
                }
            }
            ConnectField::Password => {
                let pass = self.connection.password.get_or_insert_with(String::new);
                pass.insert(self.connect_cursor, c);
                self.connect_cursor += 1;
            }
            ConnectField::Database => {
                if c.is_ascii_digit() {
                    let s = self.connection.database.to_string();
                    let mut chars: Vec<char> = s.chars().collect();
                    if self.connect_cursor <= chars.len() {
                        chars.insert(self.connect_cursor, c);
                        if let Ok(db) = chars.iter().collect::<String>().parse::<u8>() {
                            if db < 16 {
                                self.connection.database = db;
                                self.connect_cursor += 1;
                            }
                        }
                    }
                }
            }
            ConnectField::Tls => {
                // Toggle on any key
                self.connection.tls = !self.connection.tls;
            }
            ConnectField::Name => {
                self.connection.name.insert(self.connect_cursor, c);
                self.connect_cursor += 1;
            }
        }
    }

    /// Backspace in current connect field
    pub fn backspace_connect(&mut self) {
        if self.connect_cursor > 0 {
            match self.connect_field {
                ConnectField::Host => {
                    self.connect_cursor -= 1;
                    self.connection.host.remove(self.connect_cursor);
                }
                ConnectField::Port => {
                    let s = self.connection.port.to_string();
                    let mut chars: Vec<char> = s.chars().collect();
                    if self.connect_cursor > 0 && self.connect_cursor <= chars.len() {
                        self.connect_cursor -= 1;
                        chars.remove(self.connect_cursor);
                        let new_port: String = chars.into_iter().collect();
                        self.connection.port = new_port.parse().unwrap_or(6379);
                    }
                }
                ConnectField::Password => {
                    if let Some(ref mut pass) = self.connection.password {
                        self.connect_cursor -= 1;
                        pass.remove(self.connect_cursor);
                        if pass.is_empty() {
                            self.connection.password = None;
                        }
                    }
                }
                ConnectField::Database => {
                    // Don't allow empty database
                }
                ConnectField::Tls => {
                    // No-op for toggle
                }
                ConnectField::Name => {
                    self.connect_cursor -= 1;
                    self.connection.name.remove(self.connect_cursor);
                }
            }
        }
    }

    /// Reset cursor when changing fields
    pub fn reset_cursor_for_field(&mut self) {
        self.connect_cursor = match self.connect_field {
            ConnectField::Host => self.connection.host.len(),
            ConnectField::Port => self.connection.port.to_string().len(),
            ConnectField::Password => self
                .connection
                .password
                .as_ref()
                .map(|p| p.len())
                .unwrap_or(0),
            ConnectField::Database => self.connection.database.to_string().len(),
            ConnectField::Tls => 0,
            ConnectField::Name => self.connection.name.len(),
        };
    }
}
