//! Emulator plugin state

use std::path::PathBuf;

/// Supported emulator types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulatorType {
    DosBox,
}

impl EmulatorType {
    pub fn name(&self) -> &'static str {
        match self {
            EmulatorType::DosBox => "DOSBox-X",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EmulatorType::DosBox => "DOS emulator for running .EXE, .COM, and .BAT files",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            EmulatorType::DosBox => "dosbox-x",
        }
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            EmulatorType::DosBox => &["exe", "com", "bat"],
        }
    }
}

/// Current view state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmulatorView {
    #[default]
    Menu,
    FileSelect,
    Running,
    NotAvailable,
}

/// Detected game/program entry
#[derive(Debug, Clone)]
pub struct EmulatorEntry {
    pub path: PathBuf,
    pub name: String,
    pub emulator: EmulatorType,
}

/// Main emulator plugin state
pub struct EmulatorState {
    pub view: EmulatorView,
    pub selected: usize,
    pub dosbox_available: bool,
    pub file_path: Option<PathBuf>,
    pub entries: Vec<EmulatorEntry>,
    pub error: Option<String>,
    pub scroll_offset: usize,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            view: EmulatorView::Menu,
            selected: 0,
            dosbox_available: false,
            file_path: None,
            entries: Vec::new(),
            error: None,
            scroll_offset: 0,
        }
    }

    pub fn detect_emulators(&mut self) {
        // Check for DOSBox-X
        self.dosbox_available = std::process::Command::new("dosbox-x")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok();
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
            self.ensure_visible();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + self.entries.len() - 1) % self.entries.len();
            self.ensure_visible();
        }
    }

    fn ensure_visible(&mut self) {
        let visible_height = 15; // Approximate visible lines
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    /// Check if a path can be run in an emulator
    pub fn can_run(&self, path: &PathBuf) -> Option<EmulatorType> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();

        if self.dosbox_available {
            for valid_ext in EmulatorType::DosBox.extensions() {
                if ext == *valid_ext {
                    // Optionally check for DOS MZ header for .exe files
                    if ext == "exe" && !is_dos_executable(path) {
                        continue;
                    }
                    return Some(EmulatorType::DosBox);
                }
            }
        }

        None
    }
}

/// Check if a file is a DOS executable (has MZ header)
pub fn is_dos_executable(path: &PathBuf) -> bool {
    use std::fs::File;
    use std::io::Read;

    if let Ok(mut file) = File::open(path) {
        let mut header = [0u8; 64]; // Read enough for PE check
        if file.read_exact(&mut header).is_ok() {
            // Check for MZ header
            if header[0] == 0x4D && header[1] == 0x5A {
                // "MZ"
                // Check if it's a PE (Windows) executable - skip those
                // PE executables have "PE\0\0" at the offset specified at 0x3C
                let pe_offset =
                    u32::from_le_bytes([header[60], header[61], header[62], header[63]]) as usize;
                if pe_offset < 1024 {
                    // Reasonable offset
                    let mut pe_sig = [0u8; 4];
                    if let Ok(mut f) = File::open(path) {
                        use std::io::Seek;
                        if f.seek(std::io::SeekFrom::Start(pe_offset as u64)).is_ok()
                            && f.read_exact(&mut pe_sig).is_ok()
                            && &pe_sig == b"PE\0\0"
                        {
                            return false; // It's a Windows PE executable
                        }
                    }
                }
                return true; // DOS MZ executable
            }
        }
    }
    false
}
