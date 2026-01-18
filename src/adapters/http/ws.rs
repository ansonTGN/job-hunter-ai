use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::handlers::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsEvent {
    #[serde(rename = "log")]
    Log { level: String, msg: String },

    #[serde(rename = "status")]
    Status(String),

    #[serde(rename = "job_found")]
    JobFound(job_hunter_core::AnalyzedJobPosting),
}

pub fn send_log(state: &AppState, level: &str, msg: impl Into<String>) {
    let _ = state.ws_tx.send(
        serde_json::to_string(&WsEvent::Log {
            level: level.to_string(),
            msg: msg.into(),
        })
        .unwrap_or_else(|_| {
            "{\"type\":\"log\",\"payload\":{\"level\":\"warn\",\"msg\":\"serialization_error\"}}"
                .to_string()
        }),
    );
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.ws_tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    })
}
