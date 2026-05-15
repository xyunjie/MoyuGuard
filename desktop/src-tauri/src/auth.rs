use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::proto::proto::{AuthorizationRequest, AuthorizationResponse, Decision};

pub struct PendingRequest {
    pub request: AuthorizationRequest,
    pub response_tx: mpsc::Sender<AuthorizationResponse>,
}

pub struct AuthManager {
    pending: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_request(
        &self,
        request: AuthorizationRequest,
    ) -> mpsc::Receiver<AuthorizationResponse> {
        let (tx, rx) = mpsc::channel(1);
        let request_id = request.request_id.clone();
        let timeout_seconds = request.timeout_seconds;

        self.pending.write().await.insert(
            request_id.clone(),
            PendingRequest {
                request,
                response_tx: tx.clone(),
            },
        );

        let pending = self.pending.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(timeout_seconds as u64)).await;
            if let Some(req) = pending.write().await.remove(&request_id) {
                let _ = req.response_tx.send(AuthorizationResponse {
                    request_id: request_id.clone(),
                    decision: Decision::Timeout.into(),
                    reason: "Authorization timed out".to_string(),
                }).await;
            }
        });

        rx
    }

    pub async fn resolve(
        &self,
        request_id: &str,
        decision: Decision,
        reason: String,
    ) -> Option<AuthorizationRequest> {
        if let Some(pending) = self.pending.write().await.remove(request_id) {
            let _ = pending.response_tx.send(AuthorizationResponse {
                request_id: request_id.to_string(),
                decision: decision.into(),
                reason,
            }).await;
            Some(pending.request)
        } else {
            None
        }
    }

    pub async fn list_pending(&self) -> Vec<AuthorizationRequest> {
        self.pending
            .read()
            .await
            .values()
            .map(|p| p.request.clone())
            .collect()
    }
}
