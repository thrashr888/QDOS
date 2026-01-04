//! Z directory jumper support
//!
//! Reads and queries the z (https://github.com/rupa/z) frecency database
//! to provide intelligent directory suggestions.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single entry from the z database
#[derive(Debug, Clone)]
pub struct ZEntry {
    /// The directory path
    pub path: PathBuf,
    /// Accumulated rank/frequency score
    pub rank: f64,
    /// Last access timestamp (Unix epoch seconds)
    pub timestamp: u64,
}

impl ZEntry {
    /// Calculate the frecency score for this entry
    /// Formula from z.sh: 10000 * rank * (3.75 / ((0.0001 * dx + 1) + 0.25))
    /// where dx is seconds since last access
    pub fn frecency(&self, now: u64) -> f64 {
        let dx = now.saturating_sub(self.timestamp) as f64;
        10000.0 * self.rank * (3.75 / ((0.0001 * dx + 1.0) + 0.25))
    }
}

/// The z database
#[derive(Debug, Default)]
pub struct ZDatabase {
    entries: Vec<ZEntry>,
}

impl ZDatabase {
    /// Load the z database from the default location (~/.z)
    pub fn load() -> Self {
        Self::load_from(Self::default_path())
    }

    /// Get the default z database path
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".z"))
            .unwrap_or_else(|| PathBuf::from("~/.z"))
    }

    /// Load the z database from a specific path
    pub fn load_from(path: PathBuf) -> Self {
        let mut db = ZDatabase::default();

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some(entry) = Self::parse_line(line) {
                    // Only include directories that still exist
                    if entry.path.is_dir() {
                        db.entries.push(entry);
                    }
                }
            }
        }

        db
    }

    /// Parse a single line from the z database
    /// Format: path|rank|timestamp
    fn parse_line(line: &str) -> Option<ZEntry> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let path = PathBuf::from(parts[0]);
            let rank = parts[1].parse::<f64>().ok()?;
            let timestamp = parts[2].parse::<u64>().ok()?;
            Some(ZEntry {
                path,
                rank,
                timestamp,
            })
        } else {
            None
        }
    }

    /// Get current Unix timestamp
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Check if the database has any entries
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Search for directories matching a query
    /// Query terms are space-separated and matched as substrings
    /// Returns results sorted by frecency (highest first)
    pub fn search(&self, query: &str) -> Vec<&ZEntry> {
        let now = Self::now();
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();

        if terms.is_empty() {
            // Return all entries sorted by frecency
            let mut results: Vec<&ZEntry> = self.entries.iter().collect();
            results.sort_by(|a, b| {
                b.frecency(now)
                    .partial_cmp(&a.frecency(now))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            return results;
        }

        // Filter entries that match all terms
        let mut results: Vec<&ZEntry> = self
            .entries
            .iter()
            .filter(|entry| {
                let path_lower = entry.path.to_string_lossy().to_lowercase();
                terms.iter().all(|term| path_lower.contains(term))
            })
            .collect();

        // Sort by frecency
        results.sort_by(|a, b| {
            b.frecency(now)
                .partial_cmp(&a.frecency(now))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get the best match for a query (highest frecency)
    pub fn best_match(&self, query: &str) -> Option<&ZEntry> {
        self.search(query).into_iter().next()
    }

    /// Get top N matches for a query
    #[allow(dead_code)]
    pub fn top_matches(&self, query: &str, n: usize) -> Vec<&ZEntry> {
        self.search(query).into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let entry = ZDatabase::parse_line("/home/user/projects|42|1700000000").unwrap();
        assert_eq!(entry.path, PathBuf::from("/home/user/projects"));
        assert_eq!(entry.rank, 42.0);
        assert_eq!(entry.timestamp, 1700000000);
    }

    #[test]
    fn test_frecency() {
        let entry = ZEntry {
            path: PathBuf::from("/test"),
            rank: 100.0,
            timestamp: 1700000000,
        };
        // Frecency should be positive
        assert!(entry.frecency(1700001000) > 0.0);
        // More recent should have higher frecency
        assert!(entry.frecency(1700000100) > entry.frecency(1700001000));
    }

    #[test]
    fn test_empty_query() {
        let db = ZDatabase::default();
        assert!(db.search("").is_empty());
    }
}
