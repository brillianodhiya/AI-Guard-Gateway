use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use crate::handlers::proxy::AppState;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SanitizeRequest {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
pub struct SanitizeResponse {
    pub safe: bool,
    pub detected_count: usize,
    pub messages: Vec<ChatMessage>,
}

pub async fn handle_sanitize(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SanitizeRequest>,
) -> Json<SanitizeResponse> {
    let mut safe = true;
    let mut detected_count = 0;
    let mut sanitized_messages = Vec::new();

    for msg in payload.messages {
        let is_injected = state.sanitizer.check_injection(&msg.content);
        if is_injected {
            safe = false;
            detected_count += 1;
            warn!("🛡️ [AI GUARD GATEWAY] Blocked prompt injection in message role '{}': '{}'", msg.role, msg.content);
            let cleaned_content = state.sanitizer.sanitize(&msg.content);
            sanitized_messages.push(ChatMessage {
                role: msg.role,
                content: cleaned_content,
            });
        } else {
            sanitized_messages.push(msg);
        }
    }

    if safe {
        info!("✅ [AI GUARD GATEWAY] Inspection clean: {} message(s) verified safe.", sanitized_messages.len());
    } else {
        warn!("🚨 [AI GUARD GATEWAY] Neutralized {} prompt injection attempt(s).", detected_count);
    }

    Json(SanitizeResponse {
        safe,
        detected_count,
        messages: sanitized_messages,
    })
}
