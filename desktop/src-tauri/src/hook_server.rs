use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{info, warn, error};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex as TokioMutex;

use crate::auth::AuthManager;
use crate::proto::proto::*;
use crate::ws_server::WsServer;

// How long a PreToolUse approval shields the matching PermissionRequest
// from triggering a second mobile prompt.
const DEDUP_TTL: Duration = Duration::from_secs(30);

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

        // Tracks operations approved via PreToolUse so the matching
        // PermissionRequest can be auto-approved without a second prompt.
        let pre_approved: Arc<TokioMutex<HashMap<String, Instant>>> =
            Arc::new(TokioMutex::new(HashMap::new()));

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let am = auth_manager.clone();
                        let ws = ws_server.clone();
                        let app = app_handle.clone();
                        let pa = pre_approved.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, am, ws, app, pa).await {
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
    pre_approved: Arc<TokioMutex<HashMap<String, Instant>>>,
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
    }

    if buf.is_empty() {
        return Ok(());
    }

    let json: serde_json::Value = serde_json::from_slice(&buf)?;
    let event = parse_hook_event(&json);

    info!("Hook event: {} tool={:?}", event.event_name, event.tool_name);

    match event.event_name.as_str() {
        "PreToolUse" => {
            handle_pre_tool_use(stream, event, auth_manager, ws_server, app_handle, pre_approved).await?;
        }
        "PermissionRequest" => {
            handle_permission_request(stream, event, auth_manager, ws_server, app_handle, pre_approved).await?;
        }
        _ => {
            notify_event(&event, &app_handle);
            stream.write_all(b"{}").await?;
            stream.shutdown().await?;
        }
    }

    Ok(())
}

// ── PreToolUse ────────────────────────────────────────────────────────────────
// Fires before EVERY tool execution. We use this as the primary decision point
// for Edit/Write/NotebookEdit and any other tool that Claude Code auto-approves
// internally (so PermissionRequest never fires for them).

async fn handle_pre_tool_use(
    mut stream: tokio::net::UnixStream,
    event: HookEvent,
    auth_manager: Arc<AuthManager>,
    ws_server: Arc<WsServer>,
    app_handle: tauri::AppHandle,
    pre_approved: Arc<TokioMutex<HashMap<String, Instant>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_name = event.tool_name.clone().unwrap_or_default();
    let tool_input = event.tool_input.clone().unwrap_or(serde_json::json!({}));
    let risk = assess_risk(&tool_name, &tool_input);

    if risk == RiskLevel::Low as i32 {
        info!("PreToolUse: low-risk tool {}, auto-approving", tool_name);
        stream.write_all(b"{}").await?;
        stream.shutdown().await?;
        return Ok(());
    }

    let decision = request_approval(&event, &tool_name, &tool_input, risk, &auth_manager, &ws_server, &app_handle).await;

    let reply: &[u8] = if decision {
        // Approved — store in dedup cache so PermissionRequest (if it fires for
        // the same operation, e.g. Bash) doesn't show a second mobile prompt.
        let key = dedup_key(&event.session_id, &tool_name, &tool_input);
        let mut cache = pre_approved.lock().await;
        evict_expired(&mut cache);
        cache.insert(key, Instant::now());
        b"{}"
    } else {
        br#"{"decision":"block","reason":"Operation rejected on MoyuGuard"}"#
    };

    stream.write_all(reply).await?;
    stream.shutdown().await?;
    Ok(())
}

// ── PermissionRequest ─────────────────────────────────────────────────────────
// Fires when Claude Code would normally show an interactive permission dialog.
// This is the only hook Claude Code re-reads live (without restarting the
// session), so it catches Bash commands even in pre-existing sessions.

async fn handle_permission_request(
    mut stream: tokio::net::UnixStream,
    event: HookEvent,
    auth_manager: Arc<AuthManager>,
    ws_server: Arc<WsServer>,
    app_handle: tauri::AppHandle,
    pre_approved: Arc<TokioMutex<HashMap<String, Instant>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_name = event.tool_name.clone().unwrap_or_default();
    let tool_input = event.tool_input.clone().unwrap_or(serde_json::json!({}));

    // If PreToolUse already got a decision, don't ask the user again.
    {
        let key = dedup_key(&event.session_id, &tool_name, &tool_input);
        let mut cache = pre_approved.lock().await;
        evict_expired(&mut cache);
        if cache.remove(&key).is_some() {
            info!("PermissionRequest auto-approved (already decided via PreToolUse): {}", tool_name);
            stream.write_all(PERM_ALLOW).await?;
            stream.shutdown().await?;
            return Ok(());
        }
    }

    let risk = assess_risk(&tool_name, &tool_input);

    if risk == RiskLevel::Low as i32 {
        stream.write_all(PERM_ALLOW).await?;
        stream.shutdown().await?;
        return Ok(());
    }

    let approved = request_approval(&event, &tool_name, &tool_input, risk, &auth_manager, &ws_server, &app_handle).await;

    let reply = if approved { PERM_ALLOW } else { PERM_DENY };
    stream.write_all(reply).await?;
    stream.shutdown().await?;
    Ok(())
}

