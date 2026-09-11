use crate::models::openai::ChatMessage;
use serde_json::Value;
use tracing::info;

pub struct TruncatorService;

impl TruncatorService {
    /// Truncate large tool result JSON payloads to save LLM tokens (Lossless Optimization)
    pub fn truncate_tool_results(messages: &mut [ChatMessage], max_items: usize) {
        for msg in messages.iter_mut() {
            if msg.role == "tool" || msg.role == "function" {
                if let Some(ref mut content) = msg.content {
                    if let Some(text) = content.as_str() {
                        if let Ok(mut json_val) = serde_json::from_str::<Value>(text) {
                            if Self::truncate_json(&mut json_val, max_items) {
                                info!("✂️ [AI GUARD] Smart Truncation applied to tool result payload");
                                *content = Value::String(json_val.to_string());
                            }
                        }
                    } else if content.is_array() {
                        if Self::truncate_json(content, max_items) {
                            info!("✂️ [AI GUARD] Smart Truncation applied to array tool content");
                        }
                    }
                }
            }
        }
    }

    fn truncate_json(val: &mut Value, max_items: usize) -> bool {
        match val {
            Value::Array(arr) => {
                if arr.len() > max_items {
                    let total_count = arr.len();
                    arr.truncate(max_items);
                    let meta_notice = serde_json::json!({
                        "_ai_guard_meta": format!("Array truncated by AI Guard Gateway. Showing {} of {} total items.", max_items, total_count)
                    });
                    arr.push(meta_notice);
                    return true;
                }
                false
            }
            Value::Object(obj) => {
                let mut modified = false;
                for (_k, v) in obj.iter_mut() {
                    if Self::truncate_json(v, max_items) {
                        modified = true;
                    }
                }
                modified
            }
            _ => false,
        }
    }
}
