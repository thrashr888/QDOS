# Q-DOS Office Suite Specification

A comprehensive productivity suite for R-DOS, bringing classic DOS-era office applications into the modern terminal with AI enhancements.

## Overview

The Q-DOS Office Suite extends R-DOS with 10 productivity applications that share common infrastructure, integrate with existing plugins, and leverage AI where appropriate.

### Suite Applications (Priority Order)

| Rank | App | Type | Description |
|------|-----|------|-------------|
| 1 | Q-SHEET | Spreadsheet | VisiCalc/Lotus 1-2-3 style CSV editor with formulas |
| 2 | Q-DECK | Presentation | ANSI/ASCII slideshow editor with templates |
| 3 | Q-WEB | Browser | Lynx-style reader mode web viewer |
| 4 | Q-DOCS | Word Processor | WordPerfect-style document editor (MD/DOC) |
| 5 | Q-CODE | IDE | VIM-style code editor with LSP support |
| 6 | Q-PAINT | Graphics | DeluxePaint-style pixel art editor |
| 7 | Q-MAIL | Email | Pine/Mutt-style email client |
| 8 | Q-FORM | Forms | Form Master-style form designer |
| 9 | Q-DESIGN | Publishing | PrintMaster Gold-style card/banner creator |
| 10 | Q-MIDI | Music | MIDI sequencer with tracker interface |

---

## Shared Infrastructure

### 1. File Operations (`src/office/file.rs`)

Common file handling across all office apps:

```rust
pub trait OfficeDocument {
    /// File extensions this document type supports
    fn extensions() -> &'static [&'static str];

    /// Create new empty document
    fn new() -> Self;

    /// Load from file
    fn load(path: &Path) -> Result<Self, OfficeError>;

    /// Save to file
    fn save(&self, path: &Path) -> Result<(), OfficeError>;

    /// Export to different format
    fn export(&self, path: &Path, format: ExportFormat) -> Result<(), OfficeError>;

    /// Check if document has unsaved changes
    fn is_modified(&self) -> bool;

    /// Get document metadata
    fn metadata(&self) -> DocumentMetadata;
}

pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub word_count: Option<usize>,
    pub page_count: Option<usize>,
}
```

### 2. Text Editing Core (`src/office/text.rs`)

Shared text editing infrastructure (extends Q-EDIT):

```rust
pub struct TextBuffer {
    lines: Vec<String>,
    cursor: Position,
    selection: Option<Selection>,
    undo_stack: Vec<TextOp>,
    redo_stack: Vec<TextOp>,
    markers: [Option<Position>; 4], // A, B, C, D markers
    clipboard: String,
}

pub trait TextEditor {
    fn buffer(&self) -> &TextBuffer;
    fn buffer_mut(&mut self) -> &mut TextBuffer;

    // Navigation
    fn move_cursor(&mut self, motion: Motion);
    fn goto_line(&mut self, line: usize);
    fn goto_marker(&mut self, marker: char);

    // Editing
    fn insert(&mut self, text: &str);
    fn delete(&mut self, range: Range);
    fn undo(&mut self);
    fn redo(&mut self);

    // Selection
    fn select(&mut self, range: Range);
    fn copy(&mut self);
    fn cut(&mut self);
    fn paste(&mut self);

    // Search
    fn find(&mut self, query: &str, opts: FindOptions) -> Vec<Match>;
    fn replace(&mut self, from: &str, to: &str, opts: ReplaceOptions);
}
```

### 3. UI Components (`src/office/ui.rs`)

Extended component library for office apps:

```rust
// Document tabs (for multi-document editing)
pub struct TabBar {
    tabs: Vec<Tab>,
    active: usize,
}

// Toolbar with tool icons
pub struct Toolbar {
    tools: Vec<ToolButton>,
    selected: usize,
}

// Status bar with document info
pub struct DocumentStatusBar {
    filename: String,
    modified: bool,
    position: Position,
    mode: String,
    encoding: String,
}

// Canvas for graphics apps
pub struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Vec<Color>>,
    zoom: f32,
}

// Grid for spreadsheet
pub struct Grid {
    columns: Vec<Column>,
    rows: Vec<Row>,
    frozen_rows: usize,
    frozen_cols: usize,
    selection: CellRange,
}

// Timeline for MIDI/presentations
pub struct Timeline {
    tracks: Vec<Track>,
    position: usize,
    zoom: f32,
    playing: bool,
}
```

