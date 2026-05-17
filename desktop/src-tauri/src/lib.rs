mod auth;
mod config;
mod hook_installer;
mod hook_server;
mod interceptor;
mod log_store;
mod mdns;
mod proto;
mod ws_server;

use std::sync::Arc;
use log::info;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WindowEvent,
};
use tokio::sync::{mpsc, RwLock};

use auth::AuthManager;
use config::{AppConfig, TrustedClient};
use hook_server::HookServer;
use log_store::{LogEntry, LogStore};
use mdns::MdnsServer;
use proto::proto::*;
use ws_server::{IncomingMessage, WsServer};

struct AppState {
    ws_server: Arc<WsServer>,
    auth_manager: Arc<AuthManager>,
    log_store: Arc<LogStore>,
    config: Arc<RwLock<AppConfig>>,
}

#[derive(Serialize, Clone)]
struct FileChangeEvent {
    path: String,
    change_type: String,
    diff: String,
    additions: i32,
    deletions: i32,
}

#[derive(Serialize, Clone)]
struct AuthRequestEvent {
    request_id: String,
    tool_name: String,
    operation: String,
    risk_level: String,
    summary: String,
    file_count: usize,
    files: Vec<FileChangeEvent>,
    raw_command: String,
    timeout_seconds: u32,
}

#[derive(Serialize, Clone)]
struct ConnectionEvent {
    connected_count: usize,
}

#[derive(Serialize, Clone)]
struct PairPendingEvent {
    client_id: String,
    device_name: String,
    device_id: String,
    platform: String,
}

fn operation_name(op: i32) -> &'static str {
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

fn risk_name(level: i32) -> &'static str {
    match RiskLevel::try_from(level) {
        Ok(RiskLevel::Low) => "low",
        Ok(RiskLevel::Medium) => "medium",
        Ok(RiskLevel::High) => "high",
        Ok(RiskLevel::Critical) => "critical",
        _ => "unknown",
    }
}

fn update_tray_tooltip(app: &AppHandle, pending: usize, connected: usize) {
    let text = if pending > 0 {
        format!("MoyuGuard · {} 个待审批 · {} 设备", pending, connected)
    } else if connected > 0 {
        format!("MoyuGuard · {} 台已连接", connected)
    } else {
        "MoyuGuard · 等待连接".to_string()
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&text));
    }
}

fn change_type_name(ct: i32) -> &'static str {
    match ChangeType::try_from(ct) {
        Ok(ChangeType::Created) => "created",
        Ok(ChangeType::Modified) => "modified",
        Ok(ChangeType::Deleted) => "deleted",
        Ok(ChangeType::Renamed) => "renamed",
        _ => "unknown",
    }
}

fn file_change_to_event(f: &FileChange) -> FileChangeEvent {
    FileChangeEvent {
        path: f.path.clone(),
        change_type: change_type_name(f.change_type).to_string(),
        diff: f.diff.clone(),
        additions: f.additions,
        deletions: f.deletions,
    }
}

#[tauri::command]
async fn get_connected_count(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let connected = state.ws_server.connected_count().await;
    let pending = state.auth_manager.list_pending().await.len();
    update_tray_tooltip(&app, pending, connected);
    Ok(connected)
}

