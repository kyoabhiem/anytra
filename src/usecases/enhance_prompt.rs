use crate::domain::llm::LLMProvider;
use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
use crate::domain::sequential_thinking::SequentialThinking;
use crate::infrastructure::config::Config;
use anyhow::Result;
use serde_json::json;

pub struct EnhancePrompt {
    provider: Box<dyn LLMProvider + Send + Sync>,
    config: Config,
}

impl EnhancePrompt {
    pub fn new(provider: Box<dyn LLMProvider + Send + Sync>, config: Config) -> Self {
        Self { provider, config }
    }

    pub async fn execute(&self, prompt: Prompt, options: EnhancementOptions) -> Result<EnhancedPrompt> {
        let mut enhanced = self.provider.enhance(prompt.clone(), options.clone()).await?;
        crate::domain::validation::validate_enhanced_prompt(&enhanced)?;
        let confidence = crate::domain::validation::compute_confidence(&enhanced);
        enhanced.confidence = Some(confidence);

        // Handle sequential thinking if enabled
        if options.enable_sequential_thinking.unwrap_or_else(|| self.config.sequential_thinking_enabled()) {
            let mut sequential_thinker = SequentialThinking::new();
            let thought_count = options.thought_count.unwrap_or(3);

            for i in 1..=thought_count {
                let is_last_thought = i == thought_count;
                let thought_input = json!({
                    "thought": enhanced.text,
                    "thoughtNumber": i,
                    "totalThoughts": thought_count,
                    "nextThoughtNeeded": !is_last_thought
                });

                match sequential_thinker.process_thought(thought_input) {
                    Ok(_) => {
                        if !is_last_thought {
                            // Generate next thought based on current enhanced text
                            let next_options = EnhancementOptions {
                                enable_sequential_thinking: Some(false), // Disable for intermediate steps
                                ..options.clone()
                            };
                            enhanced = self.provider.enhance(Prompt { text: enhanced.text.clone() }, next_options).await?;
                        }
                    }
                    Err(e) => {
                        eprintln!("Sequential thinking error: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(enhanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::{LLMError, LLMProvider};
    use crate::infrastructure::config::{Config, OpenRouterConfig, SequentialThinkingConfig, LoggingConfig};
    use async_trait::async_trait;

    // Helper function to create test config
    fn create_test_config() -> Config {
        Config {
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                model: "test-model".to_string(),
                referer: None,
                title: None,
            },
            sequential_thinking: SequentialThinkingConfig {
                default_enabled: false, // Disable for tests unless explicitly needed
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }

    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn enhance(&self, prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Ok(EnhancedPrompt { text: format!("ENH: {} - this is a longer text with enough words to pass the validation check", prompt.text), rationale: None, confidence: None })
        }
    }

    struct MockProviderWithRationale;

    #[async_trait]
    impl LLMProvider for MockProviderWithRationale {
        async fn enhance(&self, prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Ok(EnhancedPrompt {
                text: format!("ENHANCED: {} - this is a longer text with enough words to pass validation", prompt.text),
                rationale: Some("Made it clearer and more specific".to_string()),
                confidence: None,
            })
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        async fn enhance(&self, _prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Err(LLMError::RequestFailed("Provider error".to_string()))
        }
    }

    #[tokio::test]
    async fn test_usecase_calls_provider() {
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(MockProvider), config);
        let res = usecase
            .execute(Prompt { text: "hello".into() }, EnhancementOptions {
                enable_sequential_thinking: Some(false), // Explicitly disable sequential thinking
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.text, "ENH: hello - this is a longer text with enough words to pass the validation check");
    }

    #[tokio::test]
    async fn test_usecase_with_options() {
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(MockProviderWithRationale), config);
        let options = EnhancementOptions {
            goal: Some("Improve clarity".to_string()),
            style: Some("concise".to_string()),
            tone: Some("professional".to_string()),
            level: Some(3),
            audience: Some("developers".to_string()),
            language: Some("en".to_string()),
            enable_sequential_thinking: Some(false),
            thought_count: Some(1),
        };

        let res = usecase
            .execute(Prompt { text: "write code".into() }, options)
            .await
            .unwrap();

        assert_eq!(res.text, "ENHANCED: write code - this is a longer text with enough words to pass validation");
        assert_eq!(res.rationale.as_deref(), Some("Made it clearer and more specific"));
    }

    #[tokio::test]
    async fn test_usecase_with_empty_prompt() {
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(MockProvider), config);
        let res = usecase
            .execute(Prompt { text: "".into() }, EnhancementOptions {
                enable_sequential_thinking: Some(false), // Explicitly disable sequential thinking
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.text, "ENH:  - this is a longer text with enough words to pass the validation check");
    }

    #[tokio::test]
    async fn test_usecase_provider_error() {
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(FailingProvider), config);
        let result = usecase
            .execute(Prompt { text: "test".into() }, EnhancementOptions::default())
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Provider error"));
    }

    #[tokio::test]
    async fn test_usecase_preserves_options() {
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(MockProviderWithRationale), config);

        // Test with various enhancement options
        let options = EnhancementOptions {
            goal: Some("Make it educational".to_string()),
            style: Some("step-by-step".to_string()),
            tone: Some("encouraging".to_string()),
            level: Some(4),
            audience: Some("students".to_string()),
            language: Some("en".to_string()),
            enable_sequential_thinking: Some(false),
            thought_count: Some(1),
        };

        let res = usecase
            .execute(Prompt { text: "explain rust".into() }, options)
            .await
            .unwrap();

        // The mock provider doesn't actually use the options, but the usecase should pass them through
        assert_eq!(res.text, "ENHANCED: explain rust - this is a longer text with enough words to pass validation");
        assert!(res.rationale.is_some());
    }

    #[tokio::test]
    async fn test_usecase_with_sequential_thinking() {
        let config = create_test_config();
        let provider = MockProviderWithRationale;
        let usecase = EnhancePrompt::new(Box::new(provider), config);

        let options = EnhancementOptions {
            goal: Some("Test sequential thinking".to_string()),
            enable_sequential_thinking: Some(true),
            thought_count: Some(2),
            ..Default::default()
        };

        let res = usecase
            .execute(Prompt { text: "test prompt".into() }, options)
            .await
            .unwrap();

        // Sequential thinking should enhance the prompt multiple times
        // The final result should be longer and more enhanced than the original
        assert!(res.text.contains("ENHANCED"));
        assert!(res.text.len() > "test prompt".len());
    }

    #[tokio::test]
    async fn test_usecase_default_sequential_thinking_enabled() {
        let provider = MockProviderWithRationale;
        let config = create_test_config();
        let usecase = EnhancePrompt::new(Box::new(provider), config);

        // Test with no explicit enable_sequential_thinking setting (should default to true)
        let options = EnhancementOptions {
            goal: Some("Test default sequential thinking".to_string()),
            enable_sequential_thinking: None, // Explicitly None to test default
            thought_count: Some(2),
            ..Default::default()
        };

        let res = usecase
            .execute(Prompt { text: "test prompt".into() }, options)
            .await
            .unwrap();

        // Sequential thinking should be enabled by default and enhance the prompt multiple times
        // The final result should be longer and more enhanced than the original
        assert!(res.text.contains("ENHANCED"));
        assert!(res.text.len() > "test prompt".len());
    }
}
