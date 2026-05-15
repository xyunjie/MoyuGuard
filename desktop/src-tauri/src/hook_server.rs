use std::sync::Arc;
use log::{info, warn, error};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

use crate::auth::AuthManager;
use crate::proto::proto::*;
use crate::ws_server::WsServer;

pub struct HookEvent {
    pub event_name: String,
    pub session_id: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub raw_json: serde_json::Value,
}

pub struct HookServer {
    socket_path: String,
}

impl HookServer {
    pub fn new() -> Self {
        let uid = unsafe { libc::getuid() };
        Self {
            socket_path: format!("/tmp/moyuguard-{}.sock", uid),
        }
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub async fn start(
        &self,
        auth_manager: Arc<AuthManager>,
        ws_server: Arc<WsServer>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o700))?;
        }

        info!("Hook server listening on {}", self.socket_path);

        let _socket_path = self.socket_path.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let am = auth_manager.clone();
                        let ws = ws_server.clone();
                        let app = app_handle.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, am, ws, app).await {
                                warn!("Hook connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Hook accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

impl Drop for HookServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    auth_manager: Arc<AuthManager>,
    ws_server: Arc<WsServer>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = Vec::with_capacity(65536);
    let mut tmp = [0u8; 65536];

    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 1_048_576 {
            warn!("Hook payload too large, dropping");
            return Ok(());
        }
        if n < tmp.len() { break; }
    }

    if buf.is_empty() {
        return Ok(());
    }

    let json: serde_json::Value = serde_json::from_slice(&buf)?;
    let event = parse_hook_event(&json);

    info!("Hook event: {} tool={:?}", event.event_name, event.tool_name);

    match event.event_name.as_str() {
        "PreToolUse" => {
            handle_pre_tool_use(stream, event, auth_manager, ws_server, app_handle).await?;
        }
        _ => {
            notify_event(&event, &app_handle);
            stream.write_all(b"{}").await?;
            stream.shutdown().await?;
        }
    }

    Ok(())
}

async fn handle_pre_tool_use(
    mut stream: tokio::net::UnixStream,
    event: HookEvent,
    auth_manager: Arc<AuthManager>,
    ws_server: Arc<WsServer>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_name = event.tool_name.clone().unwrap_or_default();
    let tool_input = event.tool_input.clone().unwrap_or(serde_json::json!({}));

    let risk_level = assess_risk(&tool_name, &tool_input);

    if risk_level == RiskLevel::Low as i32 {
        info!("Low risk tool {}, auto-approving", tool_name);
        stream.write_all(b"{}").await?;
        stream.shutdown().await?;
        return Ok(());
    }

    let summary = build_summary(&tool_name, &tool_input);
    let files = extract_files(&tool_name, &tool_input);
    let raw_command = extract_command(&tool_name, &tool_input);
    let operation = classify_operation(&tool_name);

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = AuthorizationRequest {
        request_id: request_id.clone(),
        tool_name: format!("claude_code:{}", tool_name),
        operation,
        risk_level,
        summary,
        files,
        raw_command,
        timeout_seconds: 120,
    };

    let envelope = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::AuthRequest.into(),
        payload: Some(envelope::Payload::AuthRequest(request.clone())),
    };
    ws_server.broadcast(&envelope).await;

    use tauri::Emitter;
    let ui_event = serde_json::json!({
        "request_id": request.request_id,
        "tool_name": request.tool_name,
        "operation": operation_name_str(request.operation),
        "risk_level": risk_name_str(request.risk_level),
        "summary": request.summary,
        "file_count": request.files.len(),
        "timeout_seconds": request.timeout_seconds,
    });
    let _ = app_handle.emit("auth-request", &ui_event);

    let mut rx = auth_manager.add_request(request).await;

    let response = rx.recv().await;

    let reply = match response {
        Some(resp) => {
            let decision = Decision::try_from(resp.decision).unwrap_or(Decision::Unspecified);
            match decision {
                Decision::Approved => {
                    info!("Hook approved: {}", request_id);
                    serde_json::json!({})
                }
                Decision::Rejected | Decision::Timeout => {
                    info!("Hook denied/timeout: {}", request_id);
                    serde_json::json!({"decision": {"behavior": "deny"}})
                }
                _ => serde_json::json!({})
            }
        }
        None => serde_json::json!({"decision": {"behavior": "deny"}}),
    };

    let reply_bytes = serde_json::to_vec(&reply)?;
    stream.write_all(&reply_bytes).await?;
    stream.shutdown().await?;

    Ok(())
}

