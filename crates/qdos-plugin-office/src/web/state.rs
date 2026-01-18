//! Q-WEB State Types
//!
//! Data structures for the text web browser.

use std::collections::VecDeque;
use std::path::PathBuf;

// =============================================================================
// CONSTANTS
// =============================================================================

pub const MAX_HISTORY: usize = 100;
pub const MAX_TABS: usize = 10;

// =============================================================================
// WEB MODE
// =============================================================================

/// Operating mode for Q-WEB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebMode {
    #[default]
    Browse, // Normal browsing
    UrlInput,  // Entering URL
    Search,    // Search in page
    Bookmarks, // Bookmark manager
    History,   // History browser
    Menu,      // Menu system
    SaveAs,    // Save page dialog
    Help,      // Help overlay
    Loading,   // Page loading
    Error,     // Error display
}

// =============================================================================
// RENDER MODE
// =============================================================================

/// Page rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Reader, // Clean article extraction
    Raw,    // Full HTML as text
    Source, // View HTML source
}

impl RenderMode {
    pub fn name(&self) -> &'static str {
        match self {
            RenderMode::Reader => "Reader",
            RenderMode::Raw => "Raw",
            RenderMode::Source => "Source",
        }
    }

    pub fn next(&self) -> RenderMode {
        match self {
            RenderMode::Reader => RenderMode::Raw,
            RenderMode::Raw => RenderMode::Source,
            RenderMode::Source => RenderMode::Reader,
        }
    }
}

// =============================================================================
// LINK
// =============================================================================

/// A link on the page
#[derive(Debug, Clone)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub line: usize, // Line number in rendered content
}

// =============================================================================
// FORM FIELD
// =============================================================================

/// Form field types
#[derive(Debug, Clone)]
pub enum FormField {
    Text {
        name: String,
        value: String,
        placeholder: String,
    },
    Password {
        name: String,
        value: String,
    },
    Submit {
        name: String,
        value: String,
    },
    Hidden {
        name: String,
        value: String,
    },
    Checkbox {
        name: String,
        checked: bool,
    },
}

// =============================================================================
// FORM
// =============================================================================

/// An HTML form
#[derive(Debug, Clone)]
pub struct Form {
    pub action: String,
    pub method: String, // GET or POST
    pub fields: Vec<FormField>,
}

// =============================================================================
// PAGE
// =============================================================================

/// A loaded web page
#[derive(Debug, Clone, Default)]
pub struct Page {
    /// URL of the page
    pub url: String,
    /// Page title
    pub title: String,
    /// Article byline (author info from reader mode)
    pub byline: Option<String>,
    /// Rendered content lines
    pub content: Vec<String>,
    /// Raw HTML source
    pub source: String,
    /// Links within the content (numbered)
    pub links: Vec<Link>,
    /// Navigation links (from header/footer, shown at end)
    pub nav_links: Vec<Link>,
    /// Forms on the page
    pub forms: Vec<Form>,
    /// Current scroll position
    pub scroll: usize,
    /// Selected link index (for TAB navigation)
    pub selected_link: Option<usize>,
}

impl Page {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn error(url: &str, message: &str) -> Self {
        Self {
            url: url.to_string(),
            title: "Error".to_string(),
            content: vec![
                String::new(),
                format!("  Error loading page"),
                String::new(),
                format!("  URL: {}", url),
                String::new(),
                format!("  {}", message),
            ],
            ..Default::default()
        }
    }

    pub fn loading(url: &str) -> Self {
        Self {
            url: url.to_string(),
            title: "Loading...".to_string(),
            content: vec![
                String::new(),
                format!("  Loading: {}", url),
                String::new(),
                "  Please wait...".to_string(),
            ],
            ..Default::default()
        }
    }
}

// =============================================================================
// BOOKMARK
// =============================================================================

/// A bookmark entry
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub tags: Vec<String>,
    pub added: String, // ISO date string
}

// =============================================================================
// HISTORY ENTRY
// =============================================================================

/// A history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub title: String,
    pub url: String,
    pub visited: String, // ISO datetime string
}

// =============================================================================
// TAB HISTORY ENTRY
// =============================================================================

/// A tab-local history entry with scroll position
#[derive(Debug, Clone)]
pub struct TabHistoryEntry {
    pub url: String,
    pub scroll: usize,
}

// =============================================================================
// TAB
// =============================================================================

