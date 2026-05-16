use std::path::PathBuf;
use log::{info, warn};
use serde::{Deserialize, Serialize};

const DEFAULT_WS_PORT: u16 = 9876;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
}

fn default_ws_port() -> u16 {
    DEFAULT_WS_PORT
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ws_port: DEFAULT_WS_PORT,
        }
    }
}

impl AppConfig {
    pub fn path() -> PathBuf {
        let dir = dirs::home_dir().unwrap_or_default().join(".moyuguard");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(cfg) => {
                    info!("Loaded config from {:?}: ws_port={}", path, cfg.ws_port);
                    cfg
                }
                Err(e) => {
                    warn!("Failed to parse {:?}: {} — using defaults", path, e);
                    Self::default()
                }
            },
            Err(e) => {
                warn!("Failed to read {:?}: {} — using defaults", path, e);
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize: {}", e))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &json).map_err(|e| format!("write tmp: {}", e))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("rename: {}", e))?;
        info!("Saved config to {:?}: ws_port={}", path, self.ws_port);
        Ok(())
    }
}
