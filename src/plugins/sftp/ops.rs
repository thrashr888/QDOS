//! SFTP Operations

use super::state::{AuthMethod, SftpConnection, SftpFileEntry, SftpState};
use ssh2::{Session, Sftp};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

/// Active SFTP session wrapper
pub struct SftpSession {
    #[allow(dead_code)]
    session: Session,
    sftp: Sftp,
}

impl SftpSession {
    /// Connect to SFTP server
    pub fn connect(config: &SftpConnection) -> Result<Self, String> {
        // Create TCP connection
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| format!("Connection failed: {}", e))?;

        // Create SSH session
        let mut session = Session::new().map_err(|e| format!("Session creation failed: {}", e))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| format!("SSH handshake failed: {}", e))?;

        // Authenticate based on method
        match &config.auth_method {
            AuthMethod::DefaultKey => {
                // Try default SSH key locations
                let home = dirs::home_dir().ok_or("Cannot find home directory")?;
                let key_paths = [
                    home.join(".ssh/id_ed25519"),
                    home.join(".ssh/id_rsa"),
                    home.join(".ssh/id_ecdsa"),
                ];

                let mut authenticated = false;
                for key_path in &key_paths {
                    if key_path.exists()
                        && session
                            .userauth_pubkey_file(&config.username, None, key_path, None)
                            .is_ok()
                    {
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    return Err("No valid SSH key found in ~/.ssh/".to_string());
                }
            }
            AuthMethod::KeyFile(path) => {
                let key_path = PathBuf::from(path);
                if !key_path.exists() {
                    return Err(format!("Key file not found: {}", path));
                }
                session
                    .userauth_pubkey_file(&config.username, None, &key_path, None)
                    .map_err(|e| format!("Key auth failed: {}", e))?;
            }
            AuthMethod::Password => {
                session
                    .userauth_password(&config.username, &config.password)
                    .map_err(|e| format!("Password auth failed: {}", e))?;
            }
            AuthMethod::Agent => {
                let mut agent = session
                    .agent()
                    .map_err(|e| format!("Cannot access SSH agent: {}", e))?;
                agent
                    .connect()
                    .map_err(|e| format!("Cannot connect to SSH agent: {}", e))?;
                agent
                    .list_identities()
                    .map_err(|e| format!("Cannot list agent identities: {}", e))?;

                let mut authenticated = false;
                let identities = agent.identities().map_err(|e| e.to_string())?;
                for identity in identities {
                    if agent.userauth(&config.username, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    return Err("No valid identity found in SSH agent".to_string());
                }
            }
        }

        if !session.authenticated() {
            return Err("Authentication failed".to_string());
        }

        // Create SFTP channel
        let sftp = session
            .sftp()
            .map_err(|e| format!("SFTP channel failed: {}", e))?;

        Ok(Self { session, sftp })
    }

    /// List directory contents
    pub fn list_dir(&self, path: &str) -> Result<Vec<SftpFileEntry>, String> {
        let dir_path = Path::new(path);
        let entries = self
            .sftp
            .readdir(dir_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut files: Vec<SftpFileEntry> = entries
            .into_iter()
            .filter_map(|(path, stat)| {
                let name = path.file_name()?.to_string_lossy().to_string();

                // Skip hidden files starting with .
                if name.starts_with('.') && name != ".." {
                    return None;
                }

                Some(SftpFileEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                    is_dir: stat.is_dir(),
                    size: stat.size.unwrap_or(0),
                    modified: stat.mtime.map(|t| t as i64),
                    permissions: stat.perm.unwrap_or(0o644),
                })
            })
            .collect();

        // Sort: directories first, then by name
        files.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        // Add parent directory entry if not at root
        if path != "/" {
            files.insert(
                0,
                SftpFileEntry {
                    name: "..".to_string(),
                    path: parent_path(path),
                    is_dir: true,
                    size: 0,
                    modified: None,
                    permissions: 0o755,
                },
            );
        }

        Ok(files)
    }

    /// Download file from remote to local
    pub fn download(&self, remote_path: &str, local_path: &Path) -> Result<u64, String> {
        let mut remote_file = self
            .sftp
            .open(Path::new(remote_path))
            .map_err(|e| format!("Failed to open remote file: {}", e))?;

        let mut local_file =
            File::create(local_path).map_err(|e| format!("Failed to create local file: {}", e))?;

        let mut buffer = [0u8; 8192];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = remote_file
                .read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;

            total_bytes += bytes_read as u64;
        }

        Ok(total_bytes)
    }

    /// Upload file from local to remote
    pub fn upload(&self, local_path: &Path, remote_path: &str) -> Result<u64, String> {
        let mut local_file =
            File::open(local_path).map_err(|e| format!("Failed to open local file: {}", e))?;

        let mut remote_file = self
            .sftp
            .create(Path::new(remote_path))
            .map_err(|e| format!("Failed to create remote file: {}", e))?;

        let mut buffer = [0u8; 8192];
        let mut total_bytes = 0u64;

        loop {
            let bytes_read = local_file
                .read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            remote_file
                .write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;

            total_bytes += bytes_read as u64;
        }

        Ok(total_bytes)
    }

    /// Get file size
    pub fn file_size(&self, path: &str) -> Result<u64, String> {
        let stat = self
            .sftp
            .stat(Path::new(path))
            .map_err(|e| format!("Failed to stat file: {}", e))?;

        Ok(stat.size.unwrap_or(0))
    }

    /// Get current working directory
    pub fn realpath(&self, path: &str) -> Result<String, String> {
        let real = self
            .sftp
            .realpath(Path::new(path))
            .map_err(|e| format!("Failed to resolve path: {}", e))?;

        Ok(real.to_string_lossy().to_string())
    }
}

/// Get parent path
fn parent_path(path: &str) -> String {
    let p = Path::new(path);
    p.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string())
}

/// Load directory contents into state
pub fn load_directory(state: &mut SftpState, session: &SftpSession, path: &str) {
    state.error = None;
    state.selected_file = 0;
    state.scroll_offset = 0;

    match session.list_dir(path) {
        Ok(files) => {
            state.files = files;
            state.current_dir = path.to_string();
        }
        Err(e) => {
            state.error = Some(e);
        }
    }
}

/// Format bytes for display
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get config file path for SFTP profiles
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rdos").join("sftp.toml"))
}

/// Load profiles from config file
pub fn load_profiles(state: &mut SftpState) {
    if let Some(path) = config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<super::state::SftpPluginConfig>(&content) {
                    state.profiles = config.profiles;
                    if let Some(last_conn) = config.last_connection {
                        state.connection = last_conn;
                    }
                }
            }
        }
    }
}

/// Save profiles to config file
pub fn save_profiles(state: &SftpState) {
    if let Some(path) = config_path() {
        // Create directory if needed
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Failed to create config dir: {}", e);
                return;
            }
        }

        let config = super::state::SftpPluginConfig {
            profiles: state.profiles.clone(),
            last_connection: Some(state.connection.clone()),
        };

        match toml::to_string_pretty(&config) {
            Ok(content) => {
                if let Err(e) = fs::write(&path, &content) {
                    eprintln!("Failed to write config: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize config: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/home/user/docs"), "/home/user");
        assert_eq!(parent_path("/home"), "/");
        assert_eq!(parent_path("/"), "/");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }
}
