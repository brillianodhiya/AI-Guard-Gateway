use crate::models::openai::ChatMessage;
use tracing::info;

pub struct ContextService;

impl ContextService {
    /// Inject scope context header into System Prompt or conversation
    pub fn inject_scope(messages: &mut Vec<ChatMessage>, scope_text: Option<&str>) {
        let scope = match scope_text {
            Some(s) if !s.trim().is_empty() => s.trim(),
            _ => return,
        };

        info!("🔐 [AI GUARD] Injecting Scope Context: '{}'", scope);

        let system_scope_directive = format!("\n\n[SECURITY DIRECTIVE - MANDATORY SCOPE BOUNDARY]: You are operating under the scope: \"{}\". Do not access or leak data outside this scope boundary.", scope);

        // Find existing system message or prepend a new one
        if let Some(sys_msg) = messages.iter_mut().find(|m| m.role == "system") {
            if let Some(ref mut content) = sys_msg.content {
                if let Some(s) = content.as_str() {
                    *content = serde_json::Value::String(format!("{}{}", s, system_scope_directive));
                }
            }
        } else {
            messages.insert(
                0,
                ChatMessage {
                    role: "system".to_string(),
                    content: Some(serde_json::Value::String(format!(
                        "System Initialized.{}",
                        system_scope_directive
                    ))),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            );
        }
    }
}
