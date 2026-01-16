//! Q-DOS Office Suite
//!
//! A collection of productivity applications for R-DOS.
//!
//! Currently includes:
//! - Q-SHEET: Spreadsheet editor with formulas and CSV support
//! - Q-DECK: Presentation editor with ANSI art and sixel images
//! - Q-WEB: Text-based web browser with reader mode
//! - Q-DOCS: Word processor with Markdown support

pub mod deck;
pub mod docs;
pub mod shared;
pub mod sheet;
pub mod web;

use crate::app::ThemeColors;
use crate::plugins::{AppEntry, KeyHandleResult, Plugin, PluginCapabilities, PluginCategory};
use crate::ui::components::FullScreenView;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    Frame,
};
use std::any::Any;
use std::path::PathBuf;

pub use deck::DeckPlugin;
pub use docs::DocsPlugin;
pub use sheet::SheetPlugin;
pub use web::WebPlugin;

// =============================================================================
// OFFICE VIEW
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OfficeView {
    #[default]
    Menu,
    App(OfficeApp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeApp {
    Sheet,
    Deck,
    Web,
    Docs,
}

// =============================================================================
// OFFICE APP INFO
// =============================================================================

struct OfficeAppInfo {
    id: OfficeApp,
    name: &'static str,
    description: &'static str,
    key: char,
}

const OFFICE_APPS: &[OfficeAppInfo] = &[
    OfficeAppInfo {
        id: OfficeApp::Sheet,
        name: "Q-SHEET",
        description: "Spreadsheet editor with formulas and CSV support",
        key: '1',
    },
    OfficeAppInfo {
        id: OfficeApp::Deck,
        name: "Q-DECK",
        description: "Presentation editor with ANSI art and sixel images",
        key: '2',
    },
    OfficeAppInfo {
        id: OfficeApp::Web,
        name: "Q-WEB",
        description: "Text browser with reader mode and bookmarks",
        key: '3',
    },
    OfficeAppInfo {
        id: OfficeApp::Docs,
        name: "Q-DOCS",
        description: "Word processor with Markdown support",
        key: '4',
    },
];

// =============================================================================
// OFFICE PLUGIN
// =============================================================================

/// Office Suite plugin container
///
/// Provides a launcher menu for all office applications.
/// Individual apps can also be launched directly via separate plugin registrations.
pub struct OfficePlugin {
    view: OfficeView,
    selected: usize,
    sheet: SheetPlugin,
    deck: DeckPlugin,
    web: WebPlugin,
    docs: DocsPlugin,
}

impl Default for OfficePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl OfficePlugin {
    pub fn new() -> Self {
        Self {
            view: OfficeView::Menu,
            selected: 0,
            sheet: SheetPlugin::new(),
            deck: DeckPlugin::new(),
            web: WebPlugin::new(),
            docs: DocsPlugin::new(),
        }
    }

    fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn select_next(&mut self) {
        if self.selected < OFFICE_APPS.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    fn launch_selected(&mut self) {
        if let Some(app) = OFFICE_APPS.get(self.selected) {
            self.launch_app(app.id);
        }
    }

    fn launch_app(&mut self, app: OfficeApp) {
        match app {
            OfficeApp::Sheet => {
                self.sheet.launch();
                self.view = OfficeView::App(OfficeApp::Sheet);
            }
            OfficeApp::Deck => {
                self.deck.launch();
                self.view = OfficeView::App(OfficeApp::Deck);
            }
            OfficeApp::Web => {
                self.web.launch();
                self.view = OfficeView::App(OfficeApp::Web);
            }
            OfficeApp::Docs => {
                self.docs.launch();
                self.view = OfficeView::App(OfficeApp::Docs);
            }
        }
    }

    fn back_to_menu(&mut self) {
        self.view = OfficeView::Menu;
    }

    fn draw_menu(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        let view = FullScreenView::new(area, " Q-DOS Office Suite ", colors);
        view.render_frame(frame);

        let normal = Style::default().fg(colors.fg());
        let highlight = Style::default()
            .fg(colors.yellow())
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(colors.grey());

        // Header
        view.render_row(
            frame,
            0,
            vec![Span::styled(
                "Select an application to launch:",
                Style::default().fg(colors.cyan()),
            )],
        );
        view.render_row(frame, 1, vec![Span::raw("")]);

        // App list
        for (i, app) in OFFICE_APPS.iter().enumerate() {
            let is_selected = i == self.selected;
            let style = if is_selected { highlight } else { normal };

            let marker = if is_selected { ">" } else { " " };
            let line = format!(" {} [{}] {}", marker, app.key, app.name);

            view.render_row(frame, 2 + i as u16 * 2, vec![Span::styled(line, style)]);
            view.render_row(
                frame,
                3 + i as u16 * 2,
                vec![Span::styled(
                    format!("       {}", app.description),
                    desc_style,
                )],
            );
        }

        // Help
        view.render_help(
            frame,
            vec![
                ("↑↓", "select"),
                ("Enter", "launch"),
                ("1-9", "quick launch"),
                ("Esc", "close"),
            ],
        );
    }
}

impl Plugin for OfficePlugin {
    fn id(&self) -> &str {
        "office"
    }

    fn name(&self) -> &str {
        "Office Suite"
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
            name: "Office Suite".to_string(),
            description: "Q-DOS productivity applications".to_string(),
            category: PluginCategory::Tools,
            key: 'O',
        })
    }

    fn launch(&mut self, _cwd: &PathBuf, _selected_file: Option<&PathBuf>) -> Result<(), String> {
        self.view = OfficeView::Menu;
        self.selected = 0;
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
        match self.view {
            OfficeView::Menu => match key.code {
                KeyCode::Esc => KeyHandleResult::CloseModal,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_prev();
                    KeyHandleResult::Handled
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    KeyHandleResult::Handled
                }
                KeyCode::Enter => {
                    self.launch_selected();
                    KeyHandleResult::Handled
                }
                KeyCode::Char('1') => {
                    self.launch_app(OfficeApp::Sheet);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('2') => {
                    self.launch_app(OfficeApp::Deck);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('3') => {
                    self.launch_app(OfficeApp::Web);
                    KeyHandleResult::Handled
                }
                KeyCode::Char('4') => {
                    self.launch_app(OfficeApp::Docs);
                    KeyHandleResult::Handled
                }
                _ => KeyHandleResult::Handled,
            },
            OfficeView::App(OfficeApp::Sheet) => {
                let result = self.sheet.handle_modal_key(key, cwd);
                if matches!(
                    result,
                    KeyHandleResult::CloseModal
                        | KeyHandleResult::CloseWithSuccess(_)
                        | KeyHandleResult::CloseWithError(_)
                ) {
                    self.back_to_menu();
                    return KeyHandleResult::Handled; // Stay in office menu
                }
                result
            }
            OfficeView::App(OfficeApp::Deck) => {
                let result = self.deck.handle_modal_key(key, cwd);
                if matches!(
                    result,
                    KeyHandleResult::CloseModal
                        | KeyHandleResult::CloseWithSuccess(_)
                        | KeyHandleResult::CloseWithError(_)
                ) {
                    self.back_to_menu();
                    return KeyHandleResult::Handled; // Stay in office menu
                }
                result
            }
            OfficeView::App(OfficeApp::Web) => {
                let result = self.web.handle_modal_key(key, cwd);
                if matches!(
                    result,
                    KeyHandleResult::CloseModal
                        | KeyHandleResult::CloseWithSuccess(_)
                        | KeyHandleResult::CloseWithError(_)
                ) {
                    self.back_to_menu();
                    return KeyHandleResult::Handled; // Stay in office menu
                }
                result
            }
            OfficeView::App(OfficeApp::Docs) => {
                let result = self.docs.handle_modal_key(key, cwd);
                if matches!(
                    result,
                    KeyHandleResult::CloseModal
                        | KeyHandleResult::CloseWithSuccess(_)
                        | KeyHandleResult::CloseWithError(_)
                ) {
                    self.back_to_menu();
                    return KeyHandleResult::Handled; // Stay in office menu
                }
                result
            }
        }
    }

    fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
        match self.view {
            OfficeView::Menu => self.draw_menu(frame, area, colors),
            OfficeView::App(OfficeApp::Sheet) => self.sheet.draw_modal(frame, area, colors),
            OfficeView::App(OfficeApp::Deck) => self.deck.draw_modal(frame, area, colors),
            OfficeView::App(OfficeApp::Web) => self.web.draw_modal(frame, area, colors),
            OfficeView::App(OfficeApp::Docs) => self.docs.draw_modal(frame, area, colors),
        }
    }

    fn tick(&mut self) {
        match self.view {
            OfficeView::App(OfficeApp::Sheet) => self.sheet.tick(),
            OfficeView::App(OfficeApp::Deck) => self.deck.tick(),
            OfficeView::App(OfficeApp::Web) => self.web.tick(),
            OfficeView::App(OfficeApp::Docs) => self.docs.tick(),
            OfficeView::Menu => {}
        }
    }

    fn help_content(&self) -> Vec<String> {
        vec![
            "Q-DOS Office Suite".to_string(),
            "".to_string(),
            "A collection of productivity applications.".to_string(),
            "".to_string(),
            "Applications:".to_string(),
            "".to_string(),
            "Q-SHEET - Spreadsheet Editor".to_string(),
            "  Arrow keys    Navigate cells".to_string(),
            "  Tab           Move right".to_string(),
            "  Enter         Move down / Confirm edit".to_string(),
            "  F2            Edit cell".to_string(),
            "  Typing        Start entering value".to_string(),
            "  Esc           Cancel edit / Close".to_string(),
            "  Ctrl+S        Save file".to_string(),
            "".to_string(),
            "Formulas (start with =):".to_string(),
            "  =SUM(A1:A10)      Sum of range".to_string(),
            "  =AVG(B1:B5)       Average".to_string(),
            "  =COUNT(C1:C10)    Count non-empty".to_string(),
            "  =MIN(D1:D5)       Minimum value".to_string(),
            "  =MAX(E1:E5)       Maximum value".to_string(),
            "  =IF(A1>10,1,0)    Conditional".to_string(),
        ]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
