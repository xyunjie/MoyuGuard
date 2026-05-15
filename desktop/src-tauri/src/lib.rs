mod auth;
mod hook_installer;
mod hook_server;
mod interceptor;
mod mdns;
mod proto;
mod ws_server;

use std::sync::Arc;
use log::info;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

use auth::AuthManager;
use hook_server::HookServer;
use mdns::MdnsServer;
use proto::proto::*;
use ws_server::{IncomingMessage, WsServer};

const WS_PORT: u16 = 9876;

struct AppState {
    ws_server: Arc<WsServer>,
    auth_manager: Arc<AuthManager>,
}

#[derive(Serialize, Clone)]
struct AuthRequestEvent {
    request_id: String,
    tool_name: String,
    operation: String,
    risk_level: String,
    summary: String,
    file_count: usize,
    timeout_seconds: u32,
}

#[derive(Serialize, Clone)]
struct ConnectionEvent {
    connected_count: usize,
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

#[tauri::command]
async fn get_connected_count(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    Ok(state.ws_server.connected_count().await)
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
            timeout_seconds: r.timeout_seconds,
        })
        .collect())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let ws_server = Arc::new(WsServer::new(WS_PORT));
            let auth_manager = Arc::new(AuthManager::new());

            let state = AppState {
                ws_server: ws_server.clone(),
                auth_manager: auth_manager.clone(),
            };
            app.manage(state);

            let app_handle = app.handle().clone();
            let ws = ws_server.clone();
            let am = auth_manager.clone();

            tauri::async_runtime::spawn(async move {
                let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();

                if let Err(e) = ws.start(msg_tx).await {
                    log::error!("Failed to start WebSocket server: {}", e);
                    return;
                }
                info!("WebSocket server started on port {}", WS_PORT);

                // Start mDNS
                match MdnsServer::new(WS_PORT) {
                    Ok(_mdns) => {
                        info!("mDNS service registered");
                        // Keep mdns alive by leaking it (it's a daemon)
                        std::mem::forget(_mdns);
                    }
                    Err(e) => log::error!("Failed to start mDNS: {}", e),
                }

                // Start Hook Server (Unix Socket)
                let hook_server = HookServer::new();
                info!("Hook socket path: {}", hook_server.socket_path());
                if let Err(e) = hook_server.start(am.clone(), ws.clone(), app_handle.clone()).await {
                    log::error!("Failed to start hook server: {}", e);
                }

                while let Some((client_id, incoming)) = msg_rx.recv().await {
                    match incoming {
                        IncomingMessage::Proto(envelope) => {
                            handle_proto_message(&client_id, envelope, &ws, &am, &app_handle).await;
                        }
                        IncomingMessage::Json(json) => {
                            handle_json_message(&client_id, json, &ws, &am, &app_handle).await;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_connected_count,
            get_pending_requests,
            send_mock_request,
            install_hooks,
            uninstall_hooks,
            get_hook_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn handle_proto_message(
    client_id: &str,
    envelope: Envelope,
    ws: &Arc<WsServer>,
    am: &Arc<AuthManager>,
    app_handle: &AppHandle,
) {
    match MessageType::try_from(envelope.r#type) {
        Ok(MessageType::AuthResponse) => {
            if let Some(envelope::Payload::AuthResponse(resp)) = envelope.payload {
                let decision = Decision::try_from(resp.decision)
                    .unwrap_or(Decision::Unspecified);
                am.resolve(&resp.request_id, decision, resp.reason).await;
                let _ = app_handle.emit("auth-resolved", &resp.request_id);
            }
        }
        Ok(MessageType::Heartbeat) => {
            info!("Heartbeat from {}", client_id);
        }
        Ok(MessageType::PairRequest) => {
            if let Some(envelope::Payload::PairRequest(req)) = envelope.payload {
                info!("Pair request from: {} ({})", req.device_name, req.platform);
                send_pair_response(client_id, ws, app_handle).await;
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
    app_handle: &AppHandle,
) {
    let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "pair_request" => {
            let device_name = json.get("device_name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let platform = json.get("platform").and_then(|v| v.as_str()).unwrap_or("unknown");
            info!("JSON pair request from: {} ({})", device_name, platform);
            send_pair_response(client_id, ws, app_handle).await;
        }
        "auth_response" => {
            let request_id = json.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
            let decision_str = json.get("decision").and_then(|v| v.as_str()).unwrap_or("");
            let reason = json.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let decision = match decision_str {
                "approved" => Decision::Approved,
                "rejected" => Decision::Rejected,
                _ => Decision::Unspecified,
            };

            info!("JSON auth response: {} -> {:?}", request_id, decision);
            am.resolve(request_id, decision, reason).await;
            let _ = app_handle.emit("auth-resolved", request_id);
        }
        "heartbeat" => {
            info!("JSON heartbeat from {}", client_id);
        }
        _ => {
            info!("Unknown JSON message type: {}", msg_type);
        }
    }
}

async fn send_pair_response(client_id: &str, ws: &Arc<WsServer>, app_handle: &AppHandle) {
    let response = Envelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        r#type: MessageType::PairResponse.into(),
        payload: Some(envelope::Payload::PairResponse(PairResponse {
            accepted: true,
            computer_name: gethostname::gethostname()
                .to_string_lossy()
                .to_string(),
            computer_id: uuid::Uuid::new_v4().to_string(),
            session_token: uuid::Uuid::new_v4().to_string(),
        })),
    };
    ws.send_to(client_id, &response).await;
    let count = ws.connected_count().await;
    let _ = app_handle.emit("connection-changed", ConnectionEvent {
        connected_count: count,
    });
}
