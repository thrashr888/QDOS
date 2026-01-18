//! Q-WEB - Text Web Browser Plugin
//!
//! Lynx-style terminal web browser with reader mode, bookmarks, and form support.

mod modal;
pub mod state;

use crate::shared::{extract_reader_content, HttpClient};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::ThemeColors;
use qdos_plugin_api::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use ratatui::{layout::Rect, Frame};
use state::{Link, Page, WebMode, WebState};
use std::any::Any;
use std::path::PathBuf;

// =============================================================================
// WEB PLUGIN
// =============================================================================

pub struct WebPlugin {
    pub state: Option<WebState>,
    http: HttpClient,
}

impl Default for WebPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPlugin {
    pub fn new() -> Self {
        Self {
            state: None,
            http: HttpClient::new(),
        }
    }

    pub fn launch(&mut self) {
        let mut state = WebState::new();
        // Load home page
        state.current_page_mut().content = vec![
            String::new(),
            "  Welcome to Q-WEB".to_string(),
            String::new(),
            "  A text-based web browser for Q-DOS".to_string(),
            String::new(),
            "  Press G to go to a URL".to_string(),
            "  Press B to view bookmarks".to_string(),
            "  Press H to view history".to_string(),
            String::new(),
            "  ─────────────────────────────────────────".to_string(),
            String::new(),
            "  Suggested Sites:".to_string(),
            String::new(),
            "  [1] https://lite.duckduckgo.com/lite/".to_string(),
            "  [2] https://text.npr.org/".to_string(),
            "  [3] https://lite.cnn.com/".to_string(),
            "  [4] https://en.m.wikipedia.org/".to_string(),
            "  [5] https://news.ycombinator.com/".to_string(),
        ];
        state.current_page_mut().title = "Q-WEB Home".to_string();
        state.current_page_mut().links = vec![
            Link {
                text: "DuckDuckGo Lite".to_string(),
                url: "https://lite.duckduckgo.com/lite/".to_string(),
                line: 13,
            },
            Link {
                text: "NPR Text".to_string(),
                url: "https://text.npr.org/".to_string(),
                line: 14,
            },
            Link {
                text: "CNN Lite".to_string(),
                url: "https://lite.cnn.com/".to_string(),
                line: 15,
            },
            Link {
                text: "Wikipedia Mobile".to_string(),
                url: "https://en.m.wikipedia.org/".to_string(),
                line: 16,
            },
            Link {
                text: "Hacker News".to_string(),
                url: "https://news.ycombinator.com/".to_string(),
                line: 17,
            },
        ];
        self.state = Some(state);
    }

    // =========================================================================
    // PAGE LOADING
    // =========================================================================

    fn load_url(&mut self, url: &str) {
        self.load_url_with_scroll(url, None);
    }

