//! Q-CURL: HTTP Client Plugin for QDOS
//!
//! A simple HTTP client inspired by curl. Features:
//! - GET/POST/PUT/DELETE requests
//! - View response headers and body
//! - Save responses to file
//! - JSON pretty printing

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use qdos_plugin_api::prelude::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Head,
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
        }
    }

    fn next(&self) -> HttpMethod {
        match self {
            HttpMethod::Get => HttpMethod::Post,
            HttpMethod::Post => HttpMethod::Put,
            HttpMethod::Put => HttpMethod::Delete,
            HttpMethod::Delete => HttpMethod::Head,
            HttpMethod::Head => HttpMethod::Get,
        }
    }

    fn prev(&self) -> HttpMethod {
        match self {
            HttpMethod::Get => HttpMethod::Head,
            HttpMethod::Post => HttpMethod::Get,
            HttpMethod::Put => HttpMethod::Post,
            HttpMethod::Delete => HttpMethod::Put,
            HttpMethod::Head => HttpMethod::Delete,
        }
    }
}

/// View state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QCurlView {
    #[default]
    Input,
    Response,
}

/// Input field focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputField {
    #[default]
    Url,
    Body,
    Headers,
}

/// HTTP response data
#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration_ms: u64,
}

/// Plugin state
#[derive(Default)]
pub struct QCurlState {
    pub view: QCurlView,
    pub method: HttpMethod,
    pub url: String,
    pub body: String,
    pub headers: String, // Format: "Key: Value\nKey2: Value2"
    pub response: Option<HttpResponse>,
    pub error: Option<String>,
    pub loading: bool,
    pub input_field: InputField,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub show_headers: bool,
}

impl QCurlState {
    pub fn new() -> Self {
        Self {
            url: "https://httpbin.org/get".to_string(),
            cursor_pos: 22,
            ..Default::default()
        }
    }

    fn current_input(&self) -> &str {
        match self.input_field {
            InputField::Url => &self.url,
            InputField::Body => &self.body,
            InputField::Headers => &self.headers,
        }
    }

    fn current_input_mut(&mut self) -> &mut String {
        match self.input_field {
            InputField::Url => &mut self.url,
            InputField::Body => &mut self.body,
            InputField::Headers => &mut self.headers,
        }
    }

    fn insert_char(&mut self, c: char) {
        let input_len = self.current_input().len();
        let cursor = self.cursor_pos;
        if cursor <= input_len {
            self.current_input_mut().insert(cursor, c);
            self.cursor_pos += 1;
        }
    }

    fn delete_char(&mut self) {
        let input_len = self.current_input().len();
        let cursor = self.cursor_pos;
        if cursor > 0 && cursor <= input_len {
            self.current_input_mut().remove(cursor - 1);
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.current_input().len() {
            self.cursor_pos += 1;
        }
    }

    fn execute_request(&mut self) {
        self.loading = true;
        self.error = None;
        self.response = None;

        let start = std::time::Instant::now();

        // Parse custom headers
        let mut custom_headers: Vec<(String, String)> = Vec::new();
        for line in self.headers.lines() {
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                if !key.is_empty() {
                    custom_headers.push((key, value));
                }
            }
        }

        // Execute request and process response
        let process_response = |result: Result<ureq::http::Response<ureq::Body>, ureq::Error>| -> Result<HttpResponse, String> {
            match result {
                Ok(response) => {
                    let status = response.status();

                    // Collect headers
                    let mut headers: Vec<(String, String)> = Vec::new();
                    for (key, value) in response.headers() {
                        if let Ok(v) = value.to_str() {
                            headers.push((key.to_string(), v.to_string()));
                        }
                    }

                    // Read body
                    let body = response.into_body().read_to_string()
                        .unwrap_or_else(|_| "[Binary data]".to_string());

                    // Try to pretty-print JSON
                    let pretty_body = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        serde_json::to_string_pretty(&json).unwrap_or(body)
                    } else {
                        body
                    };

                    Ok(HttpResponse {
                        status: status.as_u16(),
                        status_text: status.to_string(),
                        headers,
                        body: pretty_body,
                        duration_ms: start.elapsed().as_millis() as u64,
                    })
                }
                Err(e) => Err(format!("Request failed: {}", e))
            }
        };

