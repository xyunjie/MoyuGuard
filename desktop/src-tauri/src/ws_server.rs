use std::collections::HashMap;
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use log::{info, warn, error};
use prost::Message;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite;

use crate::proto::proto::Envelope;

pub type ClientId = String;

#[derive(Clone, Copy, PartialEq)]
pub enum ClientProtocol {
    Protobuf,
    Json,
}

struct Client {
    tx: mpsc::UnboundedSender<tungstenite::Message>,
    device_name: String,
    device_id: String,
    platform: String,
    protocol: ClientProtocol,
    trusted: bool,
}

pub struct WsServer {
    clients: Arc<RwLock<HashMap<ClientId, Client>>>,
    port: u16,
}

pub enum IncomingMessage {
    Proto(Envelope),
    Json(serde_json::Value),
}

impl WsServer {
    pub fn new(port: u16) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            port,
        }
    }

    pub async fn start(
        &self,
        on_message: mpsc::UnboundedSender<(ClientId, IncomingMessage)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on {}", addr);

        let clients = self.clients.clone();

        tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let clients = clients.clone();
                let on_message = on_message.clone();
                let client_id = uuid::Uuid::new_v4().to_string();

                info!("New connection from: {} (id: {})", addr, client_id);
                tokio::spawn(Self::handle_connection(
                    stream, client_id, clients, on_message,
                ));
            }
        });

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        client_id: ClientId,
        clients: Arc<RwLock<HashMap<ClientId, Client>>>,
        on_message: mpsc::UnboundedSender<(ClientId, IncomingMessage)>,
    ) {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                error!("WebSocket handshake failed: {}", e);
                return;
            }
        };

        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<tungstenite::Message>();

        clients.write().await.insert(
            client_id.clone(),
            Client {
                tx,
                device_name: "Unknown".to_string(),
                device_id: String::new(),
                platform: String::new(),
                protocol: ClientProtocol::Protobuf,
                trusted: false,
            },
        );

        let cid = client_id.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(tungstenite::Message::Binary(data)) => {
                    match Envelope::decode(data.as_ref()) {
                        Ok(envelope) => {
                            let _ = on_message.send((client_id.clone(), IncomingMessage::Proto(envelope)));
                        }
                        Err(e) => warn!("Failed to decode protobuf: {}", e),
                    }
                }
                Ok(tungstenite::Message::Text(text)) => {
                    // JSON 客户端 (Chrome 模拟器)
                    {
                        let mut cls = clients.write().await;
                        if let Some(c) = cls.get_mut(&client_id) {
                            c.protocol = ClientProtocol::Json;
                        }
                    }
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json) => {
                            let _ = on_message.send((client_id.clone(), IncomingMessage::Json(json)));
                        }
                        Err(e) => warn!("Failed to parse JSON: {}", e),
                    }
                }
                Ok(tungstenite::Message::Close(_)) => break,
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        clients.write().await.remove(&client_id);
        send_task.abort();
        info!("Client disconnected: {}", cid);
    }

    pub async fn set_client_info(&self, client_id: &str, device_id: String, device_name: String, platform: String) {
        if let Some(c) = self.clients.write().await.get_mut(client_id) {
            c.device_id = device_id;
            c.device_name = device_name;
            c.platform = platform;
        }
    }

    pub async fn set_trusted(&self, client_id: &str, trusted: bool) {
        if let Some(c) = self.clients.write().await.get_mut(client_id) {
            c.trusted = trusted;
        }
    }

    pub async fn is_trusted(&self, client_id: &str) -> bool {
        self.clients.read().await.get(client_id).map_or(false, |c| c.trusted)
    }

    pub async fn get_client_info(&self, client_id: &str) -> Option<(String, String, String)> {
        self.clients.read().await.get(client_id).map(|c| {
            (c.device_id.clone(), c.device_name.clone(), c.platform.clone())
        })
    }

    pub async fn disconnect_client(&self, client_id: &str) {
        if let Some(c) = self.clients.read().await.get(client_id) {
            let _ = c.tx.send(tungstenite::Message::Close(None));
        }
    }

    pub async fn send_to_json(&self, client_id: &str, json: &serde_json::Value) -> bool {
        if let Some(client) = self.clients.read().await.get(client_id) {
            let text = serde_json::to_string(json).unwrap_or_default();
            client.tx.send(tungstenite::Message::Text(text.into())).is_ok()
        } else {
            false
        }
    }

    pub async fn send_to(&self, client_id: &str, envelope: &Envelope) -> bool {
        if let Some(client) = self.clients.read().await.get(client_id) {
            match client.protocol {
                ClientProtocol::Json => {
                    let json = envelope_to_json(envelope);
                    let text = serde_json::to_string(&json).unwrap_or_default();
                    client.tx.send(tungstenite::Message::Text(text.into())).is_ok()
                }
                ClientProtocol::Protobuf => {
                    let data = envelope.encode_to_vec();
                    client.tx.send(tungstenite::Message::Binary(data.into())).is_ok()
                }
            }
        } else {
            false
        }
    }

    pub async fn broadcast(&self, envelope: &Envelope) {
        let proto_data = envelope.encode_to_vec();
        let json = envelope_to_json(envelope);
        let json_text = serde_json::to_string(&json).unwrap_or_default();

        for client in self.clients.read().await.values() {
            match client.protocol {
                ClientProtocol::Json => {
                    let _ = client.tx.send(tungstenite::Message::Text(json_text.clone().into()));
                }
                ClientProtocol::Protobuf => {
                    let _ = client.tx.send(tungstenite::Message::Binary(proto_data.clone().into()));
                }
            }
        }
    }

    pub async fn broadcast_json(&self, json: &serde_json::Value) {
        let text = serde_json::to_string(json).unwrap_or_default();
        for client in self.clients.read().await.values() {
            let _ = client.tx.send(tungstenite::Message::Text(text.clone().into()));
        }
    }

    pub async fn connected_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

