//! Directory watcher for event-based file updates
//!
//! Uses the notify crate to watch the current directory for changes,
//! providing event-based updates instead of polling.

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

/// Directory watcher that monitors the current directory for changes
pub struct DirWatcher {
    /// The notify watcher instance
    _watcher: RecommendedWatcher,
    /// Receiver for change events
    rx: Receiver<()>,
    /// Currently watched path
    watched_path: PathBuf,
}

impl DirWatcher {
    /// Create a new directory watcher for the given path
    pub fn new(path: &PathBuf) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        // Create watcher with debounced events (100ms)
        let config = Config::default().with_poll_interval(Duration::from_millis(100));

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    // Only trigger on meaningful file events
                    use notify::EventKind::*;
                    match event.kind {
                        Create(_) | Modify(_) | Remove(_) => {
                            // Send a signal that something changed
                            let _ = tx.send(());
                        }
                        _ => {}
                    }
                }
            },
            config,
        )?;

        // Watch the directory non-recursively (just immediate children)
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
            watched_path: path.clone(),
        })
    }

    /// Check if there are any pending directory changes
    /// Returns true if files have changed since last check
    pub fn has_changes(&self) -> bool {
        // Drain all pending events and return true if any exist
        let mut changed = false;
        loop {
            match self.rx.try_recv() {
                Ok(()) => changed = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        changed
    }

    /// Update the watched path
    pub fn watch_path(&mut self, new_path: &PathBuf) -> Result<(), notify::Error> {
        if *new_path != self.watched_path {
            // Create new watcher for the new path
            let (tx, rx) = channel();

            let config = Config::default().with_poll_interval(Duration::from_millis(100));

            let mut watcher = RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        use notify::EventKind::*;
                        match event.kind {
                            Create(_) | Modify(_) | Remove(_) => {
                                let _ = tx.send(());
                            }
                            _ => {}
                        }
                    }
                },
                config,
            )?;

            watcher.watch(new_path, RecursiveMode::NonRecursive)?;

            self._watcher = watcher;
            self.rx = rx;
            self.watched_path = new_path.clone();
        }
        Ok(())
    }

}