### 4. AI Integration (`src/office/ai.rs`)

Shared AI capabilities across suite:

```rust
pub enum AIAssistant {
    /// Writing assistance (Q-DOCS, Q-MAIL)
    Writing {
        mode: WritingMode, // Draft, Edit, Summarize, Expand
    },
    /// Formula assistance (Q-SHEET)
    Formula {
        context: Vec<CellReference>,
    },
    /// Code assistance (Q-CODE)
    Code {
        language: String,
        context: CodeContext,
    },
    /// Creative assistance (Q-DESIGN, Q-PAINT)
    Creative {
        style: String,
        prompt: String,
    },
    /// Content generation (Q-DECK)
    Presentation {
        topic: String,
        slides: usize,
    },
}

pub trait AIEnabled {
    /// Check if AI features are available
    fn ai_available(&self) -> bool;

    /// Get AI suggestions for current context
    fn ai_suggest(&self) -> Vec<AISuggestion>;

    /// Apply AI suggestion
    fn ai_apply(&mut self, suggestion: &AISuggestion);

    /// Generate content with AI
    fn ai_generate(&mut self, prompt: &str) -> Result<String, AIError>;
}
```

### 5. Network Layer (`src/office/net.rs`)

Shared networking for Q-WEB, Q-MAIL:

```rust
pub struct NetworkClient {
    http: HttpClient,
    cache: Cache,
    cookies: CookieJar,
}

impl NetworkClient {
    pub fn fetch(&self, url: &str) -> Result<Response, NetError>;
    pub fn post(&self, url: &str, body: &[u8]) -> Result<Response, NetError>;
    pub fn download(&self, url: &str, path: &Path) -> Result<(), NetError>;
}

// Email-specific
pub struct EmailClient {
    imap: ImapConnection,
    smtp: SmtpConnection,
    accounts: Vec<EmailAccount>,
}
```

### 6. Data Formats (`src/office/formats/`)

Format parsers and writers:

```
formats/
├── csv.rs       # CSV parsing (Q-SHEET)
├── markdown.rs  # Markdown (Q-DOCS, Q-DECK)
├── docx.rs      # Word documents (Q-DOCS)
├── html.rs      # HTML rendering (Q-WEB)
├── midi.rs      # MIDI files (Q-MIDI)
├── png.rs       # PNG images (Q-PAINT, Q-DESIGN)
├── bmp.rs       # BMP images (Q-PAINT)
├── ansi.rs      # ANSI art (Q-DECK, Q-DESIGN)
└── email.rs     # Email formats (Q-MAIL)
```

### 7. Clipboard Integration (`src/office/clipboard.rs`)

System clipboard support:

```rust
pub struct Clipboard {
    system: SystemClipboard,  // OS clipboard
    internal: String,         // Internal buffer
    history: Vec<String>,     // Clipboard history
}

impl Clipboard {
    pub fn copy(&mut self, text: &str);
    pub fn paste(&self) -> Option<String>;
    pub fn history(&self) -> &[String];
}
```

### 8. Help System (`src/office/help.rs`)

Context-sensitive help for all apps:

```rust
pub struct HelpSystem {
    topics: HashMap<String, HelpTopic>,
    index: Vec<HelpEntry>,
}

impl HelpSystem {
    pub fn show_help(&self, app: &str, context: &str);
    pub fn search(&self, query: &str) -> Vec<HelpEntry>;
}
```

---

## Plugin Integration

### Git/Jj Integration

All document-editing apps integrate with version control:

- **Auto-save to git**: Commit on save (optional)
- **Version history**: Browse document history
- **Diff view**: See changes between versions
- **Branch awareness**: Show current branch in status

### Beads Integration

Project management across apps:

- **Link issues**: Reference issues in documents
- **Task tracking**: Create tasks from TODOs in code/docs
- **Time tracking**: Log time spent in apps