    fn load_url_with_scroll(&mut self, url: &str, restore_scroll: Option<usize>) {
        let state = self.state.as_mut().unwrap();

        // Handle special URLs
        if url == "about:home" {
            self.launch();
            return;
        }

        // Resolve relative URLs against current page
        let url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("//") {
            // Protocol-relative URL
            format!("https:{}", url)
        } else if url.starts_with('/') {
            // Absolute path - use current host
            let current_url = &state.current_page().url;
            if let Some(base) = extract_base_url(current_url) {
                format!("{}{}", base, url)
            } else {
                format!("https://{}", url)
            }
        } else if !url.contains("://") {
            // Relative path - resolve against current page
            let current_url = &state.current_page().url;
            resolve_relative_url(current_url, url)
        } else {
            url.to_string()
        };

        // Only navigate (add to history) if not restoring from history
        if restore_scroll.is_none() {
            state.navigate(&url);
        }

        // Fetch the page
        match self.http.get_follow_redirects(&url, 5) {
            Ok(response) => {
                if response.is_success() {
                    let html = response.text().unwrap_or_default();

                    // Use reader mode to extract article content with wrapping
                    // Content width: 76 chars (80 - 4 for margins)
                    let reader_doc = extract_reader_content(&html, 76);

                    let title = if reader_doc.title.is_empty() {
                        "Untitled".to_string()
                    } else {
                        reader_doc.title
                    };

                    // Build structured content:
                    // [title]
                    // [byline if present]
                    // [blank line]
                    // [content]
                    // [blank line]
                    // [navigation section if there are nav links]
                    let mut content = Vec::new();
                    let mut all_links: Vec<Link> = Vec::new();

                    // Title (already in header, but add prominent display)
                    content.push(format!("  {}", title));
                    if let Some(byline) = &reader_doc.byline {
                        if !byline.is_empty() {
                            content.push(format!("  By: {}", byline));
                        }
                    }
                    content.push(String::new());
                    content.push("  ".to_string() + &"─".repeat(72));
                    content.push(String::new());

                    // Track where content starts for link line numbers
                    let content_start = content.len();

                    // Main article content
                    for line in &reader_doc.content {
                        content.push(format!("  {}", line));
                    }

                    // Adjust content link line numbers
                    for (text, url, line) in reader_doc.content_links {
                        all_links.push(Link {
                            text,
                            url,
                            line: line + content_start,
                        });
                    }

                    // Navigation links section (if any)
                    let nav_links: Vec<Link> = if !reader_doc.nav_links.is_empty() {
                        content.push(String::new());
                        content.push("  ".to_string() + &"─".repeat(72));
                        content.push("  Navigation Links:".to_string());
                        content.push(String::new());

                        let nav_start = content.len();
                        let mut nav_link_entries = Vec::new();

                        for (i, (text, url)) in reader_doc.nav_links.iter().enumerate() {
                            let display_text = if text.len() > 50 {
                                format!("{}...", &text[..47])
                            } else {
                                text.clone()
                            };
                            content.push(format!(
                                "  [{}] {}",
                                all_links.len() + i + 1,
                                display_text
                            ));

                            nav_link_entries.push(Link {
                                text: text.clone(),
                                url: url.clone(),
                                line: nav_start + i,
                            });
                        }

                        nav_link_entries
                    } else {
                        Vec::new()
                    };

                    let total_links = all_links.len() + nav_links.len();

                    // Update global history title first
                    if let Some(entry) = state.global_history.front_mut() {
                        entry.title = title.clone();
                    }

                    let page = state.current_page_mut();
                    page.url = response.url;
                    page.title = title;
                    page.byline = reader_doc.byline;
                    page.source = html;
                    page.content = content;
                    page.links = all_links;
                    page.nav_links = nav_links;
                    // Restore scroll position if navigating back/forward, otherwise start at top
                    page.scroll = restore_scroll.unwrap_or(0);
                    page.selected_link = None;

                    state.status_message = Some((format!("{} links", total_links), 30));
                } else {
                    let page = state.current_page_mut();
                    *page = Page::error(&url, &format!("HTTP {}", response.status));
                }
            }
            Err(e) => {
                let page = state.current_page_mut();
                *page = Page::error(&url, &e);
            }
        }
    }

    // =========================================================================
    // KEY HANDLING
    // =========================================================================

    pub fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let Some(state) = self.state.as_mut() else {
            return KeyHandleResult::NotHandled;
        };

