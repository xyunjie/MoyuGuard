use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};

use crate::proto::proto::{AuthorizationRequest, AuthorizationResponse, Decision};

pub struct PendingRequest {
    pub request: AuthorizationRequest,
    pub response_tx: mpsc::Sender<AuthorizationResponse>,
    /// Sending any value cancels the timeout task for this request.
    cancel_tx: watch::Sender<bool>,
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
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let request_id = request.request_id.clone();
        let timeout_seconds = request.timeout_seconds;

        self.pending.write().await.insert(
            request_id.clone(),
            PendingRequest {
                request,
                response_tx: tx.clone(),
                cancel_tx,
            },
        );

        let pending = self.pending.clone();
        tokio::spawn(async move {
            tokio::select! {
                // Cancelled by resolve() — request was handled normally.
                _ = cancel_rx.changed() => {}
                // Timed out waiting for a decision.
                _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_seconds as u64)) => {
                    if let Some(req) = pending.write().await.remove(&request_id) {
                        let _ = req.response_tx.send(AuthorizationResponse {
                            request_id: request_id.clone(),
                            decision: Decision::Timeout.into(),
                            reason: "Authorization timed out".to_string(),
                        }).await;
                    }
                }
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
            // Cancel the timeout task immediately.
            let _ = pending.cancel_tx.send(true);
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