### MCP Integration

AI assistants can use MCP tools:

- **File access**: Read/write files for context
- **Web search**: Research for documents
- **Database**: Query data for spreadsheets

### Viewer Integration

Seamless transitions:

- **View → Edit**: Open viewed file in appropriate editor
- **Preview**: Real-time preview in viewer pane

---

## Application Specifications

---

## 1. Q-SHEET (Spreadsheet)

### Overview
A VisiCalc/Lotus 1-2-3 inspired spreadsheet editor for CSV files with formula support.

### Screen Layout

```
 Q-SHEET: budget.csv                                              [Modified]
═══════════════════════════════════════════════════════════════════════════════
     │    A         │    B         │    C         │    D         │    E
═════╪══════════════╪══════════════╪══════════════╪══════════════╪════════════
   1 │ Item         │ Q1           │ Q2           │ Q3           │ Q4
─────┼──────────────┼──────────────┼──────────────┼──────────────┼────────────
   2 │ Revenue      │      100,000 │      120,000 │      115,000 │    140,000
   3 │ Expenses     │       80,000 │       85,000 │       82,000 │     90,000
   4 │ Profit       │ =B2-B3       │ =C2-C3       │ =D2-D3       │ =E2-E3
   5 │              │              │              │              │
   6 │ Total        │              │              │              │ =SUM(B4:E4)
═══════════════════════════════════════════════════════════════════════════════
 Cell: B4  Formula: =B2-B3  Value: 20,000                    Rows: 100 Cols: 26
───────────────────────────────────────────────────────────────────────────────
 F1 Help  F2 Edit Cell  F3 Format  F5 Goto  F7 Find  F9 Recalc  F10 Menu  ESC
```

### Features

**Cell Operations:**
- Navigate with arrow keys, Tab, Enter
- Edit cell with F2 or typing
- Multi-cell selection with Shift+arrows
- Copy/paste cells and ranges
- Fill down/right

**Formula Support:**
```
Arithmetic: +, -, *, /, ^, %
Functions:
  - SUM(range)      - Sum of values
  - AVG(range)      - Average
  - COUNT(range)    - Count of values
  - MIN(range)      - Minimum
  - MAX(range)      - Maximum
  - IF(cond,t,f)    - Conditional
  - ROUND(n,d)      - Round to decimals
  - ABS(n)          - Absolute value
  - CONCAT(a,b,...) - String concatenation

Cell References:
  - A1              - Relative reference
  - $A$1            - Absolute reference
  - A1:D10          - Range reference
  - Sheet2!A1       - Cross-sheet reference
```

**Formatting:**
- Number formats (currency, percent, date)
- Column width adjustment
- Row height adjustment
- Text alignment

**AI Features:**
- Formula suggestions based on data
- Data analysis summaries
- Auto-fill pattern detection

### File Format
Primary: CSV with formula comments
```csv
# Q-SHEET formula: B4=B2-B3
Item,Q1,Q2,Q3,Q4
Revenue,100000,120000,115000,140000
Expenses,80000,85000,82000,90000
Profit,20000,35000,33000,50000
```

---

## 2. Q-DECK (Presentations)

### Overview
ANSI/ASCII slideshow editor with templates, inspired by demo scene aesthetics.

### Screen Layout (Edit Mode)

```
 Q-DECK: presentation.qdeck                         Slide 3 of 12  [Modified]
═══════════════════════════════════════════════════════════════════════════════
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                     ╔═══════════════════════════════════╗                   │
│                     ║   WELCOME TO THE FUTURE           ║                   │
│                     ╚═══════════════════════════════════╝                   │
│                                                                             │
│     • First bullet point here                                               │
│     • Second important item                                                 │
│     • Third thing to remember                                               │
│                                                                             │
│                          ▄▄▄▄▄                                              │
│                         █░░░░░█   [ASCII art logo]                          │
│                          ▀▀▀▀▀                                              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
═══════════════════════════════════════════════════════════════════════════════
 [Title] [Bullets] [Two-Col] [Image] [Code] [Quote]     Template: Corporate
───────────────────────────────────────────────────────────────────────────────
 F1 Help  F3 Template  F5 Present  F7 Insert  F9 Preview  F10 Menu  ESC
```