        // Execute based on method type
        // WithoutBody methods use .call(), WithBody methods use .send() or .send_empty()
        let response_result = match self.method {
            HttpMethod::Get => {
                let mut req = ureq::get(&self.url);
                for (key, value) in &custom_headers {
                    req = req.header(key.as_str(), value.as_str());
                }
                process_response(req.call())
            }
            HttpMethod::Delete => {
                let mut req = ureq::delete(&self.url);
                for (key, value) in &custom_headers {
                    req = req.header(key.as_str(), value.as_str());
                }
                process_response(req.call())
            }
            HttpMethod::Head => {
                let mut req = ureq::head(&self.url);
                for (key, value) in &custom_headers {
                    req = req.header(key.as_str(), value.as_str());
                }
                process_response(req.call())
            }
            HttpMethod::Post => {
                let mut req = ureq::post(&self.url);
                for (key, value) in &custom_headers {
                    req = req.header(key.as_str(), value.as_str());
                }
                if self.body.is_empty() {
                    process_response(req.send_empty())
                } else {
                    process_response(req.send(&self.body))
                }
            }
            HttpMethod::Put => {
                let mut req = ureq::put(&self.url);
                for (key, value) in &custom_headers {
                    req = req.header(key.as_str(), value.as_str());
                }
                if self.body.is_empty() {
                    process_response(req.send_empty())
                } else {
                    process_response(req.send(&self.body))
                }
            }
        };

        match response_result {
            Ok(http_response) => {
                self.response = Some(http_response);
                self.view = QCurlView::Response;
            }
            Err(e) => {
                self.error = Some(e);
            }
        }

        self.loading = false;
    }

    fn next_field(&mut self) {
        self.input_field = match self.input_field {
            InputField::Url => InputField::Headers,
            InputField::Headers => InputField::Body,
            InputField::Body => InputField::Url,
        };
        self.cursor_pos = self.current_input().len();
    }

    fn prev_field(&mut self) {
        self.input_field = match self.input_field {
            InputField::Url => InputField::Body,
            InputField::Headers => InputField::Url,
            InputField::Body => InputField::Headers,
        };
        self.cursor_pos = self.current_input().len();
    }
}

/// Q-CURL HTTP Client Plugin
pub struct QCurlPlugin {
    pub state: QCurlState,
}

impl Default for QCurlPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl QCurlPlugin {
    pub fn new() -> Self {
        Self {
            state: QCurlState::new(),
        }
    }

