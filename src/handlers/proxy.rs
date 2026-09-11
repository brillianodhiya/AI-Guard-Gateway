use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use crate::config::AppConfig;
use crate::models::openai::{ChatCompletionRequest, ChatMessage, ErrorDetails, ErrorResponse};
use crate::services::context::ContextService;
use crate::services::sanitizer::SanitizerService;
use crate::services::truncator::TruncatorService;
use reqwest::Client;
use std::sync::Arc;
use tracing::{error, info};

pub struct AppState {
    pub config: AppConfig,
    pub http_client: Client,
    pub sanitizer: SanitizerService,
}

pub async fn handle_chat_completion(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(mut req): Json<ChatCompletionRequest>,
) -> Response {
    info!("🚀 [AI GUARD GATEWAY] Received Request for model: '{}'", req.model);

    // Normalize /responses input/prompt fields into standard ChatMessage if messages is empty
    if req.messages.is_empty() {
        let mut extracted_text = String::new();
        if let Some(ref p) = req.prompt {
            if let Some(s) = p.as_str() {
                extracted_text = s.to_string();
            } else if p.is_array() {
                if let Ok(arr_str) = serde_json::to_string(p) {
                    extracted_text = arr_str;
                }
            }
        } else if let Some(ref i) = req.input {
            if let Some(s) = i.as_str() {
                extracted_text = s.to_string();
            } else if i.is_array() {
                if let Ok(arr_str) = serde_json::to_string(i) {
                    extracted_text = arr_str;
                }
            }
        }

        if !extracted_text.is_empty() {
            req.messages.push(ChatMessage {
                role: "user".to_string(),
                content: Some(serde_json::Value::String(extracted_text)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }
    req.prompt = None;
    req.input = None;

    // 1. Extract Scope Header if present
    let scope_header_name = state.config.scope_header.to_lowercase();
    let scope_val = headers
        .iter()
        .find(|(k, _)| k.as_str().to_lowercase() == scope_header_name)
        .and_then(|(_, v)| v.to_str().ok());

    // 2. Check Prompt Injection Security Policy
    if state.config.enable_sanitizer {
        for msg in &req.messages {
            if let Some(ref content) = msg.content {
                let text_to_check = match content {
                    serde_json::Value::String(s) => s.as_str(),
                    _ => "",
                };

                if state.sanitizer.check_injection(text_to_check) {
                    error!("🛑 [AI GUARD REJECT] Request blocked due to Prompt Injection Policy");
                    let err_resp = ErrorResponse {
                        error: ErrorDetails {
                            message: "Request blocked by AI Guard Security Policy: Prompt Injection or Jailbreak Attempt Detected.".to_string(),
                            r#type: "security_policy_violation".to_string(),
                            code: "prompt_injection_blocked".to_string(),
                        },
                    };
                    return (StatusCode::BAD_REQUEST, Json(err_resp)).into_response();
                }
            }
        }
    }

    // 3. Inject Scope Context if header exists
    ContextService::inject_scope(&mut req.messages, scope_val);

    // 4. Apply Smart Tool Payload Truncation (Token Optimization)
    if state.config.enable_truncator {
        TruncatorService::truncate_tool_results(&mut req.messages, state.config.max_array_items);
    }

    // Clear non-standard extra fields that might cause 422 errors on Google Gemini API
    req.extra_fields.clear();

    // 5. Dynamic Provider & Target URL Resolution
    let custom_provider = headers
        .get("x-llm-provider")
        .and_then(|v| v.to_str().ok());

    let (target_url, provider_name) = resolve_target_url(
        custom_provider,
        &req.model,
        &state.config.llm_provider,
        &state.config.llm_base_url,
    );

    // 6. Dynamic Authorization Key Resolution
    let auth_header_val = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Bearer {}", state.config.llm_api_key));

    info!("FORWARDING request to Provider [{}] Target URL: '{}'", provider_name, target_url);

    // 7. Forward Request to Upstream Cloud Provider via reqwest Client
    let mut upstream_req = state
        .http_client
        .post(&target_url)
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header_val)
        .json(&req);

    // Forward additional original headers if needed
    for (k, v) in headers.iter() {
        let key_str = k.as_str();
        if key_str != "host" && key_str != "authorization" && key_str != "content-length" && key_str != "accept-encoding" && key_str != "x-llm-provider" {
            upstream_req = upstream_req.header(key_str, v.as_bytes());
        }
    }

    match upstream_req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            match resp.bytes().await {
                Ok(bytes) => (status, bytes).into_response(),
                Err(e) => {
                    error!("Error reading upstream response body: {}", e);
                    (StatusCode::BAD_GATEWAY, "Error reading LLM upstream response").into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to Upstream LLM: {}", e);
            let err_resp = ErrorResponse {
                error: ErrorDetails {
                    message: format!("AI Guard Gateway Upstream Error: {}", e),
                    r#type: "upstream_gateway_error".to_string(),
                    code: "bad_gateway".to_string(),
                },
            };
            (StatusCode::BAD_GATEWAY, Json(err_resp)).into_response()
        }
    }
}

/// Dynamically resolve upstream target URL based on header, model name, or config default
fn resolve_target_url(
    custom_provider: Option<&str>,
    model_name: &str,
    default_provider: &str,
    default_base_url: &str,
) -> (String, String) {
    let provider = custom_provider
        .unwrap_or_else(|| {
            let m = model_name.to_lowercase();
            if m.starts_with("gemini") {
                "gemini"
            } else if m.starts_with("llama") || m.starts_with("qwen") || m.starts_with("mixtral") {
                "groq"
            } else if m.starts_with("gpt") {
                "openai"
            } else {
                default_provider
            }
        })
        .to_lowercase();

    let base_url = match provider.as_str() {
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai",
        "groq" => "https://api.groq.com/openai/v1",
        "openai" => "https://api.openai.com/v1",
        _ => default_base_url,
    };

    let target_url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    (target_url, provider)
}