const PERM_ALLOW: &[u8] = br#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#;
const PERM_DENY:  &[u8] = br#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny"}}}"#;

// ── Shared approval flow ──────────────────────────────────────────────────────

async fn request_approval(
    event: &HookEvent,
    tool_name: &str,
    tool_input: &serde_json::Value,
    risk_level: i32,
    auth_manager: &Arc<AuthManager>,
    ws_server: &Arc<WsServer>,
    app_handle: &tauri::AppHandle,
) -> bool {
    let summary    = build_summary(tool_name, tool_input);
    let files      = extract_files(tool_name, tool_input);
    let raw_command = extract_command(tool_name, tool_input);
    let operation  = classify_operation(tool_name);

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = AuthorizationRequest {
        request_id: request_id.clone(),
        tool_name: format!("claude_code:{}", tool_name),
        operation,
        risk_level,
        summary: summary.clone(),
        files: files.clone(),
        raw_command: raw_command.clone(),
        timeout_seconds: 3600,
    };

    let envelope = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::AuthRequest.into(),
        payload: Some(envelope::Payload::AuthRequest(request.clone())),
    };
    ws_server.broadcast(&envelope).await;

    use tauri::Emitter;
    let files_json: Vec<serde_json::Value> = request.files.iter().map(|f| serde_json::json!({
        "path": f.path,
        "change_type": match ChangeType::try_from(f.change_type) {
            Ok(ChangeType::Created)  => "created",
            Ok(ChangeType::Modified) => "modified",
            Ok(ChangeType::Deleted)  => "deleted",
            Ok(ChangeType::Renamed)  => "renamed",
            _                        => "unknown",
        },
        "diff": f.diff,
        "additions": f.additions,
        "deletions": f.deletions,
    })).collect();

    let _ = app_handle.emit("auth-request", serde_json::json!({
        "request_id": request.request_id,
        "tool_name": request.tool_name,
        "operation": operation_name_str(request.operation),
        "risk_level": risk_name_str(request.risk_level),
        "summary": request.summary,
        "file_count": request.files.len(),
        "files": files_json,
        "raw_command": request.raw_command,
        "timeout_seconds": request.timeout_seconds,
    }));

    use tauri_plugin_notification::NotificationExt;
    let risk_emoji = match risk_name_str(risk_level) {
        "critical" => "🚨",
        "high"     => "⚠️",
        "medium"   => "🔔",
        _          => "📥",
    };
    let _ = app_handle
        .notification()
        .builder()
        .title(format!("{} 待审批: {}", risk_emoji, tool_name))
        .body(&summary)
        .show();

    let mut rx = auth_manager.add_request(request).await;
    match rx.recv().await {
        Some(resp) => {
            matches!(Decision::try_from(resp.decision), Ok(Decision::Approved))
        }
        None => false, // channel closed / timeout → deny for safety
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn dedup_key(session_id: &str, tool_name: &str, tool_input: &serde_json::Value) -> String {
    format!("{}:{}:{}", session_id, tool_name, tool_input)
}

fn evict_expired(cache: &mut HashMap<String, Instant>) {
    let now = Instant::now();
    cache.retain(|_, ts| now.duration_since(*ts) < DEDUP_TTL);
}

fn parse_hook_event(json: &serde_json::Value) -> HookEvent {
    // Claude Code sends "hook_event_name"; Codex/others may send "event_name".
    let event_name = json.get("hook_event_name")
        .or_else(|| json.get("event_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    HookEvent {
        event_name,
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
            let dangerous = [
                "rm -rf", "rm -r", "sudo ", "chmod 777", "push --force",
                "reset --hard", "DROP TABLE", "DELETE FROM", "> /dev/",
                "mkfs", "dd if=", ":(){ :|:& };:",
            ];
            if dangerous.iter().any(|p| cmd.contains(p)) {
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
        "Bash"        => OperationType::ShellExecute.into(),
        "Edit" | "NotebookEdit" | "Write" => OperationType::FileWrite.into(),
        _ => OperationType::OperationUnspecified.into(),
    }
}

fn operation_name_str(op: i32) -> &'static str {
    match OperationType::try_from(op) {
        Ok(OperationType::FileWrite)      => "file_write",
        Ok(OperationType::FileDelete)     => "file_delete",
        Ok(OperationType::ShellExecute)   => "shell_execute",
        Ok(OperationType::GitPush)        => "git_push",
        Ok(OperationType::PackageInstall) => "package_install",
        Ok(OperationType::ConfigModify)   => "config_modify",
        _                                 => "unknown",
    }
}

fn risk_name_str(level: i32) -> &'static str {
    match RiskLevel::try_from(level) {
        Ok(RiskLevel::Low)      => "low",
        Ok(RiskLevel::Medium)   => "medium",
        Ok(RiskLevel::High)     => "high",
        Ok(RiskLevel::Critical) => "critical",
        _                       => "unknown",
    }
}

fn notify_event(event: &HookEvent, app_handle: &tauri::AppHandle) {
    use tauri::Emitter;
    let _ = app_handle.emit("hook-event", serde_json::json!({
        "event_name": event.event_name,
        "tool_name": event.tool_name,
        "session_id": event.session_id,
    }));
}