### Features

**Slide Templates:**
- Title slide
- Bullet list
- Two-column
- Image/ASCII art
- Code block
- Quote/callout
- Comparison
- Timeline

**Content Types:**
- Text with styling (bold, dim, colors)
- ANSI art blocks
- Code blocks with syntax highlighting
- ASCII diagrams
- Embedded images (converted to ANSI)

**Presentation Mode:**
- Full-screen slides
- Keyboard navigation (arrows, space)
- Transition effects (fade, slide, reveal)
- Speaker notes (separate view)
- Timer display

**AI Features:**
- Generate slides from outline
- Suggest content for bullets
- Create ASCII art from descriptions
- Auto-layout content

### File Format (Markdown-based)

```markdown
---
title: My Presentation
author: John Doe
theme: corporate
---

# Welcome to the Future
<!-- template: title -->

---

# Key Points
<!-- template: bullets -->

* First bullet point here
* Second important item
* Third thing to remember

```ansi
╔═══════════════════════════╗
║   Company Logo Here       ║
╚═══════════════════════════╝
```

---

# Code Example
<!-- template: code -->

```rust
fn main() {
    println!("Hello, Q-DOS!");
}
```
```

---

## 3. Q-WEB (Web Browser)

### Overview
Lynx-style terminal web browser with reader mode and basic form support.

### Screen Layout

```
 Q-WEB: https://example.com/article                              [Loading...]
═══════════════════════════════════════════════════════════════════════════════
 [Back] [Forward] [Reload] [Home] [Bookmarks]           Mode: [Reader] [Raw]
───────────────────────────────────────────────────────────────────────────────

                         Example News Article

   By Jane Smith | January 12, 2026 | 5 min read

   ─────────────────────────────────────────────────────────────────────────

   This is the main article content rendered in a clean, readable format.
   Links are shown [1]like this[1] with numbered references.

   The browser automatically extracts the main content and removes
   navigation, ads, and other clutter.

   Key Features:
   • Clean text rendering
   • Numbered link references
   • Form support for search/login
   • Bookmarks and history

   [1] https://example.com/link

═══════════════════════════════════════════════════════════════════════════════
 Links: 5  Forms: 1  Images: 3 (hidden)                          Page 1 of 2
───────────────────────────────────────────────────────────────────────────────
 G Goto URL  / Search  B Bookmarks  H History  D Download  TAB Next link  ESC
```

### Features

**Navigation:**
- URL bar with history
- Back/forward navigation
- Bookmarks management
- History browser
- Tab between links
- Numbered link jumping (press number)

**Rendering Modes:**
- **Reader**: Clean article extraction
- **Raw**: Full HTML rendered as text
- **Source**: View HTML source

**Form Support:**
- Text inputs
- Password fields (masked)
- Submit buttons
- Checkboxes/radio buttons
- Select dropdowns

**AI Features:**
- Summarize long articles
- Extract key points
- Answer questions about page content
- Translate pages

### Bookmarks Format

```toml
[[bookmarks]]
title = "Example Site"
url = "https://example.com"
tags = ["news", "tech"]
added = "2026-01-12"

[[bookmarks]]
title = "Search Engine"
url = "https://duckduckgo.com"
tags = ["search"]
added = "2026-01-10"
```

---

## 4. Q-DOCS (Word Processor)

### Overview
WordPerfect-inspired document editor supporting Markdown and DOC formats.

### Screen Layout

```
 Q-DOCS: report.md                                    Page 1 of 5  [Modified]
═══════════════════════════════════════════════════════════════════════════════
 File  Edit  View  Insert  Format  Tools  Help
───────────────────────────────────────────────────────────────────────────────
                                                                              │
   # Project Report                                                           │
                                                                              │
   ## Executive Summary                                                       │
                                                                              │
   This report outlines the key findings from our Q4 analysis. The           │
   results show significant improvement in all metrics.                       │
                                                                              │
   ### Key Findings                                                           │
                                                                              │
   1. Revenue increased by 25%                                                │
   2. Customer satisfaction at 94%                                            │
   3. New product launch successful                                           │
                                                                              │
   > "The best quarter we've ever had." - CEO                                 │
                                                                              │
═══════════════════════════════════════════════════════════════════════════════
 report.md    Line: 12   Col: 45   Words: 234   INSERT   [Markdown]
───────────────────────────────────────────────────────────────────────────────
 F1 Help  F2 Save  F3 Open  F5 Find  F7 Spell  F9 Preview  F10 Menu  ESC
```

