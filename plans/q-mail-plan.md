# Q-MAIL: Email Client Plan

## Summary

Create a new plugin crate `qdos-plugin-qmail` - a terminal email client with IMAP/SMTP support. Inspired by mutt, alpine, and Mailspring.

## Key Features

1. **IMAP Support** - Connect to mail servers with TLS
2. **Folder Navigation** - Inbox, Sent, Drafts, Archive, custom folders
3. **Message List** - Threaded view with search
4. **Compose** - Write emails with Markdown formatting
5. **Attachments** - View and save attachments
6. **Address Book** - Contact management

## Dependencies

```toml
[dependencies]
qdos-plugin-api = { path = "../qdos-plugin-api" }
inventory = "0.3"
ratatui = "0.29"
crossterm = "0.28"
async-imap = "0.9"       # IMAP client
async-smtp = "0.9"       # SMTP client
lettre = "0.11"          # Alternative SMTP
mailparse = "0.15"       # Email parsing
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
native-tls = "0.2"       # TLS support
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
dirs = "6.0"
keyring = "2"            # Secure credential storage
```

## Crate Structure

```
crates/qdos-plugin-qmail/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin struct, trait impl, key handlers
    ├── state.rs        # QMailState, Message, Folder types
    ├── modal.rs        # UI rendering (list, reader, composer)
    ├── imap.rs         # IMAP connection and operations
    ├── smtp.rs         # SMTP sending
    ├── parser.rs       # Email parsing utilities
    ├── contacts.rs     # Address book
    └── config.rs       # Account configuration
```

## State Design (state.rs)

```rust
pub enum QMailView {
    Accounts,       // Account selection/setup
    FolderList,     // Folder navigation
    MessageList,    // Message list in folder
    MessageRead,    // Reading a message
    Compose,        // Composing new message
    Reply,          // Replying to message
    Search,         // Search interface
    Contacts,       // Address book
    Settings,       // Account settings
    Help,
}

pub struct Account {
    pub name: String,
    pub email: String,
    pub imap_server: String,
    pub imap_port: u16,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    // Password stored in system keyring
    pub use_tls: bool,
}

pub struct Folder {
    pub name: String,
    pub path: String,
    pub unread: u32,
    pub total: u32,
    pub folder_type: FolderType,
}

pub enum FolderType {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Archive,
    Spam,
    Custom,
}

pub struct MessageHeader {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
    pub date: DateTime<Utc>,
    pub is_read: bool,
    pub has_attachment: bool,
    pub thread_id: Option<String>,
}

pub struct Message {
    pub header: MessageHeader,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<Attachment>,
}

pub struct Attachment {
    pub filename: String,
    pub mime_type: String,
    pub size: usize,
    pub data: Option<Vec<u8>>,  // Lazy-loaded
}

pub struct Draft {
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<PathBuf>,
    pub reply_to: Option<u32>,  // UID of message being replied to
}

pub struct Contact {
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
}

pub struct QMailState {
    pub view: QMailView,

    // Accounts
    pub accounts: Vec<Account>,
    pub current_account: Option<usize>,

    // Folders
    pub folders: Vec<Folder>,
    pub current_folder: usize,

    // Messages
    pub messages: Vec<MessageHeader>,
    pub message_cursor: usize,
    pub message_scroll: usize,
    pub current_message: Option<Message>,
    pub message_scroll_offset: usize,

    // Compose
    pub draft: Draft,
    pub compose_field: ComposeField,
    pub compose_cursor: usize,

    // Search
    pub search_query: String,
    pub search_results: Vec<MessageHeader>,

    // Contacts
    pub contacts: Vec<Contact>,
    pub contact_cursor: usize,

    // Connection
    pub connected: bool,
    pub status_message: String,
    pub loading: bool,

    // Cache
    pub message_cache: HashMap<u32, Message>,
}

pub enum ComposeField {
    To,
    Cc,
    Bcc,
    Subject,
    Body,
}
```

## Views

### Message List
```
╔═════════════════════════ Q-MAIL ══════════════════════════════════╗
║ paul@example.com                                    Inbox (5 new) ║
╠═══════════════════════════════════════════════════════════════════╣
║ INBOX                   │ From              Subject          Date ║
║ ────────────────────────┼────────────────────────────────────────║
║ [+] Inbox          (5)  │ * John Smith      Re: Project...  10:42 ║
║ [ ] Sent                │ * Alice Johnson   Meeting tom...  09:15 ║
║ [ ] Drafts         (2)  │ * GitHub          [repo] New PR   08:30 ║
║ [ ] Archive             │   Bob Wilson      Thanks for...   Yesterday║
║ [ ] Trash               │   Newsletter      Weekly upda...  Yesterday║
║ [ ] Spam                │   Support         Ticket #123...  Jan 15 ║
║                         │   HR Dept         Benefits en...  Jan 14 ║
║ Labels:                 │   Alice Johnson   Re: Budget...   Jan 13 ║
║ [ ] Important           │                                          ║
║ [ ] Work                │                                          ║
║ [ ] Personal            │                                          ║
║                         │                                          ║
╠═══════════════════════════════════════════════════════════════════╣
║ Connected | 5 unread | Page 1/3                                   ║
╚═══════════════════════════════════════════════════════════════════╝
 C:Compose  R:Reply  D:Delete  A:Archive  S:Search  /:Filter
```

