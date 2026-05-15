use std::path::PathBuf;
use std::sync::Arc;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const MAX_ENTRIES: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: i64,
    pub tool_name: String,
    pub summary: String,
    pub risk_level: String,
    pub operation: String,
    pub decision: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Default)]
struct LogFile {
    entries: Vec<LogEntry>,
}

pub struct LogStore {
    entries: RwLock<Vec<LogEntry>>,
    path: PathBuf,
}

impl LogStore {
    pub fn new() -> Arc<Self> {
        let dir = dirs::home_dir().unwrap_or_default().join(".moyuguard");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.json");

        let entries = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<LogFile>(&content) {
                    Ok(f) => {
                        info!("Loaded {} log entries from {:?}", f.entries.len(), path);
                        f.entries
                    }
                    Err(e) => {
                        warn!("Failed to parse {:?}: {} — starting fresh", path, e);
                        Vec::new()
                    }
                },
                Err(e) => {
                    warn!("Failed to read {:?}: {} — starting fresh", path, e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        Arc::new(Self {
            entries: RwLock::new(entries),
            path,
        })
    }

    pub async fn append(&self, entry: LogEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);
        // Trim oldest if we exceed the cap. Newest is at the end.
        let len = entries.len();
        if len > MAX_ENTRIES {
            entries.drain(0..len - MAX_ENTRIES);
        }
        let snapshot = entries.clone();
        drop(entries);
        self.persist(snapshot).await;
    }

    pub async fn list(&self) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        // Newest first for the UI
        entries.iter().rev().cloned().collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        let snapshot = entries.clone();
        drop(entries);
        self.persist(snapshot).await;
    }

    async fn persist(&self, entries: Vec<LogEntry>) {
        let path = self.path.clone();
        // Run blocking IO off the async runtime
        tokio::task::spawn_blocking(move || {
            let file = LogFile { entries };
            match serde_json::to_string_pretty(&file) {
                Ok(json) => {
                    let tmp = path.with_extension("json.tmp");
                    if let Err(e) = std::fs::write(&tmp, &json) {
                        warn!("Failed to write log tmp: {}", e);
                        return;
                    }
                    if let Err(e) = std::fs::rename(&tmp, &path) {
                        warn!("Failed to rename log tmp: {}", e);
                    }
                }
                Err(e) => warn!("Failed to serialize log: {}", e),
            }
        });
    }
}