### Features

**Document Formats:**
- Markdown (.md) - Primary format
- Plain text (.txt)
- Word (.doc/.docx) - Read/write via pandoc

**Editing:**
- Full text editing (extends Q-EDIT)
- Markdown syntax highlighting
- Live preview pane (optional)
- Spell checking
- Word count

**Formatting:**
- Headings (H1-H6)
- Bold, italic, strikethrough
- Lists (bullet, numbered)
- Blockquotes
- Code blocks
- Tables
- Links and images

**AI Features:**
- Writing suggestions
- Grammar checking
- Summarization
- Expand/condense text
- Tone adjustment
- Translation

### Export Options
- PDF (via pandoc)
- HTML
- Plain text
- RTF

---

## 5. Q-CODE (IDE)

### Overview
VIM-style code editor with LSP support, syntax highlighting, and integrated terminal.

### Screen Layout

```
 Q-CODE: src/main.rs                                              [Rust] [LSP]
═══════════════════════════════════════════════════════════════════════════════
  1 │ use std::io;
  2 │
  3 │ fn main() {
  4 │     println!("Hello, world!");
  5▐│
  6 │     let mut input = String::new();
  7 │     io::stdin().read_line(&mut input)
  8 │         .expect("Failed to read line");
  9 │
 10 │     println!("You entered: {}", input.trim());
 11 │ }
═══════════════════════════════════════════════════════════════════════════════
 Problems (2)                                                    │ Outline
 ─────────────────────────────────────────────────────────────────┼──────────
 ⚠ Line 7: Consider using `?` instead of `.expect()`             │ fn main
 ℹ Line 4: Unused variable `input` on line 6                     │
═══════════════════════════════════════════════════════════════════════════════
 -- NORMAL --    main.rs [+]    Ln 5, Col 1    UTF-8    LF    rust-analyzer ✓
───────────────────────────────────────────────────────────────────────────────
 :w Save  :q Quit  :e Open  / Search  g Goto  K Hover  F5 Run  F9 Debug
```

### Features

**VIM Modes:**
- Normal mode (navigation)
- Insert mode (editing)
- Visual mode (selection)
- Command mode (:commands)

**LSP Features:**
- Syntax highlighting (tree-sitter)
- Autocomplete
- Go to definition
- Find references
- Hover documentation
- Diagnostics/errors
- Code actions
- Rename symbol

**Languages Supported:**
- Rust, Go, Python, JavaScript/TypeScript
- C, C++, Java, Ruby
- HTML, CSS, JSON, YAML, TOML
- Markdown, Shell scripts

**Integrated Tools:**
- Terminal pane
- File explorer
- Git integration
- Problem list
- Outline view

**AI Features:**
- Code completion (Copilot-style)
- Explain code
- Generate tests
- Refactoring suggestions
- Documentation generation

---

## 6. Q-PAINT (Graphics Editor)

### Overview
DeluxePaint-style pixel art editor for creating images in the terminal.

### Screen Layout

```
 Q-PAINT: sprite.png                                   32x32  Zoom: 4x  [Mod]
═══════════════════════════════════════════════════════════════════════════════
 [Pencil] [Line] [Rect] [Circle] [Fill] [Select] [Text]    Color: ██ #FF0000
───────────────────────────────────────────────────────────────────────────────
                        │
    ░░░░████░░░░        │  Palette:
    ░░██░░░░██░░        │  ┌────────────────────┐
    ░█░░░░░░░░█░        │  │▓▓░░████▓▓░░████▓▓░░│
    █░░██░░██░░█        │  │████░░▓▓████░░▓▓████│
    █░░░░░░░░░░█        │  │░░▓▓████░░▓▓████░░▓▓│
    █░██░░░░██░█        │  └────────────────────┘
    █░░████████░█        │
    ░█░░░░░░░░█░        │  Layers:
    ░░██░░░░██░░        │  [x] Background
    ░░░░████░░░░        │  [x] Sprite
                        │  [ ] Effects
───────────────────────────────────────────────────────────────────────────────
 Tool: Pencil  Size: 1px  X: 15  Y: 12                          Canvas: 32x32
═══════════════════════════════════════════════════════════════════════════════
 F1 Help  1-9 Tools  C Color  L Layers  Z Undo  S Save  E Export  ESC Menu
```