    fn draw_input_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-CURL ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        // Layout: Method/URL, Headers, Body, Status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Method + URL
                Constraint::Length(4), // Headers
                Constraint::Min(3),    // Body
                Constraint::Length(2), // Status
            ])
            .split(content);

        // Method and URL row
        let url_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(10), Constraint::Min(20)])
            .split(chunks[0]);

        // Method selector
        let method_style = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        let method_text = format!(" {} ", state.method.as_str());
        let method_para = Paragraph::new(method_text)
            .style(method_style)
            .block(Block::default().borders(Borders::ALL).title(" Method "));
        frame.render_widget(method_para, url_chunks[0]);

        // URL input
        let url_style = if matches!(state.input_field, InputField::Url) {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };
        let url_display = if matches!(state.input_field, InputField::Url) {
            // Show cursor
            let (before, after) = state.url.split_at(state.cursor_pos.min(state.url.len()));
            format!("{}|{}", before, after)
        } else {
            state.url.clone()
        };
        let url_para = Paragraph::new(format!(" {}", url_display))
            .style(url_style)
            .block(Block::default().borders(Borders::ALL).title(" URL "));
        frame.render_widget(url_para, url_chunks[1]);

        // Headers input
        let headers_style = if matches!(state.input_field, InputField::Headers) {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };
        let headers_text = if state.headers.is_empty() {
            " (Key: Value, one per line)".to_string()
        } else {
            format!(" {}", state.headers.replace('\n', " | "))
        };
        let headers_para = Paragraph::new(headers_text)
            .style(headers_style)
            .block(Block::default().borders(Borders::ALL).title(" Headers "));
        frame.render_widget(headers_para, chunks[1]);

        // Body input
        let body_style = if matches!(state.input_field, InputField::Body) {
            Style::default().fg(colors.yellow())
        } else {
            Style::default().fg(colors.fg())
        };
        let body_text = if state.body.is_empty() && !matches!(state.input_field, InputField::Body) {
            " (Request body for POST/PUT)".to_string()
        } else if matches!(state.input_field, InputField::Body) {
            let (before, after) = state.body.split_at(state.cursor_pos.min(state.body.len()));
            format!(" {}|{}", before, after)
        } else {
            format!(" {}", state.body)
        };
        let body_para = Paragraph::new(body_text)
            .style(body_style)
            .block(Block::default().borders(Borders::ALL).title(" Body "));
        frame.render_widget(body_para, chunks[2]);

        // Status / error
        if let Some(error) = &state.error {
            let error_para = Paragraph::new(format!(" Error: {}", error))
                .style(Style::default().fg(colors.red()));
            frame.render_widget(error_para, chunks[3]);
        } else if state.loading {
            let loading_para =
                Paragraph::new(" Sending request...").style(Style::default().fg(colors.yellow()));
            frame.render_widget(loading_para, chunks[3]);
        }

        view.render_help(
            frame,
            vec![
                ("Tab", "field"),
                ("</>", "method"),
                ("Enter", "send"),
                ("Esc", "close"),
            ],
        );
    }

    fn draw_response_view(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-CURL Response ", colors);
        view.render_frame(frame);
        let content = view.content_area();

        let state = &self.state;

        if let Some(response) = &state.response {
            // Layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Status line
                    Constraint::Min(5),    // Body or headers
                ])
                .split(content);

            // Status line
            let status_color = if response.status < 300 {
                colors.green()
            } else if response.status < 400 {
                colors.yellow()
            } else {
                colors.red()
            };
            let status_line = Line::from(vec![
                Span::styled(
                    format!(" {} ", response.status),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", response.status_text),
                    Style::default().fg(colors.fg()),
                ),
                Span::styled(
                    format!("({}ms)", response.duration_ms),
                    Style::default().fg(colors.grey()),
                ),
            ]);
            frame.render_widget(Paragraph::new(status_line), chunks[0]);

            // Headers or body
            if state.show_headers {
                let mut lines: Vec<Line> = vec![Line::from(Span::styled(
                    " Response Headers:",
                    Style::default()
                        .fg(colors.blue())
                        .add_modifier(Modifier::BOLD),
                ))];
                for (key, value) in &response.headers {
                    lines.push(Line::from(vec![
                        Span::styled(format!(" {}: ", key), Style::default().fg(colors.cyan())),
                        Span::raw(value),
                    ]));
                }
                let headers_para = Paragraph::new(lines);
                frame.render_widget(headers_para, chunks[1]);
            } else {
                // Body with scrolling
                let body_lines: Vec<Line> = response
                    .body
                    .lines()
                    .skip(state.scroll_offset)
                    .map(|line| Line::from(format!(" {}", line)))
                    .collect();

                let total_lines = response.body.lines().count();
                let visible_height = chunks[1].height as usize;

                let body_para = Paragraph::new(body_lines);
                frame.render_widget(body_para, chunks[1]);

                // Scrollbar if needed
                if total_lines > visible_height {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
                    let mut scrollbar_state =
                        ScrollbarState::new(total_lines).position(state.scroll_offset);
                    frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
                }
            }

            view.render_help(
                frame,
                vec![
                    ("H", "headers"),
                    ("Up/Down", "scroll"),
                    ("S", "save"),
                    ("Backspace", "back"),
                    ("Esc", "close"),
                ],
            );
        }
    }
}

impl Plugin for QCurlPlugin {
    fn id(&self) -> &str {
        "qcurl"
    }

