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
        for pattern in &self.patterns {
            if pattern.is_match(text) {
                warn!("🛡️ [AI GUARD] Prompt Injection Attempt Detected: '{}'", text);
                return true;
            }
        }
        false
    }

    /// Sanitize text by stripping or neutralising injected phrases
    #[allow(dead_code)]
    pub fn sanitize(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        for pattern in &self.patterns {
            if pattern.is_match(&cleaned) {
                cleaned = pattern.replace_all(&cleaned, "[neutralized_prompt_injection]").to_string();
            }
        }
        cleaned
    }
}