### Features

**Drawing Tools:**
- Pencil (freehand)
- Line
- Rectangle/Square
- Circle/Ellipse
- Fill (flood)
- Selection (rect, lasso)
- Text

**Color:**
- 256-color palette
- Custom palette editor
- Color picker
- Transparency support

**Layers:**
- Multiple layers
- Layer visibility
- Layer ordering
- Merge layers

**Export Formats:**
- PNG
- BMP
- ANSI art (for terminal display)
- ASCII art

**AI Features:**
- Generate pixel art from description
- Upscale/enhance images
- Style transfer
- Auto-palette generation

---

## 7. Q-MAIL (Email Client)

### Overview
Pine/Mutt-style email client with IMAP/SMTP support.

### Screen Layout

```
 Q-MAIL: thrashr888@example.com                        INBOX (5 new)
═══════════════════════════════════════════════════════════════════════════════
 Folders          │ Messages
 ─────────────────┼────────────────────────────────────────────────────────────
 > INBOX      (5) │   From              Subject                    Date
   Sent           │ N john@example.com  Project Update             Jan 12
   Drafts     (1) │ N support@acme.co   Your ticket #1234          Jan 12
   Archive        │   newsletter@tech   Weekly Digest              Jan 11
   Spam           │   boss@work.com     RE: Meeting tomorrow       Jan 11
   Trash          │   friend@social     Check this out!            Jan 10
                  │
                  │
═══════════════════════════════════════════════════════════════════════════════
 5 messages, 2 new                                      Account: Personal
───────────────────────────────────────────────────────────────────────────────
 C Compose  R Reply  F Forward  D Delete  M Move  A Archive  / Search  ESC
```

### Features

**Email Operations:**
- Compose/reply/forward
- Attachments (view/save)
- HTML rendering (converted to text)
- Threading view
- Search
- Filters/rules

**Account Management:**
- Multiple accounts
- IMAP/SMTP configuration
- OAuth2 support (Gmail, etc.)
- Secure credential storage

**Organization:**
- Folders/labels
- Archive
- Spam handling
- Filters

**AI Features:**
- Smart compose suggestions
- Email summarization
- Priority sorting
- Spam detection

**Security:**
- Encrypted credential storage
- TLS connections
- No tracking pixels

---

## 8. Q-FORM (Form Designer)

### Overview
Form Master-style form designer for creating printable forms.

### Screen Layout

```
 Q-FORM: invoice.qform                                            [Modified]
═══════════════════════════════════════════════════════════════════════════════
 [Text] [Field] [Line] [Box] [Table] [Logo] [Checkbox]     Grid: ON  Snap: ON
───────────────────────────────────────────────────────────────────────────────
 ┌───────────────────────────────────────────────────────────────────────────┐
 │                           INVOICE                                          │
 │                                                                            │
 │  Invoice #: [__________]              Date: [__________]                  │
 │                                                                            │
 │  Bill To:                              Ship To:                            │
 │  [_________________________]          [_________________________]         │
 │  [_________________________]          [_________________________]         │
 │                                                                            │
 │  ┌──────────────────────────────────────────────────────────────────────┐ │
 │  │ Item          │ Qty │ Price    │ Total    │                          │ │
 │  ├───────────────┼─────┼──────────┼──────────┤                          │ │
 │  │ [___________] │ [_] │ [______] │ [______] │                          │ │
 │  └──────────────────────────────────────────────────────────────────────┘ │
 └───────────────────────────────────────────────────────────────────────────┘
═══════════════════════════════════════════════════════════════════════════════
 Tool: Field    Width: 10    X: 12  Y: 3                      Page 1 of 1
───────────────────────────────────────────────────────────────────────────────
 F1 Help  F3 Preview  F5 Print  F7 Templates  F9 Fill Mode  ESC Menu
```

