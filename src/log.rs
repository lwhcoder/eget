// src/log.rs
use chrono::{DateTime, Utc};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InstallEntry {
    pub timestamp: DateTime<Utc>,
    pub repo: String,
    pub path: String,
    pub removed: bool,
    pub size: Option<u64>,
}

impl InstallEntry {
    pub fn size_human(&self) -> String {
        match self.size {
            Some(size) => {
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else if size < 1024 * 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                }
            }
            None => "N/A".to_string(),
        }
    }
}

fn get_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join(".local/share/eget/install.log")
}

pub fn load_log() -> Vec<InstallEntry> {
    let log_path = get_log_path();
    let contents = fs::read_to_string(&log_path).unwrap_or_default();

    let mut entries: Vec<InstallEntry> = contents
        .lines()
        .filter_map(|line| {
            // Format: timestamp\trepo\tpath[\tremoved]
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                return None;
            }
            let ts_raw = parts[0];
            let repo = parts[1].to_string();
            let path = parts[2].to_string();
            let removed = parts.get(3).map(|s| *s == "removed").unwrap_or(false);

            let ts = DateTime::parse_from_rfc3339(ts_raw)
                .ok()?
                .with_timezone(&Utc);

            // Get file size if it exists
            let size = if !removed {
                fs::metadata(&path).ok().map(|m| m.len())
            } else {
                None
            };

            Some(InstallEntry {
                timestamp: ts,
                repo,
                path,
                removed,
                size,
            })
        })
        .collect();

    // Sort by most recent first
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries
}

pub fn mark_as_removed(path: &str) -> anyhow::Result<()> {
    let log_path = get_log_path();
    let contents = fs::read_to_string(&log_path)?;
    
    let updated: Vec<String> = contents
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 && parts[2] == path {
                // Mark as removed if not already
                if parts.len() == 3 {
                    format!("{}\tremoved", line)
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect();

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    
    file.write_all(updated.join("\n").as_bytes())?;
    Ok(())
}