#[tauri::command]
async fn get_pending_requests(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AuthRequestEvent>, String> {
    let requests = state.auth_manager.list_pending().await;
    Ok(requests
        .into_iter()
        .map(|r| AuthRequestEvent {
            request_id: r.request_id,
            tool_name: r.tool_name,
            operation: operation_name(r.operation).to_string(),
            risk_level: risk_name(r.risk_level).to_string(),
            summary: r.summary,
            file_count: r.files.len(),
            files: r.files.iter().map(file_change_to_event).collect(),
            raw_command: r.raw_command,
            timeout_seconds: r.timeout_seconds,
        })
        .collect())
}

#[tauri::command]
async fn get_log_entries(state: tauri::State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    Ok(state.log_store.list().await)
}

#[tauri::command]
async fn clear_log(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.log_store.clear().await;
    Ok(())
}

#[tauri::command]
async fn approve_request(
    request_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let reason = "approved on desktop".to_string();
    if let Some(req) = state.auth_manager
        .resolve(&request_id, Decision::Approved, reason.clone())
        .await
    {
        record_log(&state.log_store, &app, &req, Decision::Approved, &reason).await;
        let _ = app.emit("auth-resolved", &request_id);
        state.ws_server.broadcast_json(&serde_json::json!({
            "type": "auth_resolved",
            "request_id": request_id,
        })).await;
        Ok(())
    } else {
        Err(format!("Request {} not found or already resolved", request_id))
    }
}

#[tauri::command]
async fn reject_request(
    request_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let reason = "rejected on desktop".to_string();
    if let Some(req) = state.auth_manager
        .resolve(&request_id, Decision::Rejected, reason.clone())
        .await
    {
        record_log(&state.log_store, &app, &req, Decision::Rejected, &reason).await;
        let _ = app.emit("auth-resolved", &request_id);
        state.ws_server.broadcast_json(&serde_json::json!({
            "type": "auth_resolved",
            "request_id": request_id,
        })).await;
        Ok(())
    } else {
        Err(format!("Request {} not found or already resolved", request_id))
    }
}

/// Called by the desktop UI when the user approves a pairing request.
#[tauri::command]
async fn approve_pair(
    client_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if let Some((device_id, device_name, platform)) = state.ws_server.get_client_info(&client_id).await {
        state.ws_server.set_trusted(&client_id, true).await;

        // Persist to config
        let mut cfg = state.config.write().await;
        if !cfg.trusted_clients.iter().any(|c| c.device_id == device_id) {
            cfg.trusted_clients.push(TrustedClient {
                device_id: device_id.clone(),
                device_name: device_name.clone(),
                platform: platform.clone(),
                paired_at: chrono::Utc::now().timestamp_millis(),
            });
            cfg.save().map_err(|e| e.to_string())?;
        }

        send_pair_accept(&client_id, &state.ws_server, &app).await;
        info!("Pairing approved for {} ({})", device_name, device_id);
        Ok(())
    } else {
        Err(format!("Client {} not found", client_id))
    }
}

/// Called by the desktop UI when the user rejects a pairing request.
#[tauri::command]
async fn reject_pair(
    client_id: String,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    send_pair_reject(&client_id, &state.ws_server).await;
    state.ws_server.disconnect_client(&client_id).await;
    let count = state.ws_server.connected_count().await;
    let _ = app.emit("connection-changed", ConnectionEvent { connected_count: count });
    info!("Pairing rejected for client {}", client_id);
    Ok(())
}

#[tauri::command]
async fn get_trusted_clients(state: tauri::State<'_, AppState>) -> Result<Vec<TrustedClient>, String> {
    Ok(state.config.read().await.trusted_clients.clone())
}

#[tauri::command]
async fn remove_trusted_client(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut cfg = state.config.write().await;
    cfg.trusted_clients.retain(|c| c.device_id != device_id);
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_mock_request(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let request = AuthorizationRequest {
        request_id: request_id.clone(),
        tool_name: "claude_code".to_string(),
        operation: OperationType::FileWrite.into(),
        risk_level: RiskLevel::Medium.into(),
        summary: "修改 src/main.rs 中的数据库连接配置".to_string(),
        files: vec![FileChange {
            path: "src/main.rs".to_string(),
            change_type: ChangeType::Modified.into(),
            diff: "@@ -10,3 +10,5 @@\n-let db_url = \"localhost\";\n+let db_url = \"production.db.example.com\";".to_string(),
            additions: 1,
            deletions: 1,
        }],
        raw_command: "edit src/main.rs".to_string(),
        timeout_seconds: 60,
    };

    let envelope = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::AuthRequest.into(),
        payload: Some(envelope::Payload::AuthRequest(request.clone())),
    };

    state.ws_server.broadcast(&envelope).await;

    let event = AuthRequestEvent {
        request_id: request.request_id.clone(),
        tool_name: request.tool_name.clone(),
        operation: operation_name(request.operation).to_string(),
        risk_level: risk_name(request.risk_level).to_string(),
        summary: request.summary.clone(),
        file_count: request.files.len(),
        files: request.files.iter().map(file_change_to_event).collect(),
        raw_command: request.raw_command.clone(),
        timeout_seconds: request.timeout_seconds,
    };
    let _ = app.emit("auth-request", &event);

    let mut rx = state.auth_manager.add_request(request).await;
    if let Some(response) = rx.recv().await {
        let decision = Decision::try_from(response.decision)
            .unwrap_or(Decision::Unspecified);
        Ok(format!("{:?}", decision))
    } else {
        Ok("no_response".to_string())
    }
}

#[tauri::command]
fn install_hooks(tool: String) -> Result<String, String> {
    match tool.as_str() {
        "claude_code" => hook_installer::install_claude_code(),
        "codex" => hook_installer::install_codex(),
        _ => Err(format!("Unknown tool: {}", tool)),
    }
}

#[tauri::command]
fn uninstall_hooks() -> Result<String, String> {
    hook_installer::uninstall_all()
}

#[tauri::command]
fn get_hook_status() -> serde_json::Value {
    hook_installer::get_hook_status()
}

#[tauri::command]
async fn get_app_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
async fn save_app_config(config: AppConfig, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.write().await;
    cfg.ws_port = config.ws_port;
    cfg.auto_approve_tools = config.auto_approve_tools;
    cfg.excluded_cwd_patterns = config.excluded_cwd_patterns;
    cfg.save()
}

#[tauri::command]
fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().ok();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出 MoyuGuard", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("MoyuGuard · 等待连接")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let app_config = AppConfig::load();
            let ws_port = app_config.ws_port;

            let ws_server = Arc::new(WsServer::new(ws_port));
            let auth_manager = Arc::new(AuthManager::new());
            let log_store = LogStore::new();
            let config = Arc::new(RwLock::new(app_config));

            let state = AppState {
                ws_server: ws_server.clone(),
                auth_manager: auth_manager.clone(),
                log_store: log_store.clone(),
                config: config.clone(),
            };
            app.manage(state);

            let app_handle = app.handle().clone();
            let ws = ws_server.clone();
            let am = auth_manager.clone();
            let ls = log_store.clone();

            tauri::async_runtime::spawn(async move {
                let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

                if let Err(e) = ws.start(msg_tx).await {
                    log::error!("Failed to start WebSocket server: {}", e);
                    return;
                }
                info!("WebSocket server started on port {}", ws_port);

                match MdnsServer::new(ws_port) {
                    Ok(_mdns) => {
                        info!("mDNS service registered");
                        std::mem::forget(_mdns);
                    }
                    Err(e) => log::error!("Failed to start mDNS: {}", e),
                }

                let hook_server = HookServer::new();
                info!("Hook socket path: {}", hook_server.socket_path());
                if let Err(e) = hook_server.start(am.clone(), ws.clone(), app_handle.clone(), config.clone()).await {
                    log::error!("Failed to start hook server: {}", e);
                }

                while let Some((client_id, incoming)) = msg_rx.recv().await {
                    match incoming {
                        IncomingMessage::Proto(envelope) => {
                            handle_proto_message(&client_id, envelope, &ws, &am, &ls, &config, &app_handle).await;
                        }
                        IncomingMessage::Json(json) => {
                            handle_json_message(&client_id, json, &ws, &am, &ls, &config, &app_handle).await;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_connected_count,
            get_pending_requests,
            get_log_entries,
            clear_log,
            approve_request,
            reject_request,
            approve_pair,
            reject_pair,
            get_trusted_clients,
            remove_trusted_client,
            send_mock_request,
            install_hooks,
            uninstall_hooks,
            get_hook_status,
            get_app_config,
            save_app_config,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn handle_pair_request(
    client_id: &str,
    device_id: &str,
    device_name: &str,
    platform: &str,
    ws: &Arc<WsServer>,
    config: &Arc<RwLock<AppConfig>>,
    app_handle: &AppHandle,
) {
    ws.set_client_info(client_id, device_id.to_string(), device_name.to_string(), platform.to_string()).await;

    let is_known = config.read().await.trusted_clients.iter().any(|c| c.device_id == device_id);

    if is_known {
        info!("Auto-accepting known device: {} ({})", device_name, device_id);
        ws.set_trusted(client_id, true).await;
        send_pair_accept(client_id, ws, app_handle).await;
    } else {
        info!("Pending pair from unknown device: {} ({})", device_name, device_id);
        let _ = app_handle.emit("pair-pending", PairPendingEvent {
            client_id: client_id.to_string(),
            device_name: device_name.to_string(),
            device_id: device_id.to_string(),
            platform: platform.to_string(),
        });
        // Show window so the user sees the dialog
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

async fn handle_proto_message(
    client_id: &str,
    envelope: Envelope,
    ws: &Arc<WsServer>,
    am: &Arc<AuthManager>,
    ls: &Arc<LogStore>,
    config: &Arc<RwLock<AppConfig>>,
    app_handle: &AppHandle,
) {
    match MessageType::try_from(envelope.r#type) {
        Ok(MessageType::AuthResponse) => {
            if !ws.is_trusted(client_id).await {
                log::warn!("Ignoring auth_response from untrusted client {}", client_id);
                return;
            }
            if let Some(envelope::Payload::AuthResponse(resp)) = envelope.payload {
                let decision = Decision::try_from(resp.decision)
                    .unwrap_or(Decision::Unspecified);
                if let Some(req) = am.resolve(&resp.request_id, decision, resp.reason.clone()).await {
                    record_log(ls, app_handle, &req, decision, &resp.reason).await;
                    let _ = app_handle.emit("auth-resolved", &resp.request_id);
                    ws.broadcast_json(&serde_json::json!({
                        "type": "auth_resolved",
                        "request_id": &resp.request_id,
                    })).await;
                }
            }
        }
        Ok(MessageType::Heartbeat) => {
            info!("Heartbeat from {}", client_id);
        }
        Ok(MessageType::PairRequest) => {
            if let Some(envelope::Payload::PairRequest(req)) = envelope.payload {
                let device_id = if req.device_id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    req.device_id.clone()
                };
                handle_pair_request(client_id, &device_id, &req.device_name, &req.platform, ws, config, app_handle).await;
            }
        }
        _ => {}
    }
}

async fn handle_json_message(
    client_id: &str,
    json: serde_json::Value,
    ws: &Arc<WsServer>,
    am: &Arc<AuthManager>,
    ls: &Arc<LogStore>,
    config: &Arc<RwLock<AppConfig>>,
    app_handle: &AppHandle,
) {
    let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "pair_request" => {
            let device_name = json.get("device_name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let platform = json.get("platform").and_then(|v| v.as_str()).unwrap_or("unknown");
            let device_id = json.get("device_id").and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            handle_pair_request(client_id, &device_id, device_name, platform, ws, config, app_handle).await;
        }
        "auth_response" => {
            if !ws.is_trusted(client_id).await {
                log::warn!("Ignoring auth_response from untrusted client {}", client_id);
                return;
            }
            let request_id = json.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
            let decision_str = json.get("decision").and_then(|v| v.as_str()).unwrap_or("");
            let reason = json.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let decision = match decision_str {
                "approved" => Decision::Approved,
                "rejected" => Decision::Rejected,
                _ => Decision::Unspecified,
            };

            info!("JSON auth response: {} -> {:?}", request_id, decision);
            if let Some(req) = am.resolve(request_id, decision, reason.clone()).await {
                record_log(ls, app_handle, &req, decision, &reason).await;
                let _ = app_handle.emit("auth-resolved", request_id);
                ws.broadcast_json(&serde_json::json!({
                    "type": "auth_resolved",
                    "request_id": request_id,
                })).await;
            }
        }
        "heartbeat" => {
            info!("JSON heartbeat from {}", client_id);
        }
        _ => {
            info!("Unknown JSON message type: {}", msg_type);
        }
    }
}

async fn record_log(
    ls: &Arc<LogStore>,
    app_handle: &AppHandle,
    req: &AuthorizationRequest,
    decision: Decision,
    reason: &str,
) {
    let decision_str = match decision {
        Decision::Approved => "approved",
        Decision::Rejected => "rejected",
        Decision::Timeout => "timeout",
        _ => "unknown",
    };
    let entry = LogEntry {
        id: req.request_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        tool_name: req.tool_name.clone(),
        summary: req.summary.clone(),
        risk_level: risk_name(req.risk_level).to_string(),
        operation: operation_name(req.operation).to_string(),
        decision: decision_str.to_string(),
        reason: reason.to_string(),
    };
    ls.append(entry.clone()).await;
    let _ = app_handle.emit("log-appended", &entry);
}

async fn send_pair_accept(client_id: &str, ws: &Arc<WsServer>, app_handle: &AppHandle) {
    let response = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::PairResponse.into(),
        payload: Some(envelope::Payload::PairResponse(PairResponse {
            accepted: true,
            computer_name: gethostname::gethostname().to_string_lossy().to_string(),
            computer_id: uuid::Uuid::new_v4().to_string(),
            session_token: uuid::Uuid::new_v4().to_string(),
        })),
    };
    ws.send_to(client_id, &response).await;
    let count = ws.connected_count().await;
    let _ = app_handle.emit("connection-changed", ConnectionEvent { connected_count: count });
}

async fn send_pair_reject(client_id: &str, ws: &Arc<WsServer>) {
    let response = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::PairResponse.into(),
        payload: Some(envelope::Payload::PairResponse(PairResponse {
            accepted: false,
            computer_name: String::new(),
            computer_id: String::new(),
            session_token: String::new(),
        })),
    };
    ws.send_to(client_id, &response).await;
}

