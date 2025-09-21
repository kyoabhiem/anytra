use crate::domain::llm::LLMProvider;
use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
use anyhow::Result;

pub struct EnhancePrompt {
    provider: Box<dyn LLMProvider + Send + Sync>,
}

impl EnhancePrompt {
    pub fn new(provider: Box<dyn LLMProvider + Send + Sync>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, prompt: Prompt, options: EnhancementOptions) -> Result<EnhancedPrompt> {
        let mut enhanced = self.provider.enhance(prompt, options).await?;
        crate::domain::validation::validate_enhanced_prompt(&enhanced)?;
        let confidence = crate::domain::validation::compute_confidence(&enhanced);
        enhanced.confidence = Some(confidence);
        Ok(enhanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::{LLMError, LLMProvider};
    use async_trait::async_trait;

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
        let usecase = EnhancePrompt::new(Box::new(MockProvider));
        let res = usecase
            .execute(Prompt { text: "hello".into() }, EnhancementOptions::default())
            .await
            .unwrap();
        assert_eq!(res.text, "ENH: hello - this is a longer text with enough words to pass the validation check");
    }

    #[tokio::test]
    async fn test_usecase_with_options() {
        let usecase = EnhancePrompt::new(Box::new(MockProviderWithRationale));
        let options = EnhancementOptions {
            goal: Some("Improve clarity".to_string()),
            style: Some("concise".to_string()),
            tone: Some("professional".to_string()),
            level: Some(3),
            audience: Some("developers".to_string()),
            language: Some("en".to_string()),
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
        let usecase = EnhancePrompt::new(Box::new(MockProvider));
        let res = usecase
            .execute(Prompt { text: "".into() }, EnhancementOptions::default())
            .await
            .unwrap();
        assert_eq!(res.text, "ENH:  - this is a longer text with enough words to pass the validation check");
    }

    #[tokio::test]
    async fn test_usecase_provider_error() {
        let usecase = EnhancePrompt::new(Box::new(FailingProvider));
        let result = usecase
            .execute(Prompt { text: "test".into() }, EnhancementOptions::default())
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Provider error"));
    }

    #[tokio::test]
    async fn test_usecase_preserves_options() {
        let usecase = EnhancePrompt::new(Box::new(MockProviderWithRationale));

        // Test with various enhancement options
        let options = EnhancementOptions {
            goal: Some("Make it educational".to_string()),
            style: Some("step-by-step".to_string()),
            tone: Some("encouraging".to_string()),
            level: Some(4),
            audience: Some("students".to_string()),
            language: Some("en".to_string()),
        };

        let res = usecase
            .execute(Prompt { text: "explain rust".into() }, options)
            .await
            .unwrap();

        // The mock provider doesn't actually use the options, but the usecase should pass them through
        assert_eq!(res.text, "ENHANCED: explain rust - this is a longer text with enough words to pass validation");
        assert!(res.rationale.is_some());
    }

    #[test]
    fn test_usecase_creation() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        // Just verify it can be created without panicking
        assert!(true); // This test mainly ensures the constructor works
    }
}