### Features

**Form Elements:**
- Static text/labels
- Input fields (text, number, date)
- Checkboxes/radio buttons
- Lines and boxes
- Tables
- Logo/image placeholders

**Templates:**
- Invoice
- Receipt
- Order form
- Application form
- Survey
- Certificate

**Output:**
- Print directly
- Export to PDF
- Save as template

**AI Features:**
- Generate form from description
- Smart field suggestions
- Layout optimization

---

## 9. Q-DESIGN (Desktop Publishing)

### Overview
PrintMaster Gold-style greeting card and banner creator.

### Screen Layout

```
 Q-DESIGN: birthday.qdesign                            Card  [Front]  [Mod]
═══════════════════════════════════════════════════════════════════════════════
 [Text] [Art] [Border] [Shape] [Photo]                 Library: [Celebrations]
───────────────────────────────────────────────────────────────────────────────
                        │
  ╔══════════════════╗  │  Clip Art Library:
  ║  ☆ HAPPY ☆       ║  │  ┌────────────────────┐
  ║                  ║  │  │ 🎂 🎈 🎁 🎉 🎊    │
  ║    ╭─────╮       ║  │  │ 🌟 ❤️  🎵 🌸 🦋   │
  ║    │░░░░░│       ║  │  └────────────────────┘
  ║    │░░░░░│       ║  │
  ║    │░🕯️░░│       ║  │  Text Styles:
  ║    ╰─────╯       ║  │  • Banner
  ║                  ║  │  • Script
  ║  BIRTHDAY!       ║  │  • Block
  ╚══════════════════╝  │  • Shadow
                        │
───────────────────────────────────────────────────────────────────────────────
 View: Front    Size: 5x7    Orientation: Portrait           Template: Cake
═══════════════════════════════════════════════════════════════════════════════
 F1 Help  1-4 Pages  T Text  A Art  B Border  P Print  E Export  ESC Menu
```

### Features

**Project Types:**
- Greeting cards (4 panels)
- Banners
- Signs
- Certificates
- Invitations
- Business cards

**Design Elements:**
- Clip art library (ASCII/ANSI)
- Text effects (shadow, outline)
- Borders and frames
- Shapes
- Import images

**Categories:**
- Birthdays
- Holidays
- Thank you
- Congratulations
- Business
- Custom

**AI Features:**
- Generate card designs
- Suggest messages
- Create custom ASCII art

---

## 10. Q-MIDI (MIDI Sequencer)

### Overview
MIDI sequencer with tracker-style interface for creating music.

### Screen Layout

```
 Q-MIDI: song.mid                                    BPM: 120  4/4  [Playing]
═══════════════════════════════════════════════════════════════════════════════
 Track 1: Piano    │ Track 2: Bass     │ Track 3: Drums    │ Track 4: Strings
 ──────────────────┼───────────────────┼───────────────────┼──────────────────
 C-4 .. .. .. F#4  │ C-2 .. .. .. ..   │ K.. S.. K.. S..   │ .. .. .. .. ..
 E-4 .. .. .. A-4  │ .. .. G-2 .. ..   │ H.. H.. H.. H..   │ C-5 .. .. .. ..
 G-4 .. .. .. C-5  │ C-2 .. .. .. ..   │ K.. S.. KK. S..   │ E-5 .. .. .. ..
▐.. .. .. .. ..    │ .. .. E-2 .. ..   │ H.. H.. H.. HO.   │ G-5 .. .. .. ..
 C-4 .. .. .. ..   │ A-1 .. .. .. ..   │ K.. S.. K.. S..   │ .. .. .. .. ..
═══════════════════════════════════════════════════════════════════════════════
 Pattern: 01/16    Position: 4        │ Instruments        │ Piano Roll
 ────────────────────────────────────┼────────────────────┼──────────────────
 [|||||||||||||.......]              │ 1. Grand Piano     │ ░░▓▓░░░░░░░░▓▓░░
                                     │ 2. Electric Bass   │ ▓▓░░░░▓▓░░░░░░░░
 Vol: [████████░░] 80%               │ 3. Drum Kit        │ ████░░██████░░██
═══════════════════════════════════════════════════════════════════════════════
 Space: Play/Stop  ←→: Pattern  ↑↓: Track  Tab: Edit  I: Instruments  ESC
```

