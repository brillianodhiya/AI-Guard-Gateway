use regex::Regex;
use tracing::warn;

pub struct SanitizerService {
    patterns: Vec<Regex>,
}

impl SanitizerService {
    pub fn new() -> Self {
        let raw_patterns = vec![
            r"(?i)ignore\s+(all\s+)?(previous|prior)\s+(instructions|directives|prompts)",
            r"(?i)disregard\s+(all\s+)?(previous|prior)\s+(instructions|rules)",
            r"(?i)system\s+prompt\s+override",
            r"(?i)you\s+are\s+now\s+in\s+DAN\s+mode",
            r"(?i)do\s+anything\s+now",
            r"(?i)forget\s+(your|all)\s+rules",
            r"(?i)bypass\s+safety\s+filters",
            r"(?i)jailbreak\s+mode",
        ];

        let patterns = raw_patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self { patterns }
    }

    /// Check if user input contains suspicious prompt injection patterns
    pub fn check_injection(&self, text: &str) -> bool {
        let text_to_scan = if text.len() > 10000 {
            &text[..10000]
        } else {
            text
        };

        for pattern in &self.patterns {
            if pattern.is_match(text_to_scan) {
                warn!("🛡️ [AI GUARD] Prompt Injection Attempt Detected: '{}'", text_to_scan);
                return true;
            }
        }
        false
    }

    /// Check if serde_json::Value contains prompt injection
    pub fn check_value_injection(&self, val: &serde_json::Value) -> bool {
        match val {
            serde_json::Value::String(s) => self.check_injection(s),
            serde_json::Value::Array(arr) => arr.iter().any(|v| self.check_value_injection(v)),
            serde_json::Value::Object(map) => map.values().any(|v| self.check_value_injection(v)),
            _ => false,
        }
    }

    /// Sanitize text by stripping or neutralising injected phrases
    pub fn sanitize(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        for pattern in &self.patterns {
            if pattern.is_match(&cleaned) {
                cleaned = pattern.replace_all(&cleaned, "[neutralized_prompt_injection]").to_string();
            }
        }
        cleaned
    }

    /// Sanitize serde_json::Value recursively
    #[allow(dead_code)]
    pub fn sanitize_value(&self, val: &serde_json::Value) -> serde_json::Value {
        match val {
            serde_json::Value::String(s) => serde_json::Value::String(self.sanitize(s)),
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.sanitize_value(v)).collect())
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.sanitize_value(v));
                }
                serde_json::Value::Object(new_map)
            }
            _ => val.clone(),
        }
    }
}

