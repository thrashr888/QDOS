//! BASIC Runner plugin state types
//!
//! State for the BASIC interpreter plugin.

use std::path::PathBuf;

/// Available BASIC interpreters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicInterpreter {
    /// bas55 - Minimal ANSI BASIC (Ecma-55)
    Bas55,
    /// pc-basic - GW-BASIC/BASICA compatible
    PcBasic,
    /// bwbasic - Bywater BASIC
    BwBasic,
    /// cbmbasic - Commodore 64 BASIC
    CbmBasic,
}

impl BasicInterpreter {
    pub fn command(&self) -> &'static str {
        match self {
            BasicInterpreter::Bas55 => "bas55",
            BasicInterpreter::PcBasic => "pc-basic",
            BasicInterpreter::BwBasic => "bwbasic",
            BasicInterpreter::CbmBasic => "cbmbasic",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BasicInterpreter::Bas55 => "bas55 (ANSI BASIC)",
            BasicInterpreter::PcBasic => "PC-BASIC (GW-BASIC)",
            BasicInterpreter::BwBasic => "Bywater BASIC",
            BasicInterpreter::CbmBasic => "CBM BASIC (C64)",
        }
    }

    pub fn install_hint(&self) -> &'static str {
        match self {
            BasicInterpreter::Bas55 => "brew install bas55",
            BasicInterpreter::PcBasic => "brew install pc-basic",
            BasicInterpreter::BwBasic => "brew install bwbasic",
            BasicInterpreter::CbmBasic => "brew install cbmbasic",
        }
    }

    /// Ranking for recommendation (lower = better)
    /// cbmbasic: Good C64 compatibility, flexible syntax
    /// pc-basic: Full GW-BASIC, most features
    /// bwbasic: Modern but less retro
    /// bas55: Very strict Ecma-55, uppercase only, limited IF-THEN
    pub fn rank(&self) -> u8 {
        match self {
            BasicInterpreter::CbmBasic => 1,
            BasicInterpreter::PcBasic => 2,
            BasicInterpreter::BwBasic => 3,
            BasicInterpreter::Bas55 => 4,
        }
    }

    /// Check if this interpreter is available
    pub fn is_available(&self) -> bool {
        // Use 'which' to check if command exists, since some interpreters
        // don't support --version (e.g., cbmbasic crashes with it)
        std::process::Command::new("which")
            .arg(self.command())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get all interpreter variants in ranked order (best first)
    pub fn all() -> &'static [BasicInterpreter] {
        &[
            BasicInterpreter::CbmBasic,
            BasicInterpreter::PcBasic,
            BasicInterpreter::BwBasic,
            BasicInterpreter::Bas55,
        ]
    }
}

/// Current view in the BASIC plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BasicView {
    #[default]
    /// Main menu showing interpreter selection
    Menu,
    /// Running a BASIC program
    Running,
    /// Viewing output from a run
    Output,
    /// Error state
    Error,
}

/// BASIC plugin state
#[derive(Debug, Clone, Default)]
pub struct BasicState {
    /// Current view
    pub view: BasicView,
    /// Available interpreters (detected at init)
    pub available_interpreters: Vec<BasicInterpreter>,
    /// Currently selected interpreter index
    pub selected_interpreter: usize,
    /// File to run (if any)
    pub file_path: Option<PathBuf>,
    /// Output from last run
    pub output: Vec<String>,
    /// Scroll position in output
    pub scroll_offset: usize,
    /// Error message (if any)
    pub error: Option<String>,
    /// Whether currently running
    pub is_running: bool,
}

impl BasicState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect available interpreters
    pub fn detect_interpreters(&mut self) {
        self.available_interpreters = BasicInterpreter::all()
            .iter()
            .filter(|i| i.is_available())
            .copied()
            .collect();
    }

    /// Get currently selected interpreter
    pub fn selected(&self) -> Option<&BasicInterpreter> {
        self.available_interpreters.get(self.selected_interpreter)
    }

    /// Select previous interpreter
    pub fn select_prev(&mut self) {
        if self.selected_interpreter > 0 {
            self.selected_interpreter -= 1;
        }
    }

    /// Select next interpreter
    pub fn select_next(&mut self) {
        let max = self.available_interpreters.len().saturating_sub(1);
        if self.selected_interpreter < max {
            self.selected_interpreter += 1;
        }
    }

    /// Scroll output up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll output down
    pub fn scroll_down(&mut self, visible_lines: usize) {
        let max_scroll = self.output.len().saturating_sub(visible_lines);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }
}
