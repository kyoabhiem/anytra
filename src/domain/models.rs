use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnhancementOptions {
    /// overall purpose or outcome you want the model to achieve
    #[serde(default)]
    pub goal: Option<String>,
    /// writing style, e.g., concise, formal, friendly
    #[serde(default)]
    pub style: Option<String>,
    /// tone, e.g., neutral, persuasive, enthusiastic
    #[serde(default)]
    pub tone: Option<String>,
    /// how strongly to enhance (1 = minimal edits, 5 = substantial refactor)
    #[serde(default)]
    pub level: Option<u8>,
    /// optional target audience for clarity
    #[serde(default)]
    pub audience: Option<String>,
    /// optional language code for output (e.g., en, id, es)
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedPrompt {
    pub text: String,
    pub rationale: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_prompt_creation() {
        let prompt = Prompt {
            text: "Write a hello world program".to_string(),
        };
        assert_eq!(prompt.text, "Write a hello world program");
    }

    #[test]
    fn test_prompt_serialization() {
        let prompt = Prompt {
            text: "Test prompt".to_string(),
        };
        let json = serde_json::to_string(&prompt).unwrap();
        assert_eq!(json, r#"{"text":"Test prompt"}"#);

        let deserialized: Prompt = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "Test prompt");
    }

    #[test]
    fn test_enhancement_options_default() {
        let options = EnhancementOptions::default();
        assert!(options.goal.is_none());
        assert!(options.style.is_none());
        assert!(options.tone.is_none());
        assert!(options.level.is_none());
        assert!(options.audience.is_none());
        assert!(options.language.is_none());
    }

    #[test]
    fn test_enhancement_options_with_values() {
        let options = EnhancementOptions {
            goal: Some("Create a clear instruction".to_string()),
            style: Some("concise".to_string()),
            tone: Some("professional".to_string()),
            level: Some(3),
            audience: Some("developers".to_string()),
            language: Some("en".to_string()),
        };

        assert_eq!(options.goal.as_deref(), Some("Create a clear instruction"));
        assert_eq!(options.style.as_deref(), Some("concise"));
        assert_eq!(options.tone.as_deref(), Some("professional"));
        assert_eq!(options.level, Some(3));
        assert_eq!(options.audience.as_deref(), Some("developers"));
        assert_eq!(options.language.as_deref(), Some("en"));
    }

    #[test]
    fn test_enhancement_options_serialization() {
        let options = EnhancementOptions {
            goal: Some("Test goal".to_string()),
            style: None,
            tone: None,
            level: Some(2),
            audience: None,
            language: None,
        };

        let json = serde_json::to_string(&options).unwrap();
        let expected = r#"{"goal":"Test goal","style":null,"tone":null,"level":2,"audience":null,"language":null}"#;
        assert_eq!(json, expected);

        let deserialized: EnhancementOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.goal.as_deref(), Some("Test goal"));
        assert_eq!(deserialized.level, Some(2));
        assert!(deserialized.style.is_none());
    }

    #[test]
    fn test_enhanced_prompt_creation() {
        let enhanced = EnhancedPrompt {
            text: "Enhanced prompt text".to_string(),
            rationale: Some("Made it clearer".to_string()),
            confidence: None,
        };
        assert_eq!(enhanced.text, "Enhanced prompt text");
        assert_eq!(enhanced.rationale.as_deref(), Some("Made it clearer"));
    }

    #[test]
    fn test_enhanced_prompt_without_rationale() {
        let enhanced = EnhancedPrompt {
            text: "Enhanced prompt text".to_string(),
            rationale: None,
            confidence: None,
        };
        assert_eq!(enhanced.text, "Enhanced prompt text");
        assert!(enhanced.rationale.is_none());
    }

    #[test]
    fn test_enhanced_prompt_serialization() {
        let enhanced = EnhancedPrompt {
            text: "Enhanced text".to_string(),
            rationale: Some("Test rationale".to_string()),
            confidence: None,
        };
        let json = serde_json::to_string(&enhanced).unwrap();
        assert_eq!(json, r#"{"text":"Enhanced text","rationale":"Test rationale","confidence":null}"#);

        let deserialized: EnhancedPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "Enhanced text");
        assert_eq!(deserialized.rationale.as_deref(), Some("Test rationale"));
    }

    #[test]
    fn test_enhanced_prompt_serialization_without_rationale() {
        let enhanced = EnhancedPrompt {
            text: "Enhanced text".to_string(),
            rationale: None,
            confidence: None,
        };
        let json = serde_json::to_string(&enhanced).unwrap();
        assert_eq!(json, r#"{"text":"Enhanced text","rationale":null,"confidence":null}"#);
    }
}