/// A browser tab
#[derive(Debug, Clone)]
pub struct Tab {
    pub page: Page,
    pub history: VecDeque<TabHistoryEntry>,
    pub history_pos: usize,
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            page: Page::empty(),
            history: VecDeque::new(),
            history_pos: 0,
        }
    }
}

impl Tab {
    pub fn new(url: &str) -> Self {
        let mut history = VecDeque::new();
        history.push_back(TabHistoryEntry {
            url: url.to_string(),
            scroll: 0,
        });
        Self {
            page: Page::loading(url),
            history,
            history_pos: 0,
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history_pos > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_pos < self.history.len().saturating_sub(1)
    }

    /// Save current scroll position to history before navigating
    fn save_scroll(&mut self) {
        if let Some(entry) = self.history.get_mut(self.history_pos) {
            entry.scroll = self.page.scroll;
        }
    }

    pub fn go_back(&mut self) -> Option<(String, usize)> {
        if self.can_go_back() {
            self.save_scroll();
            self.history_pos -= 1;
            self.history
                .get(self.history_pos)
                .map(|e| (e.url.clone(), e.scroll))
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<(String, usize)> {
        if self.can_go_forward() {
            self.save_scroll();
            self.history_pos += 1;
            self.history
                .get(self.history_pos)
                .map(|e| (e.url.clone(), e.scroll))
        } else {
            None
        }
    }

    pub fn navigate(&mut self, url: &str) {
        // Save current scroll position before navigating
        self.save_scroll();
        // Remove forward history
        while self.history.len() > self.history_pos + 1 {
            self.history.pop_back();
        }
        // Add new entry
        self.history.push_back(TabHistoryEntry {
            url: url.to_string(),
            scroll: 0,
        });
        if self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        } else {
            self.history_pos += 1;
        }
        self.page = Page::loading(url);
    }
}

// =============================================================================
// WEB STATE
// =============================================================================

/// Main browser state
pub struct WebState {
    // Mode
    pub mode: WebMode,
    pub render_mode: RenderMode,

    // Tabs
    pub tabs: Vec<Tab>,
    pub active_tab: usize,

    // URL input
    pub url_input: String,
    pub url_cursor: usize,

    // Search
    pub search_query: String,
    pub search_results: Vec<usize>, // Line numbers
    pub search_index: usize,

    // Bookmarks and history
    pub bookmarks: Vec<Bookmark>,
    pub global_history: VecDeque<HistoryEntry>,
    pub bookmarks_path: Option<PathBuf>,

    // UI state
    pub bookmarks_selected: usize,
    pub history_selected: usize,

    // Save As
    pub save_as_input: String,
    pub save_as_cursor: usize,

    // Status
    pub status_message: Option<(String, u32)>,
    pub tick_count: u32,
}

impl Default for WebState {
    fn default() -> Self {
        Self::new()
    }
}

impl WebState {
    pub fn new() -> Self {
        // Start with a blank tab showing home page
        let home = Tab::new("about:home");

        Self {
            mode: WebMode::Browse,
            render_mode: RenderMode::Reader,
            tabs: vec![home],
            active_tab: 0,
            url_input: String::new(),
            url_cursor: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_index: 0,
            bookmarks: Vec::new(),
            global_history: VecDeque::new(),
            bookmarks_path: None,
            bookmarks_selected: 0,
            history_selected: 0,
            save_as_input: String::new(),
            save_as_cursor: 0,
            status_message: None,
            tick_count: 0,
        }
    }

    // =========================================================================
    // TAB MANAGEMENT
    // =========================================================================

    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn current_page(&self) -> &Page {
        &self.current_tab().page
    }

    pub fn current_page_mut(&mut self) -> &mut Page {
        &mut self.current_tab_mut().page
    }

    pub fn new_tab(&mut self, url: &str) {
        if self.tabs.len() < MAX_TABS {
            self.tabs.push(Tab::new(url));
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    pub fn next_tab(&mut self) {
        if self.active_tab < self.tabs.len() - 1 {
            self.active_tab += 1;
        }
    }

    pub fn prev_tab(&mut self) {
        if self.active_tab > 0 {
            self.active_tab -= 1;
        }
    }

    // =========================================================================
    // NAVIGATION
    // =========================================================================

    pub fn navigate(&mut self, url: &str) {
        self.current_tab_mut().navigate(url);
        // Add to global history
        self.global_history.push_front(HistoryEntry {
            title: String::new(), // Will be updated when page loads
            url: url.to_string(),
            visited: chrono::Utc::now().to_rfc3339(),
        });
        if self.global_history.len() > MAX_HISTORY {
            self.global_history.pop_back();
        }
    }

    pub fn go_back(&mut self) -> Option<(String, usize)> {
        self.current_tab_mut().go_back()
    }

    pub fn go_forward(&mut self) -> Option<(String, usize)> {
        self.current_tab_mut().go_forward()
    }

    pub fn reload(&mut self) -> String {
        self.current_page().url.clone()
    }

    // =========================================================================
    // SCROLLING
    // =========================================================================

    pub fn scroll_up(&mut self, lines: usize) {
        let page = self.current_page_mut();
        page.scroll = page.scroll.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, max_height: usize) {
        let page = self.current_page_mut();
        let max_scroll = page.content.len().saturating_sub(max_height);
        page.scroll = (page.scroll + lines).min(max_scroll);
    }

    pub fn scroll_page_up(&mut self, page_height: usize) {
        self.scroll_up(page_height.saturating_sub(2));
    }

    pub fn scroll_page_down(&mut self, page_height: usize) {
        self.scroll_down(page_height.saturating_sub(2), page_height);
    }

    // =========================================================================
    // LINK NAVIGATION
    // =========================================================================

    /// Get total number of links (content + navigation)
    fn total_links(&self) -> usize {
        let page = self.current_page();
        page.links.len() + page.nav_links.len()
    }

    /// Get a link by combined index (content links first, then nav links)
    fn get_link(&self, idx: usize) -> Option<&Link> {
        let page = self.current_page();
        if idx < page.links.len() {
            page.links.get(idx)
        } else {
            page.nav_links.get(idx - page.links.len())
        }
    }

    pub fn next_link(&mut self) {
        let total = self.total_links();
        if total == 0 {
            return;
        }
        let page = self.current_page_mut();
        page.selected_link = Some(page.selected_link.map(|i| (i + 1) % total).unwrap_or(0));
    }

    pub fn prev_link(&mut self) {
        let total = self.total_links();
        if total == 0 {
            return;
        }
        let page = self.current_page_mut();
        page.selected_link = Some(
            page.selected_link
                .map(|i| if i == 0 { total - 1 } else { i - 1 })
                .unwrap_or(total - 1),
        );
    }

    pub fn activate_link(&self) -> Option<String> {
        let page = self.current_page();
        if let Some(idx) = page.selected_link {
            self.get_link(idx).map(|l| l.url.clone())
        } else {
            None
        }
    }

    pub fn goto_link(&self, num: usize) -> Option<String> {
        let total = self.total_links();
        if num > 0 && num <= total {
            self.get_link(num - 1).map(|l| l.url.clone())
        } else {
            None
        }
    }

    /// Get the currently selected link (for status bar display)
    pub fn selected_link_info(&self) -> Option<&Link> {
        let page = self.current_page();
        page.selected_link.and_then(|idx| self.get_link(idx))
    }

    // =========================================================================
    // URL INPUT
    // =========================================================================

    pub fn enter_url_mode(&mut self) {
        self.mode = WebMode::UrlInput;
        self.url_input = self.current_page().url.clone();
        self.url_cursor = self.url_input.len();
    }

    pub fn url_insert(&mut self, c: char) {
        self.url_input.insert(self.url_cursor, c);
        self.url_cursor += 1;
    }

    pub fn url_backspace(&mut self) {
        if self.url_cursor > 0 {
            self.url_cursor -= 1;
            self.url_input.remove(self.url_cursor);
        }
    }

    // =========================================================================
    // BOOKMARKS
    // =========================================================================

    pub fn add_bookmark(&mut self) {
        let page = self.current_page();
        self.bookmarks.push(Bookmark {
            title: page.title.clone(),
            url: page.url.clone(),
            tags: Vec::new(),
            added: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        });
        self.status_message = Some(("Bookmark added".to_string(), 30));
    }

    pub fn remove_bookmark(&mut self, index: usize) {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
        }
    }

    // =========================================================================
    // DISPLAY HELPERS
    // =========================================================================

    pub fn display_url(&self) -> &str {
        &self.current_page().url
    }

    pub fn display_title(&self) -> &str {
        let title = &self.current_page().title;
        if title.is_empty() {
            "Q-WEB"
        } else {
            title
        }
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn link_count(&self) -> usize {
        self.total_links()
    }

    pub fn form_count(&self) -> usize {
        self.current_page().forms.len()
    }
}