### Features

**Sequencer:**
- Tracker-style pattern editor
- Piano roll view
- Multiple tracks (16+)
- Pattern chains

**MIDI:**
- Note input (keyboard or MIDI)
- Velocity/volume
- Pitch bend
- Control changes
- Program changes

**Instruments:**
- General MIDI instruments
- SoundFont support (via audio plugin)
- Drum kits

**Export:**
- MIDI file (.mid)
- WAV audio (with soundfont)

**AI Features:**
- Generate melodies
- Chord suggestions
- Drum pattern generation
- Style matching

---

## Cross-App Integration

### Universal Commands

All office apps share common keybindings:

| Key | Action |
|-----|--------|
| F1 | Help |
| F2 | Save |
| F3 | Open |
| F10 | Menu |
| Ctrl+Q | Quit |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+X | Cut |
| Ctrl+F | Find |

### App Interoperability

| From | To | Integration |
|------|------|-------------|
| Q-SHEET | Q-DOCS | Insert table from spreadsheet |
| Q-DOCS | Q-DECK | Convert document to slides |
| Q-CODE | Q-DOCS | Document code with comments |
| Q-PAINT | Q-DESIGN | Use artwork in cards |
| Q-WEB | Q-DOCS | Save article as document |
| Q-MAIL | Q-DOCS | Save email as document |
| Any | Viewer | Preview documents |

### Git/Jj Integration

All document types are version-controlled:
- Auto-commit on save (configurable)
- Diff view for documents
- History navigation
- Branch per project

### Beads Integration

- Create issues from TODOs
- Link documents to issues
- Track time in apps
- Project dashboards

---

## Implementation Phases

### Phase 1: Foundation (Shared Infrastructure)
1. Document trait and file operations
2. Text editing core
3. Extended UI components
4. Clipboard integration
5. Help system

### Phase 2: Core Apps
1. Q-SHEET (highest utility)
2. Q-DECK (unique to Q-DOS)
3. Q-WEB (high utility)

### Phase 3: Document Apps
4. Q-DOCS (completes office suite)
5. Q-CODE (developer tool)

### Phase 4: Creative Apps
6. Q-PAINT (graphics)
7. Q-DESIGN (publishing)
8. Q-FORM (forms)

### Phase 5: Communication & Media
9. Q-MAIL (email)
10. Q-MIDI (music)

---

## Quality Standards

All office apps must:

1. **Follow Q-DOS aesthetic** - DOS-era UI with modern features
2. **Use component library** - FullScreenView, ModalFrame, etc.
3. **Implement OfficeDocument** - Standard file operations
4. **Support Git integration** - Version control awareness
5. **Provide help** - F1 context-sensitive help
6. **Handle errors gracefully** - Q-DOS style error messages
7. **Pass quality gates** - fmt, clippy, tests

---

## File Structure

```
src/plugins/office/
├── mod.rs              # Office plugin registration
├── shared/
│   ├── mod.rs
│   ├── document.rs     # OfficeDocument trait
│   ├── text.rs         # TextBuffer, TextEditor
│   ├── clipboard.rs    # Clipboard operations
│   ├── help.rs         # Help system
│   └── ai.rs           # AI integration
├── formats/
│   ├── mod.rs
│   ├── csv.rs
│   ├── markdown.rs
│   ├── html.rs
│   └── ...
├── sheet/              # Q-SHEET
├── deck/               # Q-DECK
├── web/                # Q-WEB
├── docs/               # Q-DOCS
├── code/               # Q-CODE
├── paint/              # Q-PAINT
├── mail/               # Q-MAIL
├── form/               # Q-FORM
├── design/             # Q-DESIGN
└── midi/               # Q-MIDI
```
