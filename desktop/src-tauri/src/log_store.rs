use std::path::PathBuf;
use std::sync::Arc;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

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
    /// All disk writes are serialised through this channel so concurrent
    /// append() calls never race on the same tmp file.
    write_tx: mpsc::UnboundedSender<Vec<LogEntry>>,
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

        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Vec<LogEntry>>();

        // Single background task that serialises all disk writes.
        tokio::spawn({
            let path = path.clone();
            async move {
                while let Some(snapshot) = write_rx.recv().await {
                    persist_sync(&path, snapshot);
                }
            }
        });

        Arc::new(Self {
            entries: RwLock::new(entries),
            write_tx,
        })
    }

    pub async fn append(&self, entry: LogEntry) {
        let mut entries = self.entries.write().await;
        entries.push(entry);
        let len = entries.len();
        if len > MAX_ENTRIES {
            entries.drain(0..len - MAX_ENTRIES);
        }
        let _ = self.write_tx.send(entries.clone());
    }

    pub async fn list(&self) -> Vec<LogEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().cloned().collect()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        let _ = self.write_tx.send(entries.clone());
    }
}

fn persist_sync(path: &PathBuf, entries: Vec<LogEntry>) {
    let file = LogFile { entries };
    match serde_json::to_string_pretty(&file) {
        Ok(json) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = std::fs::write(&tmp, &json) {
                warn!("Failed to write log tmp: {}", e);
                return;
            }
            if let Err(e) = std::fs::rename(&tmp, path) {
                warn!("Failed to rename log tmp: {}", e);
            }
        }
        Err(e) => warn!("Failed to serialize log: {}", e),
    }
}
