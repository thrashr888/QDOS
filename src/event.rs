use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Terminal events
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Terminal tick (for animations/updates)
    Tick,
    /// Terminal resize
    Resize(u16, u16),
    /// Directory content changed (file created, modified, deleted)
    DirChanged,
}

/// Event handler for terminal events
pub struct EventHandler {
    /// Tick rate in milliseconds
    tick_rate: u64,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: u64) -> Self {
        Self { tick_rate }
    }

    /// Get the next event
    pub async fn next(&self) -> Result<Option<Event>> {
        if event::poll(Duration::from_millis(self.tick_rate))? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    // Ignore key release events on some platforms
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        return Ok(Some(Event::Key(key)));
                    }
                }
                CrosstermEvent::Resize(width, height) => {
                    return Ok(Some(Event::Resize(width, height)));
                }
                _ => {}
            }
        }

        Ok(Some(Event::Tick))
    }
}
