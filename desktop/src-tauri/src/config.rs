use std::path::PathBuf;
use log::{info, warn};
use serde::{Deserialize, Serialize};

const DEFAULT_WS_PORT: u16 = 9876;

/// Claude Code internal tools shown in the "自动批准工具" settings panel.
/// Mirrors CodeIsland's allAutoApproveTools list.
pub const ALL_AUTO_APPROVE_TOOLS: &[(&str, &str)] = &[
    ("TaskCreate",    "创建新任务"),
    ("TaskUpdate",    "更新已有任务"),
    ("TaskGet",       "获取任务详情"),
    ("TaskList",      "列出所有任务"),
    ("TaskOutput",    "获取任务输出"),
    ("TaskStop",      "停止运行中的任务"),
    ("TodoRead",      "读取待办列表"),
    ("TodoWrite",     "写入待办列表"),
    ("EnterPlanMode", "进入计划模式"),
    ("ExitPlanMode",  "退出计划模式并请求审批"),
];

fn default_auto_approve_tools() -> Vec<String> {
    ALL_AUTO_APPROVE_TOOLS.iter().map(|(name, _)| name.to_string()).collect()
}

fn default_ws_port() -> u16 {
    DEFAULT_WS_PORT
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrustedClient {
    pub device_id:   String,
    pub device_name: String,
    pub platform:    String,
    pub paired_at:   i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
    #[serde(default)]
    pub trusted_clients: Vec<TrustedClient>,
    /// Tools that are automatically approved without a mobile prompt.
    /// Defaults to all Claude Code internal tools (Task*, Todo*, Plan*).
    #[serde(default = "default_auto_approve_tools")]
    pub auto_approve_tools: Vec<String>,
    /// Comma-separated cwd substrings; hook events whose cwd matches any entry
    /// are silently dropped (e.g. ".claude-mem,.cache/agents").
    #[serde(default)]
    pub excluded_cwd_patterns: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ws_port:               DEFAULT_WS_PORT,
            trusted_clients:       Vec::new(),
            auto_approve_tools:    default_auto_approve_tools(),
            excluded_cwd_patterns: String::new(),
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
