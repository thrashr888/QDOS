//! Native terminal progress for batch file operations (ConEmu OSC 9;4).
//!
//! Ghostty expires progress after about 15 seconds. A dedicated worker keeps the
//! current state alive even while a synchronous file copy blocks the UI thread.
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    percent: u8,
    failed: bool,
}

impl Progress {
    pub fn new(done: usize, total: usize, failed: bool) -> Self {
        let percent = if total == 0 {
            100
        } else {
            ((done.min(total) as u128 * 100) / total as u128) as u8
        };
        Self { percent, failed }
    }

    fn sequence(self) -> String {
        format!(
            "\x1b]9;4;{};{}\x1b\\",
            if self.failed { 2 } else { 1 },
            self.percent
        )
    }
}

const CLEAR: &str = "\x1b]9;4;0\x1b\\";

fn enabled(tty: bool, setting: &str, program: &str, term: &str, multiplexed: bool) -> bool {
    if !tty || setting == "off" {
        return false;
    }
    if setting == "on" {
        return true;
    }
    !multiplexed
        && program != "tmux"
        && program != "screen"
        && (program == "ghostty" || term == "xterm-ghostty")
}

/// Scoped reporter: clears on normal return, cancellation, errors and unwinding.
/// No clipboard access, terminal queries, or output on redirected stdout.
pub struct TerminalProgress {
    sender: Option<Sender<Option<Progress>>>,
    worker: Option<JoinHandle<()>>,
    previous: Option<Progress>,
}

impl Default for TerminalProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalProgress {
    pub fn new() -> Self {
        let env = |key| std::env::var(key).unwrap_or_default();
        let term = env("TERM");
        let allowed = enabled(
            io::stdout().is_terminal(),
            &env("RDOS_TERMINAL_PROGRESS"),
            &env("TERM_PROGRAM"),
            &term,
            !env("TMUX").is_empty()
                || !env("STY").is_empty()
                || term.starts_with("screen")
                || term.starts_with("tmux"),
        );
        let mut reporter = Self {
            sender: None,
            worker: None,
            previous: None,
        };
        if allowed {
            let (sender, receiver) = mpsc::channel();
            if let Ok(worker) = thread::Builder::new()
                .name("terminal-progress".into())
                .spawn(move || {
                    report(receiver, Duration::from_secs(1), |sequence| {
                        let mut stdout = io::stdout().lock();
                        stdout.write_all(sequence.as_bytes())?;
                        stdout.flush()
                    });
                })
            {
                reporter.sender = Some(sender);
                reporter.worker = Some(worker);
            }
        }
        reporter
    }

    pub fn update(&mut self, progress: Option<Progress>) {
        if self.previous != progress {
            if let Some(sender) = &self.sender {
                let _ = sender.send(progress);
            }
            self.previous = progress;
        }
    }
}

impl Drop for TerminalProgress {
    fn drop(&mut self) {
        // Disconnect wakes the worker immediately, clears any active indicator,
        // and joins before terminal teardown or an error is printed.
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn report(
    receiver: Receiver<Option<Progress>>,
    heartbeat: Duration,
    mut write: impl FnMut(&str) -> io::Result<()>,
) {
    let mut active: Option<Progress> = None;
    loop {
        match receiver.recv_timeout(heartbeat) {
            Ok(next) => {
                if next.is_none() && active.is_none() {
                    continue;
                }
                active = next;
            }
            Err(RecvTimeoutError::Timeout) if active.is_none() => continue,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if active.is_some() {
                    let _ = write(CLEAR);
                }
                return;
            }
        }
        let sequence = active
            .map(Progress::sequence)
            .unwrap_or_else(|| CLEAR.to_owned());
        if write(&sequence).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_detection_and_override() {
        assert!(enabled(true, "", "ghostty", "xterm-256color", false));
        assert!(enabled(true, "auto", "", "xterm-ghostty", false));
        assert!(!enabled(false, "on", "ghostty", "xterm-ghostty", false));
        assert!(!enabled(true, "off", "ghostty", "xterm-ghostty", false));
        assert!(!enabled(true, "", "ghostty", "screen", true));
        assert!(!enabled(true, "", "unknown", "xterm", false));
        assert!(enabled(true, "on", "unknown", "screen", true));
        assert!(!enabled(true, "", "tmux", "xterm-ghostty", false));
    }

    #[test]
    fn sequences_are_bounded_and_terminated() {
        assert_eq!(Progress::new(1, 4, false).sequence(), "\x1b]9;4;1;25\x1b\\");
        assert_eq!(
            Progress::new(usize::MAX, 1, true).sequence(),
            "\x1b]9;4;2;100\x1b\\"
        );
        assert_eq!(Progress::new(0, 0, false).percent, 100);
        assert_eq!(Progress::new(usize::MAX, usize::MAX, false).percent, 100);
    }

    #[test]
    fn heartbeat_and_cancellation_clear() {
        let (tx, rx) = mpsc::channel();
        let (output, observed) = mpsc::channel();
        let worker = thread::spawn(move || {
            report(rx, Duration::from_millis(10), |s| {
                output.send(s.to_owned()).unwrap();
                Ok(())
            })
        });
        let progress = Progress::new(0, 2, false);
        tx.send(Some(progress)).unwrap();
        assert_eq!(
            observed.recv_timeout(Duration::from_secs(2)).unwrap(),
            progress.sequence()
        );
        assert_eq!(
            observed.recv_timeout(Duration::from_secs(2)).unwrap(),
            progress.sequence()
        );
        tx.send(None).unwrap();
        // A heartbeat can already be in flight; consume through the clear.
        while observed.recv_timeout(Duration::from_secs(2)).unwrap() != CLEAR {}
        drop(tx);
        worker.join().unwrap();
        assert!(observed.try_recv().is_err());
    }

    #[test]
    fn idle_disconnect_does_not_emit_or_clear_someone_elses_progress() {
        let (tx, rx) = mpsc::channel();
        tx.send(None).unwrap();
        drop(tx);
        report(rx, Duration::from_secs(1), |_| {
            panic!("idle reporter wrote output")
        });
    }

    #[test]
    fn output_failure_stops_reporting() {
        let (tx, rx) = mpsc::channel();
        tx.send(Some(Progress::new(0, 2, false))).unwrap();
        let mut calls = 0;
        report(rx, Duration::from_secs(1), |_| {
            calls += 1;
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        });
        assert_eq!(calls, 1);
    }

    #[test]
    fn disconnect_clears_active_progress() {
        let (tx, rx) = mpsc::channel();
        tx.send(Some(Progress::new(1, 2, false))).unwrap();
        drop(tx);
        let mut output = Vec::new();
        report(rx, Duration::from_secs(1), |s| {
            output.push(s.to_owned());
            Ok(())
        });
        assert_eq!(
            output,
            vec![Progress::new(1, 2, false).sequence(), CLEAR.to_owned()]
        );
    }
}