### Message Reader
```
╔════════════════════════ Q-MAIL ═══════════════════════════════════╗
║ From: John Smith <john@company.com>                               ║
║ To: paul@example.com                                              ║
║ Date: January 17, 2026 10:42 AM                                   ║
║ Subject: Re: Project Update                                       ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║ Hi Paul,                                                          ║
║                                                                   ║
║ Thanks for the update on the project. I've reviewed the latest    ║
║ changes and they look great!                                      ║
║                                                                   ║
║ A few notes:                                                      ║
║                                                                   ║
║ 1. The new feature implementation is solid                        ║
║ 2. Tests are passing in CI                                        ║
║ 3. Let's schedule a demo for next week                            ║
║                                                                   ║
║ Best regards,                                                     ║
║ John                                                              ║
║                                                                   ║
║ Attachments: [1] report.pdf (245 KB)                              ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║ Message 1/8 in Inbox                                              ║
╚═══════════════════════════════════════════════════════════════════╝
 R:Reply  F:Forward  D:Delete  A:Archive  1-9:Open attachment
```

### Compose
```
╔═════════════════════ Q-MAIL: Compose ═════════════════════════════╗
║                                                                   ║
║ To:      [john@company.com, alice@example.com_______]             ║
║ Cc:      [________________________________________________]       ║
║ Bcc:     [________________________________________________]       ║
║ Subject: [Re: Project Update_________________________________]    ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║ Hi John,                                                          ║
║                                                                   ║
║ Thanks for the feedback! I'll schedule the demo for Tuesday       ║
║ afternoon. Does 2 PM work for you?                                ║
║                                                                   ║
║ I've attached the updated timeline for your review.               ║
║                                                                   ║
║ Best,                                                             ║
║ Paul                                                              ║
║ |                                                                 ║
║                                                                   ║
║ Attachments: [1] timeline.xlsx                                    ║
║                                                                   ║
╠═══════════════════════════════════════════════════════════════════╣
║ Insert mode | Ln 12, Col 1                                        ║
╚═══════════════════════════════════════════════════════════════════╝
 Tab:Next field  Ctrl+A:Attach  Ctrl+Enter:Send  Esc:Discard
```

## Key Bindings

### Message List
| Key | Action |
|-----|--------|
| ↑↓/jk | Navigate messages |
| Enter | Read message |
| C | Compose new |
| R | Reply |
| F | Forward |
| D | Delete |
| A | Archive |
| S | Mark as spam |
| U | Toggle read/unread |
| / | Search |
| Tab | Switch folders |
| Esc | Exit |

### Message Reader
| Key | Action |
|-----|--------|
| ↑↓/jk | Scroll message |
| R | Reply |
| F | Forward |
| D | Delete |
| A | Archive |
| 1-9 | Open attachment |
| S | Save attachment |
| Esc | Back to list |

### Compose
| Key | Action |
|-----|--------|
| Tab | Next field |
| Shift+Tab | Previous field |
| Ctrl+A | Add attachment |
| Ctrl+Enter | Send |
| Esc | Discard draft |

## Implementation Phases

### Phase 1: Core Structure
1. Create crate skeleton with Cargo.toml
2. Implement state types (Account, Folder, Message)
3. Implement Plugin trait boilerplate
4. Add to workspace Cargo.toml

### Phase 2: Account Setup
1. Account configuration UI
2. Secure credential storage (keyring)
3. Connection testing

### Phase 3: IMAP Connection
1. Async IMAP connection with TLS
2. Folder listing
3. Message header fetching
4. Message body fetching

### Phase 4: Message List
1. Message list rendering
2. Folder navigation
3. Read/unread indicators
4. Pagination

### Phase 5: Message Reader
1. Message body display
2. HTML to text conversion
3. Attachment listing

### Phase 6: Compose & Send
1. Compose UI with fields
2. Markdown body editing
3. SMTP sending
4. Attachment support

### Phase 7: Polish
1. Search functionality
2. Contact management
3. Threading support
4. Integration with Office Suite

## Security Considerations

1. **Credentials** - Use system keyring, never store plain text
2. **TLS** - Always use TLS for IMAP/SMTP
3. **OAuth2** - Support for Gmail, Outlook (optional, complex)
4. **No eval** - Never execute content from emails

## File Modifications

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `qdos-plugin-qmail` to members |
| `crates/qdos-plugin-qmail/*` | **NEW** - All plugin files |
| `src/plugins/mod.rs` | Add import for QMailPlugin |
| `src/app/mod.rs` | Register QMailPlugin |
| `src/plugins/office/mod.rs` | Add to Office Suite menu |

## Configuration Storage

```
~/.config/rdos/qmail/
├── accounts.json       # Account settings (no passwords!)
├── contacts.json       # Address book
└── cache/
    └── {account}/      # Cached messages per account
```

## Verification

1. `cargo build -p qdos-plugin-qmail` - Plugin compiles
2. Configure test account (use app password for Gmail)
3. Fetch inbox messages
4. Read message with attachments
5. Compose and send test email
6. Quality checks: `cargo fmt -- --check && cargo clippy -- -D warnings`

## Complexity Notes

This is the most complex plugin due to:
- Async networking (IMAP/SMTP)
- Authentication (especially OAuth2)
- Email parsing (MIME, encodings)
- Threading logic
- Attachment handling

Consider starting with basic IMAP/SMTP and adding OAuth2 later.
