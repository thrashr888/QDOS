//! Telnet session management
//!
//! Provides telnet connectivity with terminal emulation for connecting
//! to remote servers.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tui_term::vt100;

/// An active telnet session
pub struct TelnetSession {
    /// TCP stream (wrapped for Send/Sync safety)
    stream: Arc<Mutex<TcpStream>>,
    /// VT100 terminal parser
    parser: Arc<Mutex<vt100::Parser>>,
    /// Remote host
    pub host: String,
    /// Remote port
    pub port: u16,
    /// Connection state
    connected: Arc<Mutex<bool>>,
}

impl TelnetSession {
    /// Connect to a telnet server
    pub fn connect(host: &str, port: u16, cols: u16, rows: u16) -> anyhow::Result<Self> {
        let addr = format!("{}:{}", host, port);

        // Connect with timeout
        let stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?,
            Duration::from_secs(10),
        )?;

        // Set non-blocking for tick-based polling
        stream.set_nonblocking(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;

        // Create VT100 parser with scrollback
        let parser = vt100::Parser::new(rows, cols, 1000);

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            parser: Arc::new(Mutex::new(parser)),
            host: host.to_string(),
            port,
            connected: Arc::new(Mutex::new(true)),
        })
    }

    /// Write data to the remote host
    pub fn write(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut stream = self.stream.lock().unwrap();
        stream.write_all(data)?;
        stream.flush()?;
        Ok(())
    }

    /// Try to read available data (non-blocking)
    pub fn try_read(&self) -> anyhow::Result<bool> {
        let mut stream = self.stream.lock().unwrap();
        let mut buffer = [0u8; 4096];

        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed
                *self.connected.lock().unwrap() = false;
                Ok(false)
            }
            Ok(n) => {
                // Process received data, handling telnet protocol
                let data = &buffer[..n];
                let processed = self.process_telnet_data(data);

                let mut parser = self.parser.lock().unwrap();
                parser.process(&processed);
                Ok(true)
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    Ok(false)
                } else {
                    *self.connected.lock().unwrap() = false;
                    Err(e.into())
                }
            }
        }
    }

    /// Process telnet protocol bytes, stripping IAC sequences
    fn process_telnet_data(&self, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len());
        let mut responses = Vec::new();
        let mut i = 0;

        while i < data.len() {
            if data[i] == 255 {
                // IAC (Interpret As Command)
                if i + 1 < data.len() {
                    match data[i + 1] {
                        251 | 252 => {
                            // WILL or WON'T - respond with DON'T
                            if i + 2 < data.len() {
                                responses.push(vec![255, 254, data[i + 2]]);
                                i += 3;
                                continue;
                            }
                        }
                        253 | 254 => {
                            // DO or DON'T - respond with WON'T
                            if i + 2 < data.len() {
                                responses.push(vec![255, 252, data[i + 2]]);
                                i += 3;
                                continue;
                            }
                        }
                        250 => {
                            // SB (subnegotiation) - skip until SE
                            let mut j = i + 2;
                            while j < data.len() {
                                if data[j] == 255 && j + 1 < data.len() && data[j + 1] == 240 {
                                    i = j + 2;
                                    break;
                                }
                                j += 1;
                            }
                            if j >= data.len() {
                                i = data.len();
                            }
                            continue;
                        }
                        255 => {
                            // Escaped 255
                            result.push(255);
                            i += 2;
                            continue;
                        }
                        _ => {
                            // Other command, skip
                            i += 2;
                            continue;
                        }
                    }
                }
                i += 1;
            } else {
                result.push(data[i]);
                i += 1;
            }
        }

        // Send responses if any
        if !responses.is_empty() {
            if let Ok(mut stream) = self.stream.lock() {
                for response in responses {
                    let _ = stream.write_all(&response);
                }
                let _ = stream.flush();
            }
        }

        result
    }

    /// Get reference to the VT100 parser for rendering
    pub fn screen(&self) -> Arc<Mutex<vt100::Parser>> {
        Arc::clone(&self.parser)
    }

    /// Resize the terminal
    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let mut parser = self.parser.lock().unwrap();
        parser.set_size(rows, cols);
        Ok(())
    }

    /// Check if still connected
    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    /// Disconnect from the server
    pub fn disconnect(&self) {
        *self.connected.lock().unwrap() = false;
    }

    /// Get connection info for display
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// Re-export key_to_bytes from pty module
pub use super::pty::key_to_bytes;