fn parse_hook_event(json: &serde_json::Value) -> HookEvent {
    HookEvent {
        event_name: json.get("event_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        session_id: json.get("session_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        tool_name: json.get("tool_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        tool_input: json.get("tool_input").cloned(),
        raw_json: json.clone(),
    }
}

fn assess_risk(tool_name: &str, tool_input: &serde_json::Value) -> i32 {
    match tool_name {
        "Read" | "Glob" | "Grep" | "WebSearch" | "WebFetch" | "TodoRead" => {
            RiskLevel::Low.into()
        }
        "Bash" => {
            let cmd = tool_input.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let dangerous_patterns = [
                "rm -rf", "rm -r", "sudo ", "chmod 777", "push --force",
                "reset --hard", "DROP TABLE", "DELETE FROM", "> /dev/",
                "mkfs", "dd if=", ":(){ :|:& };:",
            ];
            if dangerous_patterns.iter().any(|p| cmd.contains(p)) {
                RiskLevel::Critical.into()
            } else {
                RiskLevel::High.into()
            }
        }
        "Edit" | "Write" | "NotebookEdit" => {
            let path = tool_input.get("file_path")
                .or_else(|| tool_input.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let sensitive = [".env", "credentials", "secret", "password", ".pem", ".key", "config.toml", "settings.json"];
            if sensitive.iter().any(|s| path.contains(s)) {
                RiskLevel::High.into()
            } else {
                RiskLevel::Medium.into()
            }
        }
        _ => RiskLevel::Medium.into(),
    }
}

fn build_summary(tool_name: &str, tool_input: &serde_json::Value) -> String {
    match tool_name {
        "Bash" => {
            let cmd = tool_input.get("command").and_then(|v| v.as_str()).unwrap_or("(unknown)");
            let display = if cmd.len() > 100 { &cmd[..100] } else { cmd };
            format!("执行命令: {}", display)
        }
        "Edit" => {
            let path = tool_input.get("file_path").and_then(|v| v.as_str()).unwrap_or("(unknown)");
            format!("编辑文件: {}", path)
        }
        "Write" => {
            let path = tool_input.get("file_path").and_then(|v| v.as_str()).unwrap_or("(unknown)");
            format!("写入文件: {}", path)
        }
        _ => format!("工具调用: {}", tool_name),
    }
}

fn extract_files(tool_name: &str, tool_input: &serde_json::Value) -> Vec<FileChange> {
    match tool_name {
        "Edit" => {
            let path = tool_input.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let old = tool_input.get("old_string").and_then(|v| v.as_str()).unwrap_or("");
            let new = tool_input.get("new_string").and_then(|v| v.as_str()).unwrap_or("");
            let diff = format!("-{}\n+{}", old, new);
            vec![FileChange {
                path,
                change_type: ChangeType::Modified.into(),
                diff,
                additions: new.lines().count() as i32,
                deletions: old.lines().count() as i32,
            }]
        }
        "Write" => {
            let path = tool_input.get("file_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let content = tool_input.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let lines = content.lines().count();
            vec![FileChange {
                path,
                change_type: ChangeType::Created.into(),
                diff: format!("+({} lines)", lines),
                additions: lines as i32,
                deletions: 0,
            }]
        }
        _ => vec![],
    }
}

fn extract_command(tool_name: &str, tool_input: &serde_json::Value) -> String {
    match tool_name {
        "Bash" => tool_input.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

fn classify_operation(tool_name: &str) -> i32 {
    match tool_name {
        "Bash" => OperationType::ShellExecute.into(),
        "Edit" | "NotebookEdit" => OperationType::FileWrite.into(),
        "Write" => OperationType::FileWrite.into(),
        _ => OperationType::OperationUnspecified.into(),
    }
}

fn operation_name_str(op: i32) -> &'static str {
    match OperationType::try_from(op) {
        Ok(OperationType::FileWrite) => "file_write",
        Ok(OperationType::FileDelete) => "file_delete",
        Ok(OperationType::ShellExecute) => "shell_execute",
        Ok(OperationType::GitPush) => "git_push",
        Ok(OperationType::PackageInstall) => "package_install",
        Ok(OperationType::ConfigModify) => "config_modify",
        _ => "unknown",
    }
}

fn risk_name_str(level: i32) -> &'static str {
    match RiskLevel::try_from(level) {
        Ok(RiskLevel::Low) => "low",
        Ok(RiskLevel::Medium) => "medium",
        Ok(RiskLevel::High) => "high",
        Ok(RiskLevel::Critical) => "critical",
        _ => "unknown",
    }
}

fn notify_event(event: &HookEvent, app_handle: &tauri::AppHandle) {
    use tauri::Emitter;
    let _ = app_handle.emit("hook-event", &serde_json::json!({
        "event_name": event.event_name,
        "tool_name": event.tool_name,
        "session_id": event.session_id,
    }));
}