        match state.mode {
            WebMode::Browse => self.handle_browse_key(key),
            WebMode::UrlInput => self.handle_url_input_key(key),
            WebMode::Search => self.handle_search_key(key),
            WebMode::Bookmarks => self.handle_bookmarks_key(key),
            WebMode::History => self.handle_history_key(key),
            WebMode::SaveAs => self.handle_save_as_key(key, cwd),
            WebMode::Menu | WebMode::Help | WebMode::Loading | WebMode::Error => {
                // Esc or any key exits these modes
                if key.code == KeyCode::Esc {
                    state.mode = WebMode::Browse;
                }
                KeyHandleResult::Handled
            }
        }
    }

    fn handle_browse_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Close browser
            KeyCode::Esc => KeyHandleResult::CloseModal,

            // Scrolling
            KeyCode::Up | KeyCode::Char('k') => {
                state.scroll_up(1);
                KeyHandleResult::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.scroll_down(1, 20); // TODO: get actual height
                KeyHandleResult::Handled
            }
            KeyCode::PageUp => {
                state.scroll_page_up(20);
                KeyHandleResult::Handled
            }
            KeyCode::PageDown => {
                state.scroll_page_down(20);
                KeyHandleResult::Handled
            }
            KeyCode::Home => {
                state.current_page_mut().scroll = 0;
                KeyHandleResult::Handled
            }
            KeyCode::End => {
                let page = state.current_page_mut();
                page.scroll = page.content.len().saturating_sub(20);
                KeyHandleResult::Handled
            }

            // Link navigation
            KeyCode::Tab => {
                state.next_link();
                KeyHandleResult::Handled
            }
            KeyCode::BackTab => {
                state.prev_link();
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(url) = state.activate_link() {
                    let url = url.clone();
                    self.load_url(&url);
                }
                KeyHandleResult::Handled
            }

            // Quick link jump (1-9)
            KeyCode::Char(c) if c.is_ascii_digit() && !ctrl => {
                let num = c.to_digit(10).unwrap_or(0) as usize;
                if let Some(url) = state.goto_link(num) {
                    let url = url.clone();
                    self.load_url(&url);
                }
                KeyHandleResult::Handled
            }

            // Go to URL
            KeyCode::Char('g') | KeyCode::Char('G') => {
                state.enter_url_mode();
                KeyHandleResult::Handled
            }

            // Back/Forward
            KeyCode::Left | KeyCode::Char('[') => {
                if let Some((url, scroll)) = state.go_back() {
                    self.load_url_with_scroll(&url, Some(scroll));
                }
                KeyHandleResult::Handled
            }
            KeyCode::Right | KeyCode::Char(']') => {
                if let Some((url, scroll)) = state.go_forward() {
                    self.load_url_with_scroll(&url, Some(scroll));
                }
                KeyHandleResult::Handled
            }

            // Reload
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => {
                let url = state.reload();
                self.load_url(&url);
                KeyHandleResult::Handled
            }

            // Bookmarks
            KeyCode::Char('b') | KeyCode::Char('B') => {
                state.mode = WebMode::Bookmarks;
                KeyHandleResult::Handled
            }

            // Add bookmark
            KeyCode::Char('a') | KeyCode::Char('A') if ctrl => {
                state.add_bookmark();
                KeyHandleResult::Handled
            }

            // History
            KeyCode::Char('h') | KeyCode::Char('H') => {
                state.mode = WebMode::History;
                KeyHandleResult::Handled
            }

            // Search in page
            KeyCode::Char('/') => {
                state.mode = WebMode::Search;
                state.search_query.clear();
                KeyHandleResult::Handled
            }

            // Toggle render mode
            KeyCode::Char('m') | KeyCode::Char('M') => {
                state.render_mode = state.render_mode.next();
                state.status_message = Some((format!("Mode: {}", state.render_mode.name()), 30));
                KeyHandleResult::Handled
            }

            // Save page
            KeyCode::Char('s') if ctrl => {
                state.mode = WebMode::SaveAs;
                state.save_as_input.clear();
                state.save_as_cursor = 0;
                KeyHandleResult::Handled
            }

            // Download (same as save for text pages)
            KeyCode::Char('d') | KeyCode::Char('D') => {
                state.mode = WebMode::SaveAs;
                state.save_as_input.clear();
                state.save_as_cursor = 0;
                KeyHandleResult::Handled
            }

            // New tab
            KeyCode::Char('t') if ctrl => {
                state.new_tab("about:home");
                self.load_url("about:home");
                KeyHandleResult::Handled
            }

            // Close tab
            KeyCode::Char('w') if ctrl => {
                if state.tabs.len() > 1 {
                    state.close_tab();
                } else {
                    return KeyHandleResult::CloseModal;
                }
                KeyHandleResult::Handled
            }

            // Next/prev tab
            KeyCode::Char('n') if ctrl => {
                state.next_tab();
                KeyHandleResult::Handled
            }
            KeyCode::Char('p') if ctrl => {
                state.prev_tab();
                KeyHandleResult::Handled
            }

            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_url_input_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                let url = state.url_input.clone();
                state.mode = WebMode::Browse;
                if !url.is_empty() {
                    self.load_url(&url);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.url_insert(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.url_backspace();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                // Perform search - collect results first to avoid borrow issues
                let query = state.search_query.to_lowercase();
                let results: Vec<usize> = state
                    .current_page()
                    .content
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| line.to_lowercase().contains(&query))
                    .map(|(i, _)| i)
                    .collect();

                state.search_results = results;
                state.search_index = 0;
                if !state.search_results.is_empty() {
                    let line = state.search_results[0];
                    state.current_page_mut().scroll = line.saturating_sub(5);
                    state.status_message =
                        Some((format!("Found {} matches", state.search_results.len()), 30));
                } else {
                    state.status_message = Some(("No matches found".to_string(), 30));
                }
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.search_query.push(c);
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_bookmarks_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if state.bookmarks_selected > 0 {
                    state.bookmarks_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if state.bookmarks_selected < state.bookmarks.len().saturating_sub(1) {
                    state.bookmarks_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(bookmark) = state.bookmarks.get(state.bookmarks_selected) {
                    let url = bookmark.url.clone();
                    state.mode = WebMode::Browse;
                    self.load_url(&url);
                }
                KeyHandleResult::Handled
            }
            KeyCode::Delete => {
                state.remove_bookmark(state.bookmarks_selected);
                if state.bookmarks_selected >= state.bookmarks.len() && state.bookmarks_selected > 0
                {
                    state.bookmarks_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Up => {
                if state.history_selected > 0 {
                    state.history_selected -= 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Down => {
                if state.history_selected < state.global_history.len().saturating_sub(1) {
                    state.history_selected += 1;
                }
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if let Some(entry) = state.global_history.get(state.history_selected) {
                    let url = entry.url.clone();
                    state.mode = WebMode::Browse;
                    self.load_url(&url);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    fn handle_save_as_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        let state = self.state.as_mut().unwrap();

        match key.code {
            KeyCode::Esc => {
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Enter => {
                if !state.save_as_input.is_empty() {
                    let mut path = if state.save_as_input.starts_with('/') {
                        PathBuf::from(&state.save_as_input)
                    } else if let Some(rest) = state.save_as_input.strip_prefix("~/") {
                        if let Some(home) = dirs::home_dir() {
                            home.join(rest)
                        } else {
                            cwd.join(&state.save_as_input)
                        }
                    } else {
                        cwd.join(&state.save_as_input)
                    };

                    // Add .txt extension if none
                    if path.extension().is_none() {
                        path.set_extension("txt");
                    }

                    // Save based on extension
                    let content = if path.extension().map(|e| e == "html").unwrap_or(false) {
                        state.current_page().source.clone()
                    } else {
                        state.current_page().content.join("\n")
                    };

                    match std::fs::write(&path, content) {
                        Ok(()) => {
                            state.status_message =
                                Some((format!("Saved to {}", path.display()), 60));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), 60));
                        }
                    }
                }
                state.mode = WebMode::Browse;
                KeyHandleResult::Handled
            }
            KeyCode::Char(c) => {
                state.save_as_input.insert(state.save_as_cursor, c);
                state.save_as_cursor += 1;
                KeyHandleResult::Handled
            }
            KeyCode::Backspace => {
                if state.save_as_cursor > 0 {
                    state.save_as_cursor -= 1;
                    state.save_as_input.remove(state.save_as_cursor);
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Handled,
        }
    }

    // =========================================================================
    // RENDERING
    // =========================================================================

    pub fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        if let Some(state) = &self.state {
            modal::draw_web_modal(frame, area, state, colors);
        }
    }

    pub fn tick(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.tick_count = state.tick_count.wrapping_add(1);

            // Decrement status message
            if let Some((_, ticks)) = &mut state.status_message {
                if *ticks > 0 {
                    *ticks -= 1;
                } else {
                    state.status_message = None;
                }
            }
        }
    }
}

// =============================================================================
// PLUGIN TRAIT
// =============================================================================

impl Plugin for WebPlugin {
    fn id(&self) -> &str {
        "web"
    }

    fn name(&self) -> &str {
        "Q-WEB"
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
            name: "Q-WEB".to_string(),
            description: "Text-based web browser".to_string(),
            category: PluginCategory::Tools,
            key: 'W',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        WebPlugin::launch(self);
        Ok(())
    }

    fn handle_global_key(
        &mut self,
        _key: KeyEvent,
        _cwd: &PathBuf,
        _selected_file: Option<&PathBuf>,
    ) -> KeyHandleResult {
        KeyHandleResult::NotHandled
    }

    fn handle_modal_key(&mut self, key: KeyEvent, cwd: &PathBuf) -> KeyHandleResult {
        WebPlugin::handle_modal_key(self, key, cwd)
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        WebPlugin::draw_modal(self, frame, area, colors);
    }

    fn tick(&mut self) {
        WebPlugin::tick(self);
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-WEB - Text Web Browser".to_string(),
            "".to_string(),
            "Lynx-style text browser with reader mode".to_string(),
            "and basic form support.".to_string(),
            "".to_string(),
            "NAVIGATION:".to_string(),
            "  G           Go to URL".to_string(),
            "  Enter       Follow selected link".to_string(),
            "  1-9         Jump to link by number".to_string(),
            "  Tab         Next link".to_string(),
            "  Shift+Tab   Previous link".to_string(),
            "  [ / Left    Go back".to_string(),
            "  ] / Right   Go forward".to_string(),
            "  R / F5      Reload page".to_string(),
            "".to_string(),
            "SCROLLING:".to_string(),
            "  Up/Down     Scroll line".to_string(),
            "  PgUp/PgDn   Scroll page".to_string(),
            "  Home/End    Top/bottom".to_string(),
            "".to_string(),
            "FEATURES:".to_string(),
            "  B           Bookmarks".to_string(),
            "  Ctrl+A      Add bookmark".to_string(),
            "  H           History".to_string(),
            "  /           Search in page".to_string(),
            "  M           Toggle render mode".to_string(),
            "  Ctrl+S / D  Save/download page".to_string(),
            "".to_string(),
            "TABS:".to_string(),
            "  Ctrl+T      New tab".to_string(),
            "  Ctrl+W      Close tab".to_string(),
            "  Ctrl+N/P    Next/prev tab".to_string(),
            "".to_string(),
            "  Esc         Close Q-WEB".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// =============================================================================
// URL HELPERS
// =============================================================================

/// Extract the base URL (scheme + host) from a full URL
fn extract_base_url(url: &str) -> Option<String> {
    // Find scheme
    let scheme_end = url.find("://")?;
    let scheme = &url[..scheme_end];

    // Find host end (first / after scheme://)
    let after_scheme = &url[scheme_end + 3..];
    let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
    let host = &after_scheme[..host_end];

    Some(format!("{}://{}", scheme, host))
}

/// Resolve a relative URL against a base URL
fn resolve_relative_url(base_url: &str, relative: &str) -> String {
    // Get the base (scheme + host)
    let base = match extract_base_url(base_url) {
        Some(b) => b,
        None => return format!("https://{}", relative),
    };

    // Get the current path from base URL
    let scheme_end = base_url.find("://").unwrap_or(0) + 3;
    let after_scheme = &base_url[scheme_end..];
    let path_start = after_scheme.find('/').map(|i| scheme_end + i);

    let current_path = match path_start {
        Some(start) => &base_url[start..],
        None => "/",
    };

    // Remove query string and fragment from current path
    let current_path = current_path
        .split('?')
        .next()
        .unwrap_or(current_path)
        .split('#')
        .next()
        .unwrap_or(current_path);

    // Get the directory of current path (remove filename)
    let dir = if current_path.ends_with('/') {
        current_path.to_string()
    } else {
        current_path
            .rfind('/')
            .map(|i| current_path[..=i].to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    // Handle relative path
    if relative.starts_with("./") {
        format!("{}{}{}", base, dir, &relative[2..])
    } else if relative.starts_with("../") {
        // Go up one directory
        let mut path = dir.to_string();
        let mut rel = relative;
        while rel.starts_with("../") {
            // Remove trailing slash and go up
            if path.len() > 1 && path.ends_with('/') {
                path.pop();
            }
            if let Some(idx) = path.rfind('/') {
                path = path[..=idx].to_string();
            }
            rel = &rel[3..];
        }
        format!("{}{}{}", base, path, rel)
    } else {
        format!("{}{}{}", base, dir, relative)
    }
}
