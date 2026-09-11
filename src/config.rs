use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub llm_provider: String,
    pub llm_base_url: String,
    pub llm_api_key: String,
    pub enable_sanitizer: bool,
    pub enable_truncator: bool,
    pub max_array_items: usize,
    pub scope_header: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let llm_provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| "groq".to_string());
        
        let llm_base_url = env::var("LLM_BASE_URL").unwrap_or_else(|_| {
            if llm_provider.eq_ignore_ascii_case("openai") {
                "https://api.openai.com/v1".to_string()
            } else {
                "https://api.groq.com/openai/v1".to_string()
            }
        });

        let llm_api_key = env::var("LLM_API_KEY")
            .or_else(|_| env::var("GROQ_API_KEY"))
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .unwrap_or_else(|_| "dummy_api_key".to_string());

        let enable_sanitizer = env::var("ENABLE_PROMPT_SANITIZER")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);

        let enable_truncator = env::var("ENABLE_SMART_TRUNCATE")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);

        let max_array_items = env::var("MAX_TOOL_ARRAY_ITEMS")
            .ok()
            .and_then(|m| m.parse().ok())
            .unwrap_or(5);

        let scope_header = env::var("SCOPE_HEADER_NAME")
            .unwrap_or_else(|_| "X-Guard-Scope".to_string());

        Self {
            port,
            llm_provider,
            llm_base_url,
            llm_api_key,
            enable_sanitizer,
            enable_truncator,
            max_array_items,
            scope_header,
        }
    }
}