fn envelope_to_json(envelope: &Envelope) -> serde_json::Value {
    use crate::proto::proto::envelope::Payload;
    use crate::proto::proto::*;

    match &envelope.payload {
        Some(Payload::AuthRequest(req)) => {
            let files: Vec<serde_json::Value> = req.files.iter().map(|f| {
                serde_json::json!({
                    "path": f.path,
                    "changeType": match ChangeType::try_from(f.change_type) {
                        Ok(ChangeType::Created) => "created",
                        Ok(ChangeType::Modified) => "modified",
                        Ok(ChangeType::Deleted) => "deleted",
                        Ok(ChangeType::Renamed) => "renamed",
                        _ => "unknown",
                    },
                    "diff": f.diff,
                    "additions": f.additions,
                    "deletions": f.deletions,
                })
            }).collect();

            serde_json::json!({
                "type": "auth_request",
                "message_id": envelope.message_id,
                "request_id": req.request_id,
                "tool_name": req.tool_name,
                "operation": match OperationType::try_from(req.operation) {
                    Ok(OperationType::FileWrite) => "file_write",
                    Ok(OperationType::FileDelete) => "file_delete",
                    Ok(OperationType::ShellExecute) => "shell_execute",
                    Ok(OperationType::GitPush) => "git_push",
                    Ok(OperationType::PackageInstall) => "package_install",
                    Ok(OperationType::ConfigModify) => "config_modify",
                    _ => "unknown",
                },
                "risk_level": match RiskLevel::try_from(req.risk_level) {
                    Ok(RiskLevel::Low) => "low",
                    Ok(RiskLevel::Medium) => "medium",
                    Ok(RiskLevel::High) => "high",
                    Ok(RiskLevel::Critical) => "critical",
                    _ => "unknown",
                },
                "summary": req.summary,
                "files": files,
                "raw_command": req.raw_command,
                "timeout_seconds": req.timeout_seconds,
            })
        }
        Some(Payload::PairResponse(resp)) => {
            serde_json::json!({
                "type": "pair_response",
                "accepted": resp.accepted,
                "computer_name": resp.computer_name,
                "computer_id": resp.computer_id,
            })
        }
        Some(Payload::Heartbeat(_)) => {
            serde_json::json!({ "type": "heartbeat" })
        }
        _ => serde_json::json!({ "type": "unknown" }),
    }
}