    fn name(&self) -> &str {
        "Q-CURL"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            has_modal: true,
            has_menu: false,
            has_keys: false,
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
            id: "qcurl".to_string(),
            name: "Q-CURL".to_string(),
            description: "HTTP client".to_string(),
            category: PluginCategory::Tools,
            key: 'U',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.state = QCurlState::new();
        Ok(())
    }

    fn handle_modal_key(&mut self, key: KeyEvent, _cwd: &PathBuf) -> KeyHandleResult {
        let state = &mut self.state;

        match state.view {
            QCurlView::Input => match key.code {
                KeyCode::Esc => return KeyHandleResult::CloseModal,
                KeyCode::Tab => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        state.prev_field();
                    } else {
                        state.next_field();
                    }
                }
                KeyCode::Enter => {
                    if !state.url.is_empty() {
                        state.execute_request();
                    }
                }
                KeyCode::Char('<') | KeyCode::Left
                    if key.modifiers.contains(KeyModifiers::SHIFT)
                        || matches!(state.input_field, InputField::Url)
                            && state.cursor_pos == 0 =>
                {
                    state.method = state.method.prev();
                }
                KeyCode::Char('>') | KeyCode::Right
                    if key.modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    state.method = state.method.next();
                }
                KeyCode::Left => state.move_cursor_left(),
                KeyCode::Right => state.move_cursor_right(),
                KeyCode::Backspace => state.delete_char(),
                KeyCode::Char(c) => state.insert_char(c),
                KeyCode::Home => state.cursor_pos = 0,
                KeyCode::End => state.cursor_pos = state.current_input().len(),
                _ => return KeyHandleResult::NotHandled,
            },
            QCurlView::Response => {
                match key.code {
                    KeyCode::Esc => return KeyHandleResult::CloseModal,
                    KeyCode::Backspace => {
                        state.view = QCurlView::Input;
                        state.scroll_offset = 0;
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        state.show_headers = !state.show_headers;
                    }
                    KeyCode::Up => {
                        if state.scroll_offset > 0 {
                            state.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down => {
                        state.scroll_offset += 1;
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(20);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset += 20;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        // Save response to file
                        if let Some(response) = &state.response {
                            let filename = format!(
                                "response_{}.txt",
                                chrono::Local::now().format("%Y%m%d_%H%M%S")
                            );
                            if let Some(downloads) = dirs::download_dir() {
                                let path = downloads.join(&filename);
                                if std::fs::write(&path, &response.body).is_ok() {
                                    return KeyHandleResult::CloseWithSuccess(format!(
                                        "Saved to {}",
                                        path.display()
                                    ));
                                }
                            }
                        }
                    }
                    _ => return KeyHandleResult::NotHandled,
                }
            }
        }

        KeyHandleResult::Handled
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        // Clear area
        frame.render_widget(Clear, area);

        match self.state.view {
            QCurlView::Input => self.draw_input_view(frame, area, colors),
            QCurlView::Response => self.draw_response_view(frame, area, colors),
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-CURL - HTTP Client".to_string(),
            "".to_string(),
            "A simple HTTP client for making web requests.".to_string(),
            "".to_string(),
            "Features:".to_string(),
            "  - GET, POST, PUT, DELETE, HEAD methods".to_string(),
            "  - Custom headers support".to_string(),
            "  - Request body for POST/PUT".to_string(),
            "  - JSON pretty printing".to_string(),
            "  - Save responses to file".to_string(),
            "".to_string(),
            "Keybindings:".to_string(),
            "  Tab/Shift+Tab  - Switch between URL/Headers/Body".to_string(),
            "  </> or Shift+Arrows - Change HTTP method".to_string(),
            "  Enter          - Send request".to_string(),
            "  H              - Toggle headers view".to_string(),
            "  S              - Save response".to_string(),
            "  Up/Down        - Scroll response".to_string(),
            "  Backspace      - Back to input".to_string(),
            "  Esc            - Close".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Self-registration
inventory::submit! {
    PluginRegistration::new("qcurl", || Box::new(QCurlPlugin::new()))
}
