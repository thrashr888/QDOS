//! PTY session management for interactive shell support
//!
//! Provides PTY allocation and terminal emulation for running
//! interactive programs like bash, vim, htop.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tui_term::vt100;

/// An interactive PTY session
pub struct PtySession {
    /// PTY master for read/write (wrapped in Mutex for Sync)
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// Child process
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    /// VT100 terminal parser
    parser: Arc<Mutex<vt100::Parser>>,
    /// Current terminal size
    size: PtySize,
}

impl PtySession {
    /// Spawn a new interactive shell session
    pub fn spawn(shell: Option<&str>, cwd: &PathBuf, cols: u16, rows: u16) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size)?;

        // Use provided shell or detect from environment
        let shell_cmd = shell
            .map(|s| s.to_string())
            .unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()));

        let mut cmd = CommandBuilder::new(&shell_cmd);
        cmd.cwd(cwd);

        // Set TERM for proper terminal emulation
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)?;

        // Create VT100 parser with scrollback
        let parser = vt100::Parser::new(rows, cols, 1000);

        Ok(Self {
            master: Arc::new(Mutex::new(pair.master)),
            child: Arc::new(Mutex::new(child)),
            parser: Arc::new(Mutex::new(parser)),
            size,
        })
    }

    /// Write input bytes to the PTY (keystrokes from user)
    pub fn write(&self, data: &[u8]) -> anyhow::Result<()> {
        let master = self.master.lock().unwrap();
        let mut writer = master.take_writer()?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    /// Read available output from the PTY and process through VT100 parser
    pub fn read_and_process(&self) -> anyhow::Result<bool> {
        let master = self.master.lock().unwrap();
        let mut reader = master.try_clone_reader()?;
        drop(master); // Release lock before blocking read

        // Non-blocking read - try to read available data
        let mut buf = [0u8; 4096];

        // Note: portable-pty doesn't directly support non-blocking,
        // so we use a short timeout approach via try_clone_reader
        match reader.read(&mut buf) {
            Ok(0) => Ok(false), // EOF or no data
            Ok(n) => {
                let mut parser = self.parser.lock().unwrap();
                parser.process(&buf[..n]);
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Try to read with a timeout (non-blocking check)
    pub fn try_read(&self) -> anyhow::Result<bool> {
        // Check if child is still running
        {
            let mut child = self.child.lock().unwrap();
            if let Some(_status) = child.try_wait()? {
                return Ok(false);
            }
        }

        // Try to read any available output
        self.read_and_process()
    }

    /// Get reference to the VT100 parser's screen for rendering
    pub fn screen(&self) -> Arc<Mutex<vt100::Parser>> {
        Arc::clone(&self.parser)
    }

    /// Resize the PTY
    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let new_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let master = self.master.lock().unwrap();
        master.resize(new_size)?;
        drop(master);

        let mut parser = self.parser.lock().unwrap();
        parser.set_size(rows, cols);
        Ok(())
    }

    /// Check if the child process is still running
    pub fn is_running(&self) -> bool {
        let mut child = self.child.lock().unwrap();
        child.try_wait().ok().flatten().is_none()
    }

    /// Get current size
    pub fn size(&self) -> (u16, u16) {
        (self.size.cols, self.size.rows)
    }
}

/// Convert a crossterm KeyEvent to bytes for the PTY
pub fn key_to_bytes(key: crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let bytes = match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Control characters: Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                let ctrl_char = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a' - 1);
                if ctrl_char <= 26 {
                    vec![ctrl_char]
                } else {
                    vec![c as u8]
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                // Alt key sends ESC prefix
                vec![0x1b, c as u8]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::F(1) => b"\x1bOP".to_vec(),
        KeyCode::F(2) => b"\x1bOQ".to_vec(),
        KeyCode::F(3) => b"\x1bOR".to_vec(),
        KeyCode::F(4) => b"\x1bOS".to_vec(),
        KeyCode::F(5) => b"\x1b[15~".to_vec(),
        KeyCode::F(6) => b"\x1b[17~".to_vec(),
        KeyCode::F(7) => b"\x1b[18~".to_vec(),
        KeyCode::F(8) => b"\x1b[19~".to_vec(),
        KeyCode::F(9) => b"\x1b[20~".to_vec(),
        KeyCode::F(10) => b"\x1b[21~".to_vec(),
        KeyCode::F(11) => b"\x1b[23~".to_vec(),
        KeyCode::F(12) => b"\x1b[24~".to_vec(),
        _ => return None,
    };

    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_to_bytes_char() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_to_bytes(key), Some(vec![b'a']));
    }

    #[test]
    fn test_key_to_bytes_ctrl_c() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_to_bytes(key), Some(vec![0x03]));
    }

    #[test]
    fn test_key_to_bytes_enter() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_to_bytes(key), Some(vec![b'\r']));
    }

    #[test]
    fn test_key_to_bytes_arrow_up() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_to_bytes(key), Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn test_key_to_bytes_alt_char() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
        assert_eq!(key_to_bytes(key), Some(vec![0x1b, b'x']));
    }
}
